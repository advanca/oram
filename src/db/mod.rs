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

use crate::fmt;
use crate::vec::Vec;
use crate::Box;
use crate::HashMap;
use cfg_if::cfg_if;

#[cfg(feature = "std")]
mod leveldb;

#[cfg(feature = "sgx")]
mod sgxfs;

enum Persistence {
    #[allow(dead_code)]
    LevelDb,
    #[cfg(feature = "sgx")]
    SgxFs,
}

pub struct Options {
    /// Decides what persistence will be used.
    ///
    /// - `None`: data will be stored in memory for testing but with no persistence
    /// - `Some(LevelDb)`: Use leveldb. (Currently no available in SGX)
    /// - `Some(SgxFs)`: Use SGX protected fs. (Not available in std)
    persistence: Option<Persistence>,
}

impl Options {
    pub fn in_memory() -> Self {
        Options { persistence: None }
    }

    #[allow(dead_code)]
    pub fn leveldb() -> Self {
        Options {
            persistence: Some(Persistence::LevelDb),
        }
    }

    #[cfg(feature = "sgx")]
    pub fn sgxfs() -> Self {
        Options {
            persistence: Some(Persistence::SgxFs),
        }
    }
}

pub struct Database {
    /// The name of the `Database`. It also affects the data directory name on file system.
    name: &'static str,
    /// The backend of the `Database`.
    backend: Box<dyn Storage>,
    /// If the database exists before it's opened.
    existed: bool,
}

impl fmt::Debug for Database {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "name {{ name: {:?}, backend: <...>, existed: {} }}",
            self.name, self.existed,
        )
    }
}

impl Database {
    pub fn open(name: &'static str, opt: Options) -> Database {
        match opt.persistence {
            None => Self::new_memory(name),
            Some(Persistence::LevelDb) => Self::new_leveldb(name),
            #[cfg(feature = "sgx")]
            Some(Persistence::SgxFs) => Self::new_sgxfs(name),
        }
    }

    fn new_memory(name: &'static str) -> Self {
        Database {
            name,
            backend: Box::new(Memory::new()),
            existed: false,
        }
    }

    /// If the database exists before it's opened.
    pub fn existed(&self) -> bool {
        self.existed
    }

    cfg_if! {
        if #[cfg(feature = "sgx")] {
            fn new_leveldb(_name: &'static str) -> Self {
                //TODO: The development is ongoing, it will use https://github.com/mesalock-linux/rusty_leveldb_sgx
                unimplemented!();
            }

            fn new_sgxfs(name: &'static str) -> Self {
                let (db, existed) = sgxfs::DB::open(name);
                Database {
                    name,
                    backend: Box::new(db),
                    existed
                }
            }
        } else if #[cfg(feature = "std")] {
            fn new_leveldb(name: &'static str) -> Self {
                let (db, existed) = leveldb::DB::open(name);
                Database {
                    name,
                    backend: Box::new(db),
                    existed
                }
            }
        }
    }

    pub fn put(&mut self, key: &[u8], value: &[u8]) -> bool {
        self.backend.put(key, value)
    }

    pub fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        self.backend.get(key)
    }
}

trait Storage {
    fn put(&mut self, key: &[u8], value: &[u8]) -> bool;
    fn get(&mut self, key: &[u8]) -> Option<Vec<u8>>;
}

struct Memory {
    data: HashMap<Vec<u8>, Vec<u8>>,
}

impl Memory {
    fn new() -> Self {
        Memory {
            data: HashMap::new(),
        }
    }
}

impl Storage for Memory {
    fn put(&mut self, key: &[u8], value: &[u8]) -> bool {
        self.data.insert(key.to_vec(), value.to_vec()).is_some()
    }
    fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.get(key).map(|k| k.clone())
    }
}
