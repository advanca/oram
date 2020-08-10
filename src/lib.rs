// Copyright 2020 ADVANCA PTE. LTD.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

use cfg_if::cfg_if;

#[cfg(feature = "sgx")]
#[macro_use]
extern crate sgx_tstd as std;

cfg_if! {
    if #[cfg(feature = "sgx")] {
        use sgx_tstd::{prelude::v1::*};
        use sgx_rand::{Rng, thread_rng};
        use log_sgx::{trace};
        use bincode_sgx::{serialize, deserialize};
        use serde_sgx::{Serialize, Deserialize};
        use serde_sgx::ser::{Serializer, SerializeTuple};
        use serde_sgx::de::{self as de, Deserializer, Visitor,  SeqAccess};
        use blake2_sgx::{VarBlake2b, digest::{Input, VariableOutput}};
    } else if #[cfg(feature = "std")] {
        use rand::{Rng, thread_rng};
        use log::{trace};
        use serde::{Serialize, Deserialize};
        use serde::ser::{Serializer, SerializeTuple};
        use serde::de::{self as de, Deserializer, Visitor,  SeqAccess};
        use bincode::{serialize, deserialize};
        use blake2::{VarBlake2b, digest::{Input, VariableOutput}};
        use rand_core::RngCore;
    }
}

use std::boxed::Box;
use std::collections::HashMap;
use std::convert::TryInto;
use std::default::Default;
use std::fmt;
use std::ops::Range;
use std::str;
use std::vec;

mod db;
pub mod sort;

mod data;
pub use data::Data;
use data::DataWrapper;
use db::Database;
type Salt = [u8; 32];
pub struct SqrtOram {
    /// Number of real blocks
    n: usize,
    /// Number of blocks in shelter
    shelter_size: usize,
    /// Total number of blocks in storage
    capacity: usize,
    /// Salt for PRF
    salt: Salt,
    /// Database
    db: Database,
    /// Number of read/write operations executed,
    count: usize,
    /// Cache of stored blocks
    cache: Vec<BlockCache>,
    /// Length of data stored in each block
    block_size: usize,
}

#[cfg_attr(feature = "sgx", serde(crate = "serde_sgx"))]
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
struct BlockCache {
    tag: u32,
    index: u32,
}

const DUMMY_INDEX: u32 = u32::MAX;

#[cfg_attr(feature = "sgx", serde(crate = "serde_sgx"))]
#[derive(Serialize, Deserialize, Clone)]
/// Block content
struct Block {
    header: BlockCache,
    /// The stored data of the block
    data: DataWrapper,
}

impl Block {
    /// Create a block with valid index (>0) or create dummy block (index = -1)
    ///
    /// other parameters:
    /// - `size`: length of data stored
    /// - `salt`: salt used to compute tag
    fn new(index: u32, size: usize, salt: Salt) -> Self {
        let mut buf = vec![0; size];
        thread_rng().fill_bytes(&mut buf);
        let data = DataWrapper { buf, max_len: size };
        let tag = Self::derive_tag(index, salt);
        Block {
            header: BlockCache { tag, index },
            data,
        }
    }

    /// Make a dummy clone with only tag unchanged
    ///
    /// In the clone, `index` is set to DUMMY_INDEX and `data` is randomized.
    fn dummy_clone(&self) -> Self {
        let mut buf = vec![0; self.data.max_len];
        thread_rng().fill_bytes(&mut buf);
        let data = DataWrapper {
            buf,
            max_len: self.data.max_len,
        };
        Block {
            header: BlockCache {
                tag: self.header.tag,
                index: DUMMY_INDEX,
            },
            data,
        }
    }

    fn derive_tag(index: u32, salt: Salt) -> u32 {
        let mut hasher = VarBlake2b::new_keyed(&salt, 4);
        hasher.input(index.to_be_bytes());
        let hash = hasher.vec_result();
        u32::from_be_bytes(hash[0..4].try_into().expect("slice to array"))
    }
}

