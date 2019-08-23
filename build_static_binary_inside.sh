#!/bin/bash
source ./config.sh
set -e

BASE_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd)"
BUILD_SCRIPT=$( basename "$0" )

if [[ -z ${CARGO_INCREMENTAL} ]] || [[ $CARGO_INCREMENTAL = false ]] || [[ $CARGO_INCREMENTAL = 0 ]]; then
    export CARGO_INCREMENTAL="CARGO_INCREMENTAL=0 "
fi

if [[ -z ${RUST_BACKTRACE} ]] || [[ RUST_BACKTRACE = true ]] || [[ RUST_BACKTRACE = 1 ]]; then
    export RUST_BACKTRACE="RUST_BACKTRACE=1 "
fi

echo "Current Cargo Incremental Setting: ${CARGO_INCREMENTAL}"
echo "Current Rust Backtrace Setting: ${RUST_BACKTRACE}"


# for TASK in acl-fw dpi lpm macswap maglev monitoring nat-tcp-v4 acl-fw-ipsec dpi-ipsec lpm-ipsec macswap-ipsec maglev-ipsec monitoring-ipsec nat-tcp-v4-ipsec
# do 
# 	# Build enclave APP
# 	pushd examples/$TASK
# 	if [ "$MODE" == "debug" ]; then
# 		cargo build
# 	else
# 		cargo build --release
# 	fi
# 	popd
# done

# directly building non-ipsec
for TASK in acl-fw dpi lpm macswap maglev monitoring nat-tcp-v4
do 
	# Build enclave APP
	pushd examples/$TASK
	if [ "$MODE" == "debug" ]; then
		# cargo +nightly build --target=x86_64-unknown-linux-musl
		cargo +stage1 build --target=x86_64-unknown-linux-musl
		cargo +stage2 build --target=x86_64-unknown-linux-musl
	else
	    # cargo +nightly build --target=x86_64-unknown-linux-musl --release
	    cargo +stage2 build --target=x86_64-unknown-linux-musl --release
	fi
	popd
done