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
use crate::db::Storage;
use rusty_leveldb::{Options, DB as LDB};

use std::path::Path;
pub struct DB(LDB);

impl DB {
    /// Create a new LevelDB.
    ///
    /// - `name`: the name of the database
    ///
    /// # Returns
    ///
    /// It returns a tuple, where the first element is the Database and the
    /// second element indicates if the database is already existed
    pub fn open(name: &'static str) -> (Self, bool) {
        let opt = Options::default();
        let existed = Path::new(name).exists();
        (DB(LDB::open(name, opt).expect("open leveldb")), existed)
    }
}

impl Storage for DB {
    fn put(&mut self, key: &[u8], value: &[u8]) -> bool {
        self.0.put(key, value).is_ok()
    }
    fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.get(key)
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {}
