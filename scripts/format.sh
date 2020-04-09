#!/bin/bash
cargo fmt
cargo fmt --manifest-path examples/sgx/app/Cargo.toml
cargo fmt --manifest-path examples/sgx/enclave/Cargo.toml