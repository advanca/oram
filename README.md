# oram

![](https://github.com/advanca/oram/workflows/Rust/badge.svg)

A rust implementation of ORAM storage in Intel SGX.

Currently available algorithms:

- [x] Square-Root ORAM

Currently available storage backends:

|     backend     | std support :one:  |    sgx support     |    persistence     |
|:---------------:|:------------------:|:------------------:|:------------------:|
| in-memory :two: | :white_check_mark: | :white_check_mark: |                    |
|     LevelDB     | :white_check_mark: |   :construction:   | :white_check_mark: |
|      SgxFS      |                    | :white_check_mark: | :white_check_mark: |

Note:

- :one: The std support is mostly for development and testing
- :two: The in-memory backend has no persistence and should only be used for testing

## Development

This crate can be built with `std` for development purpose.

```shell
# quick check in default std environment
cargo check
# unit test
cargo test
```

To make sure the crate also works in SGX.

```shell
cargo check --no-default-features --features=sgx
```

A all-in-one scripts for compile check.

```shell
./scripts/check.sh
```

## Benchmarking

Install gnuplot.

```shell
sudo apt install gnuplot -y
```

Run the benchmarks

```shell
cargo bench
```

Open the file `target/criterion/report/index.html` in your browser and see the report.

## Use in SGX

To use the crate in SGX environment, see the example project at [examples/sgx](examples/sgx/README.md).

## License

[Apache 2.0](./LICENSE)
