#!/bin/bash

set -e

cargo test --release --package oram --lib -- tests::leveldb_read_write --exact --nocapture --ignored
cargo test --release --package oram --lib -- tests::leveldb_reopen --exact --nocapture --ignored