impl SqrtOram {
    /// Create a new SqrtOram in memory. If persistence is needed, see `open`.
    ///
    /// - `n`: number of real blocks
    /// - `block_size`: size of each blocks in bytes
    pub fn new(n: usize, block_size: usize) -> Self {
        Self::create(n, block_size, None)
    }

    /// Open an existing or create a new SqrtORAM on disk.
    ///
    /// - `name`: name of the storage; name of the data directory on file system
    /// - `n`: number of real blocks
    /// - `block_size`: size of each blocks in bytes
    pub fn open(name: &'static str, n: usize, block_size: usize) -> Self {
        Self::create(n, block_size, Some(name))
    }

    /// An internal method for creating SqrtOram
    fn create(n: usize, block_size: usize, name: Option<&'static str>) -> Self {
        let shelter_size = (n as f64).sqrt() as usize;
        let capacity = n + 2 * shelter_size;
        let salt = Self::generate_salt();
        let db = Self::create_db(name);
        let cache = vec![Default::default(); capacity];
        let existed = db.existed();

        let mut oram = SqrtOram {
            n,
            shelter_size,
            capacity,
            salt,
            db,
            count: 0,
            cache,
            block_size,
        };

        if existed {
            // If this is a re-open, recalculate the hash
            oram.warm_up_cache();
            oram.rehash();
        } else {
            // If DB is opened for the first time, initialize the blocks
            oram.init_blocks();
        }

        oram.shuffle();
        oram
    }

    fn warm_up_cache(&mut self) {
        for i in 0..self.capacity {
            let data = self.db.get(&(i as u32).to_be_bytes()).expect("get block");
            let block: Block = deserialize(&data[..]).expect("deserialize block");
            self.cache[i] = block.header;
        }
    }

    fn init_blocks(&mut self) {
        for i in 0..self.capacity {
            let mut block_index = 0 as u32;
            if self.real_range().contains(&i) || self.dummy_range().contains(&i) {
                block_index = i as u32;
            } else if self.shelter_range().contains(&i) {
                block_index = DUMMY_INDEX;
            }
            let block = Block::new(block_index, self.block_size, self.salt);
            self.write_block(i as u32, &block);
        }
    }

    fn read_block(&mut self, k: u32) -> Block {
        trace!("read_block(key={})", k);
        let data = self.db.get(&k.to_be_bytes()).expect("get block");
        let block: Block = deserialize(&data[..]).expect("deserialize block");
        assert_eq!(
            block.data.max_len, self.block_size,
            "a corrupt block as `max_len` is incorrect"
        );
        assert_eq!(self.cache[k as usize], block.header);
        block
    }

    fn write_block(&mut self, k: u32, v: &Block) {
        trace!("write_block(key={})", k);
        self.cache[k as usize] = v.header.clone();
        self.db
            .put(&k.to_be_bytes(), &serialize(v).expect("serialize block"));
    }

    fn real_range(&self) -> Range<usize> {
        0..self.n
    }

    fn dummy_range(&self) -> Range<usize> {
        self.n..self.n + self.shelter_size
    }

    fn shelter_range(&self) -> Range<usize> {
        self.n + self.shelter_size..self.capacity
    }

    #[cfg(feature = "std")]
    fn generate_salt() -> Salt {
        rand::thread_rng().gen::<Salt>()
    }

    #[cfg(feature = "sgx")]
    fn generate_salt() -> Salt {
        sgx_rand::thread_rng().gen::<Salt>()
    }

    #[cfg(feature = "std")]
    fn create_db(name: Option<&'static str>) -> Database {
        match name {
            Some(db_name) => Database::open(db_name, db::Options::leveldb()),
            None => Database::open("in-memory", db::Options::in_memory()),
        }
    }

    #[cfg(feature = "sgx")]
    fn create_db(name: Option<&'static str>) -> Database {
        match name {
            Some(db_name) => Database::open(db_name, db::Options::sgxfs()),
            None => Database::open("in-memory", db::Options::in_memory()),
        }
    }

