#!/bin/bash
sudo rm Cargo.lock
docker run --rm -it -v $PWD:/home/rust/SafeBricks ekidd/rust-musl-builder:nightly-2019-06-08-openssl11 /bin/sh -c 'sudo chown -R rust:rust ../SafeBricks && cd ../SafeBricks && ./_build_musl.sh'
sudo chown -R yangzhou:lambda-mpi-PG0 ../NetBricks-GEM5/