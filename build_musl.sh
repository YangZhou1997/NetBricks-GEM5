#!/bin/bash
docker run --rm -it -v /users/yangzhou/SafeBricks:/home/rust/SafeBricks ekidd/rust-musl-builder:nightly-2019-04-25-openssl11 /bin/sh -c 'sudo chown -R rust:rust ../SafeBricks && cd ../SafeBricks && ./build_static_binary_inside.sh'
sudo chown -R yangzhou:lambda-mpi-PG0 ../SafeBricks/