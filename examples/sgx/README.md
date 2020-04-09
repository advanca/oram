# SGX

This is an example of how to use `oram` in SGX environment.

## Build

Make sure you have SGX environment configured.

```shell
make
```

## Run

```shell
cd bin
./app
```

## FAQ

1. How to resolve runtime error `fatal runtime error: memory allocation failed`?

    Check if you set enough heap memory in `Enclave.config.xml` (See `HeapMaxSize` field)

## TODO

- [ ] support Xargo
- [ ] delete db folder after completion
- [ ] auto-pull submodule
- [ ] clean up