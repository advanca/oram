#!/bin/bash
set -e

cargo check --lib
cargo check --no-default-features --features=sgx --lib