    /// Store data `v` at key `k`
    ///
    /// `v` has a capacity limit up to `self.block_size`.
    ///
    /// # Panic
    ///
    /// panic when `v.len()` is greater than self.block_size
    pub fn put(&mut self, k: u32, v: Data) {
        assert!(
            v.len() <= self.block_size,
            "`v.len()` should be less than block_size"
        );
        self.access(
            k,
            Some(DataWrapper {
                buf: v,
                max_len: self.block_size,
            }),
        );
    }

    /// Similar to HashMap::get()
    pub fn get(&mut self, k: u32) -> Option<Data> {
        self.access(k, None).map(|d| d.buf)
    }

    /// If write is None, access() will run read operation, otherwise write.
    fn access(&mut self, k: u32, write: Option<DataWrapper>) -> Option<DataWrapper> {
        let mut found_in_shelter = false;
        let mut found_block = Block::new(DUMMY_INDEX, self.block_size, self.salt);

        for i in self.shelter_range() {
            trace!("accessing block {} in shelter", i);
            let block = self.read_block(i as u32);
            if !found_in_shelter && self.cache[i].index == k {
                found_in_shelter = true;
                found_block = block.clone();
            }
            self.write_block(i as u32, &block);
        }

        if found_in_shelter {
            let seek = Block::derive_tag((self.n + self.count) as u32, self.salt);
            let location = match self.cache[0..self.capacity - self.shelter_size]
                .binary_search_by(|c| c.tag.cmp(&seek))
            {
                Ok(i) => i as u32,
                //TODO: handle the error
                Err(_) => panic!("Binary search should be successful"),
            };
            let block = self.read_block(location);
            self.write_block(location, &block);
        } else {
            let seek = Block::derive_tag(k as u32, self.salt);
            let location = match self.cache[0..self.capacity - self.shelter_size]
                .binary_search_by(|c| c.tag.cmp(&seek))
            {
                Ok(i) => i as u32,
                //TODO: handle the error
                Err(_) => panic!("Binary search should be successful"),
            };
            found_block = self.read_block(location);
            self.write_block(location, &found_block.dummy_clone());
        }

        let shelter_write_index = (self.n + self.shelter_size + self.count) as u32;
        let mut is_write = false;
        if found_in_shelter {
            self.write_block(
                shelter_write_index,
                &Block::new(DUMMY_INDEX, self.block_size, self.salt),
            )
        } else if let Some(data) = write {
            is_write = true;
            found_block.data = data;
            self.write_block(shelter_write_index, &found_block);
        } else {
            self.write_block(shelter_write_index, &found_block);
        }
        self.count += 1;
        if self.count == self.shelter_size {
            self.rearrange();
            self.rehash();
            self.shuffle();
            self.count = 0;
        }

        if is_write {
            None
        } else {
            Some(found_block.data)
        }
    }

    /// Rotate the salt value and re-derive the tag value for each block
    ///
    /// TODO: find a better name or move the code
    fn rehash(&mut self) {
        self.salt = Self::generate_salt();
        for i in 0..self.dummy_range().end {
            let mut block = self.read_block(i as u32);
            block.header.tag = Block::derive_tag(i as u32, self.salt);
            self.write_block(i as u32, &block);
        }
    }

    /// Shuffle real and dummy blocks
    ///
    /// Internally it sorts real and dummy blocks accroding to their tag.
    fn shuffle(&mut self) {
        sort::odd_even_mergesort(
            0..self.dummy_range().end,
            |x: &Block, y: &Block| x.header.tag < y.header.tag,
            |i, w| match w {
                Some(x) => {
                    self.write_block(i as u32, x);
                    None
                }
                None => Some(self.read_block(i as u32)),
            },
        )
    }

