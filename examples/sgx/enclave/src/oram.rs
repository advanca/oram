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

use oram::SqrtOram;

pub fn example_in_memory() {
    let n = 16 as usize;
    let block_size = 16 as usize;
    let mut oram = SqrtOram::new(n, block_size);

    for i in 0..n {
        let mut data = vec![0u8; block_size];
        data[0..8].copy_from_slice(&i.to_le_bytes());
        oram.put(i as u32, data);
    }

    for i in 0..n {
        let mut data = vec![0u8; block_size];
        data[0..8].copy_from_slice(&i.to_le_bytes());
        assert_eq!(&data[..], &oram.get(i as u32).unwrap()[..]);
    }
}

pub fn example_on_disk(get_only: bool) {
    let n = 64 as usize;
    let block_size = 512 as usize;
    let mut oram = SqrtOram::open("db", n, block_size);

    if !get_only {
        for i in 0..n {
            let mut data = vec![0u8; block_size];
            data[0..8].copy_from_slice(&i.to_le_bytes());
            oram.put(i as u32, data);
            println!("ENCLAVE put data {}", i);
        }
    }

    for i in 0..n {
        let mut data = vec![0u8; block_size];
        data[0..8].copy_from_slice(&i.to_le_bytes());
        assert_eq!(&data[..], &oram.get(i as u32).unwrap()[..]);
        println!("ENCLAVE get data {}", i);
    }
}
