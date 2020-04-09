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

//! A naive implementaion of database using SGX protected filesystem
use blake2_sgx::{digest::Digest, Blake2b};
use hex_sgx::encode as hex_encode;
use protected_fs;
use sgx_tstd::fs;
use sgx_tstd::io::Read;
use sgx_tstd::io::Write;
use sgx_tstd::path::PathBuf;
use sgx_tstd::prelude::v1::*;
use sgx_tstd::vec;

use crate::db::Storage;
use bincode_sgx::{deserialize, serialize};
use serde_derive_sgx::{Deserialize, Serialize};

#[derive(Debug)]
pub struct DB {
    name: &'static str,
    path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "sgx", serde(crate = "serde_sgx"))]
struct DataFile {
    key: Vec<u8>,
    value: Vec<u8>,
}

impl DB {
    /// Open database directory
    ///
    /// # Return
    ///
    /// This function returns a tuple, where
    /// - `0`: The DB
    /// - `1`: if the database (directory) exists
    pub fn open(name: &'static str) -> (Self, bool) {
        let path = PathBuf::from(name);
        let existed = path.exists();

        if !existed {
            fs::create_dir_all(path.as_path()).expect("create directory");
        }

        let db = DB { name, path };

        (db, existed)
    }
}

fn data_filename(key: &[u8]) -> String {
    let mut hasher = Blake2b::new();
    hasher.input(key);
    let hash = hasher.result();
    hex_encode(&hash)
}

impl Storage for DB {
    fn put(&mut self, key: &[u8], value: &[u8]) -> bool {
        // open file based on key name
        let mut p = self.path.clone();
        p.push(data_filename(key));

        let mut f = protected_fs::OpenOptions::default()
            .write(true)
            .append(false)
            .open(p.as_path())
            .expect("open file to write");
        let data_file = DataFile {
            key: key.to_vec(),
            value: value.to_vec(),
        };
        f.write(&serialize(&data_file).expect("serialize DataFile"))
            .is_ok()
    }

    fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        // open file based on key name
        let mut p = self.path.clone();
        p.push(data_filename(key));

        let mut f = protected_fs::OpenOptions::default()
            .read(true)
            .open(p.as_path())
            .expect("open file to read");

        let mut buf = vec![];

        if f.read_to_end(&mut buf).is_err() {
            //TODO: better error handling
            return None;
        }

        let data_file: DataFile = deserialize(&buf).expect("deserialize DataFile");
        Some(data_file.value)
    }
}
