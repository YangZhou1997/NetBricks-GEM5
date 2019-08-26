#!/bin/bash
source ./config.sh
set -e

TASK=macswap

BASE_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd)"

if [[ -z ${CARGO_INCREMENTAL} ]] || [[ $CARGO_INCREMENTAL = false ]] || [[ $CARGO_INCREMENTAL = 0 ]]; then
    export CARGO_INCREMENTAL="CARGO_INCREMENTAL=0 "
fi

if [[ -z ${RUST_BACKTRACE} ]] || [[ RUST_BACKTRACE = true ]] || [[ RUST_BACKTRACE = 1 ]]; then
    export RUST_BACKTRACE="RUST_BACKTRACE=1 "
fi

echo "Current Cargo Incremental Setting: ${CARGO_INCREMENTAL}"
echo "Current Rust Backtrace Setting: ${RUST_BACKTRACE}"

if [ $# -ge 1 ]; then
    TASK=$1
fi
echo $TASK

cd target/$MODE/
if [ $# -eq 2 ]; then
    RUST_BACKTRACE=1 ./$TASK $2
else
    RUST_BACKTRACE=1 ./$TASK
fi
cd -