    /// Rearrange the blocks so that real blocks are sorted into `Self::real_range()`.
    ///
    /// Internally it sorts all blocks accroding to the original index. Real blocks
    /// will have valid index while dummy and shelter blocks have DUMMY_INDEX.
    fn rearrange(&mut self) {
        sort::odd_even_mergesort(
            0..self.capacity,
            |x: &Block, y: &Block| x.header.index < y.header.index,
            |i, w| match w {
                Some(x) => {
                    self.write_block(i as u32, x);
                    None
                }
                None => Some(self.read_block(i as u32)),
            },
        )
    }
}

impl Drop for SqrtOram {
    fn drop(&mut self) {
        self.rearrange();
    }
}

// do all the unitest with std libraries
#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use env_logger::Env;
    use hex;
    use log::{debug, info};
    use std::fmt;
    use std::fs;

    const TEST_BLOCK_SIZE: usize = 32;

    impl fmt::Debug for Block {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(
                formatter,
                "Block {{ header: {:?}, data: {}... }}",
                self.header,
                hex::encode(self.data.buf[0..16].to_vec())
            )
        }
    }

    fn init_logger() {
        let _ = env_logger::from_env(Env::default().default_filter_or("debug"))
            .is_test(true)
            .try_init();
    }

    #[test]
    fn initialization() {
        init_logger();

        let n = 16 as usize;
        let mut oram = SqrtOram::new(n, TEST_BLOCK_SIZE);
        oram.init_blocks();

        for i in 0..oram.capacity {
            let block = oram.read_block(i as u32);
            info!("Block {} = {:?}", i, &block);
            if oram.real_range().contains(&i) || oram.dummy_range().contains(&i) {
                assert_eq!(block.header.index, i as u32);
            } else if oram.shelter_range().contains(&i) {
                assert_eq!(block.header.index, DUMMY_INDEX);
            } else {
                panic!(
                    "index {} should be in either real_range(), dummy_range() or shelter_range()",
                    i
                );
            }
        }
    }

    fn dump_blocks(oram: &mut SqrtOram) {
        for i in 0..oram.capacity {
            debug!("{:?}", oram.read_block(i as u32));
        }
    }

    #[test]
    fn single_write_and_read() {
        init_logger();
        let mut oram = SqrtOram::new(16, TEST_BLOCK_SIZE);

        dump_blocks(&mut oram);
        let key = 15 as u32;
        let mut data = [0u8; TEST_BLOCK_SIZE];
        data[0..4].copy_from_slice(&key.to_le_bytes());
        oram.put(key, data.to_vec());
        dump_blocks(&mut oram);
        let read_data = oram.get(key).unwrap();
        assert_eq!(read_data[..], data[..]);
    }

    #[test]
    fn basic_test() {
        init_logger();

        let n = 8 as usize;
        let mut oram = SqrtOram::new(n, TEST_BLOCK_SIZE);

        for i in 0..n {
            let mut data = [0u8; TEST_BLOCK_SIZE];
            data[0..8].copy_from_slice(&i.to_le_bytes());
            assert_eq!(oram.count, i % oram.shelter_size);
            oram.put(i as u32, data.to_vec());
        }

        for i in 0..n {
            let mut data = [0u8; TEST_BLOCK_SIZE];
            data[0..8].copy_from_slice(&i.to_le_bytes());
            assert_eq!(oram.count % oram.shelter_size, (n + i) % oram.shelter_size);
            assert_eq!(&data[..], &oram.get(i as u32).unwrap()[..]);
        }
    }

    #[test]
    #[should_panic]
    fn put_lengthy_data() {
        init_logger();

        let n = 8 as usize;
        let mut oram = SqrtOram::new(n, TEST_BLOCK_SIZE);

        oram.put(0 as u32, vec![0; TEST_BLOCK_SIZE + 1]);
    }

    #[test]
    fn put_right_sized_data() {
        init_logger();

        let n = 8 as usize;
        let mut oram = SqrtOram::new(n, TEST_BLOCK_SIZE);

        oram.put(0 as u32, vec![0; TEST_BLOCK_SIZE]);
        oram.put(0 as u32, vec![0; TEST_BLOCK_SIZE - 1]);
    }

    #[test]
    #[ignore]
    // Ignore this test as it takes long time to complete.
    //
    // To execute this test quickly, run with 'release' profile:
    // ```
    // cargo test --release --package oram --lib -- tests::medium_size_test --exact --nocapture --ignored
    // ``
    fn medium_size_test() {
        init_logger();
        info!("logger initialized");

        let n = 2048 as usize;
        let mut oram = SqrtOram::new(n, TEST_BLOCK_SIZE);
        info!("oram initialized");

        for i in 0..n {
            let mut data = [0u8; TEST_BLOCK_SIZE];
            data[0..8].copy_from_slice(&i.to_le_bytes());
            assert_eq!(oram.count, i % oram.shelter_size);
            oram.put(i as u32, data.to_vec());
        }
        info!("oram put done");

        for i in 0..n {
            let mut data = [0u8; TEST_BLOCK_SIZE];
            data[0..8].copy_from_slice(&i.to_le_bytes());
            assert_eq!(oram.count % oram.shelter_size, (n + i) % oram.shelter_size);
            assert_eq!(&data[..], &oram.get(i as u32).unwrap()[..]);
        }
        info!("oram get done");
    }

    #[test]
    #[ignore]
    // Ignore this test as it takes long time to complete.
    //
    // To execute this test quickly, run with 'release' profile:
    // ```
    // cargo test --release --package oram --lib -- tests::access_same_location --exact --nocapture --ignored
    // ``
    fn access_same_location() {
        init_logger();
        info!("logger initialized");

        let n = 8192 as usize;
        let mut oram = SqrtOram::new(n, TEST_BLOCK_SIZE);
        info!("oram initialized");

        for i in 0..n {
            info!("putting for {} time", i);
            let k = 0 as usize;
            let mut data = [0u8; TEST_BLOCK_SIZE];
            data[0..8].copy_from_slice(&k.to_le_bytes());
            oram.put(k as u32, vec![0; TEST_BLOCK_SIZE]);
        }
        info!("oram put done");
    }

    fn remove_db_folder(path: &str) {
        use std::io::ErrorKind;
        use std::io::Result;

        fs::remove_dir_all(path)
            .or_else(|e| -> Result<()> {
                if e.kind() == ErrorKind::NotFound {
                    return Ok(());
                } else {
                    Err(e)
                }
            })
            .expect("remove existing db directory");
    }

    #[test]
    #[ignore]
    // Ignore this test as it takes long time to complete.
    // Also, there's a strange stack overflow bug when running this with 'debug' profile
    //
    // To execute this test quickly, run with 'release' profile:
    // ```
    // cargo test --release --package oram --lib -- tests::leveldb_read_write --exact --nocapture --ignored
    // ``
    fn leveldb_read_write() {
        init_logger();

        let db_name = "db";

        remove_db_folder(db_name);

        let n = 512 as usize;
        let mut oram = SqrtOram::open(db_name, n, TEST_BLOCK_SIZE);

        for i in 0..n {
            assert_eq!(oram.count, i % oram.shelter_size);
            oram.put(i as u32, i.to_be_bytes().to_vec());
        }

        for i in 0..n {
            assert_eq!(oram.count % oram.shelter_size, (n + i) % oram.shelter_size);
            assert_eq!(i.to_be_bytes().to_vec(), oram.get(i as u32).unwrap());
        }
    }

    #[test]
    #[ignore]
    // Ignore this test as it has to be run after `leveldb_read_write`
    //
    // To execute this test quickly, run with 'release' profile:
    // ```
    // cargo test --release --package oram --lib -- tests::leveldb_reopen --exact --nocapture --ignored
    // ``
    fn leveldb_reopen() {
        init_logger();

        let db_name = "db";

        let n = 512 as usize;
        let mut oram = SqrtOram::open(db_name, n, TEST_BLOCK_SIZE);

        for i in 0..n {
            assert_eq!(i.to_be_bytes().to_vec(), oram.get(i as u32).unwrap());
        }

        // clean up
        remove_db_folder(db_name);
    }
}
