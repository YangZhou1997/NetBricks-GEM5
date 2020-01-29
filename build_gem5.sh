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
echo $TARGET

# export CARGO_TARGET_ARM_UNKNOWN_LINUX_MUSLEABI_LINKER=arm-linux-gnueabi-gcc
# export CC_arm_unknown_linux_musleabi=arm-linux-gnueabi-gcc

# cargo build --target arm-unknown-linux-musleabi --release

if [ "$MODE" == "debug" ]; then
	# cargo +stage1 build
	# cargo build --target=$TARGET
	cross build --target=$TARGET
else
	# cargo +stage1 build --release
	# cargo build --target=$TARGET --release
	cross build --target=$TARGET --release
fi

# directly building all NFs using customized rustc without stack overflow check. 
# for TASK in acl-fw dpi lpm macswap maglev monitoring nat-tcp-v4 acl-fw-ipsec dpi-ipsec lpm-ipsec macswap-ipsec maglev-ipsec monitoring-ipsec nat-tcp-v4-ipsec dumptrace
# for TASK in macswap dumptrace dpi-master spmc dpi
# for TASK in dpi macswap spmc
# for TASK in macswap dpi spmc
# do 
# 	# Build enclave APP
# 	pushd examples/$TASK
# 	if [ "$MODE" == "debug" ]; then
# 		# cargo +stage1 build
# 		# cargo build --target=$TARGET
# 		cross build --target=$TARGET
# 	else
# 		# cargo +stage1 build --release
# 		# cargo build --target=$TARGET --release
# 		cross build --target=$TARGET --release
# 	fi
# 	popd
# done
