[![Build Status](https://travis-ci.org/williamofockham/NetBricks.svg?branch=master)](https://travis-ci.org/williamofockham/NetBricks)

[NetBricks](http://netbricks.io/) is a Rust based framework for NFV development. Please refer to the
[paper](https://people.eecs.berkeley.edu/~apanda/assets/papers/osdi16.pdf) for information
about the architecture and design. Currently NetBricks requires a relatively modern Linux version.

## How to remove stack overflow check in Rust runtime
Directly build in user account (not root)

```shell
curl https://sh.rustup.rs -sSf | sh
source $HOME/.cargo/env
rustup install nightly
rustup default nightly
rustup install nightly-2019-05-22
rustup override set nightly-2019-05-22
source $HOME/.cargo/env

# basic setup for rustc src
git clone git@github.com:YangZhou1997/rust.git
cd rust
git submodule update --init

# seems you do not need these four steps
# checkout all rep in src/tools/ to the old version.
# add “cargo-features = ["default-run"]” to the topo of miri cargo.toml
# cargo install cargo-vendor
# cargo vendor

# install musl: 
git clone git@github.com:ifduyue/musl.git
./configure && sudo make install

# add this to config.toml (has been added):
# [target.x86_64-unknown-linux-musl]
# musl-root = "/usr/local/musl"

# build rust x86_64-unknown-linux-musl toolchain 
# comment compiler_builtins_lib in src/libstd/lib.rs
./x.py build --target x86_64-unknown-linux-musl
rustup toolchain link stage1 build/x86_64-unknown-linux-gnu/stage1
rustup toolchain link stage2 build/x86_64-unknown-linux-gnu/stage2

# build helloworld using stage1
cargo +stage1 build --target x86_64-unknown-linux-musl
```

## How to build Netbricks for GEM5 simulation
GEM5 does not support sigaltstack() (131) and sys_getrandom() (318). 
Thus, 
* We remove the stack overflow check in Rust runtime by modifying its rustc compiler, and get toolchain `stage1`. 
* We also remove all rand related crates, and implement our own random number generator using hash with seed following srand() and rand() in C.

1. Make sure you are using nightly-2019-05-22: 
    ```shell
    rustup install nightly-2019-05-22
    rustup override set nightly-2019-05-22
    ```

    Export necessary env (do not need to be root): 
    ```shell
    source $HOME/.cargo/env
    cd NetBricks-GEM5
    ```

2. In the `NetBricks-GEM5` folder, you can change configuration in `config.sh`.
    * `MODE` decides whether you will run NF in debug mode or release mode

3. Build NetBricks (release or debug determined by `config.sh`):
    ```shell
    # release might require 10mins for the first time. 
    ./build_gem5.sh
    ```
    
4. Run the NF instances; currently, we support 7*2 NFs:
    ```shell
    ./run_gem5.sh acl-fw 1048576
    # dpi lpm macswap maglev monitoring nat-tcp-v4 acl-fw-ipsec dpi-ipsec lpm-ipsec macswap-ipsec maglev-ipsec monitoring-ipsec nat-tcp-v4-ipsec
    ```
    You can specify the number of packet using commandline as shown above. 

    * **Note1:** Please use 2097152 (2M) packets for monitoring, and 1048576 (1M) packets for the others. 
    * **Note2:** NFs with IPSec might be too slow to run in GEM5; currently, we just test the NFs without IPSec. 
    <!-- Due to some unknown reason, GEM5-NFs will have "memory double free error", if we specify the number of packets in command line.  
    Instead, you can change the packet number (const PKT_NUM) in `$HOME/NetBricks-GEM5/framework/src/scheduler/mod.rs` for NFs without IPSec and `$HOME/NetBricks-GEM5/framework-ipsec/src/scheduler/mod.rs` for NFs with IPSec, and then **rebuild**. 
    Note that this error still appears in rustc 1.38.0-nightly, not only in our hacked toolchain (1.39.0-dev) (`rustc +stage1 -vV`) -->

5. Kill the running NF instances: 
    Wait for the specified number of packet getting processed, or ctrl + c
    
### Note
1. Binary files are located in `$HOME/NetBricks-GEM5/target/release` or `$HOME/NetBricks-GEM5/target/debug`. 
2. Since we are generating packets (both header and payload) with hash, the packet processing speed is pretty slow. 
    For example, 
    ```Shell
    time ./run_gem5.sh acl-fw 2097152
    real    4m28.498s
    user    3m17.712s
    sys     1m10.796s
    ```

## How to build completely static NetBricks binaries with no external dependencies
We are using musl-gcc and musl-libc to build completely static rust binaries. 
We use containerized musl and openssl environments provided by [rust-musl-builder](https://github.com/emk/rust-musl-builder). 

0. Install Docker: 
    ```
    sudo apt install docker.io
    sudo usermod -a -G docker $USER
    # completely log out of your account and log back in
    ```

1. Make sure you are using nightly-2019-05-22: 
    ```shell
    rustup install nightly-2019-05-22
    rustup override set nightly-2019-05-22
    ```

    Export necessary env (do not need to be root): 
    ```shell
    source $HOME/.cargo/env
    cd NetBricks-GEM5
    ```

2. In the `NetBricks-GEM5` folder, you can change configuration in `config.sh`.
    *. `MODE` decides whether you will run NF in debug mode or release mode

3. Build NetBricks (release or debug determined by `config.sh`):
    ```shell
    # release might require 10mins for the first time. 
    ./build_musl.sh
    ```
    Verifying the binaries do not have external dependencies by `readelf --dyn-syms lpm`

4. Run the NF instances; currently, we support 7*2 NFs:
    ```shell
    ./run_musl.sh acl-fw 2097152
    # dpi lpm macswap maglev monitoring nat-tcp-v4 acl-fw-ipsec dpi-ipsec lpm-ipsec macswap-ipsec maglev-ipsec monitoring-ipsec nat-tcp-v4-ipsec
    ```
    You can specify the number of packet using commandline as shown above. 
    
5. Kill the running NF instances: 
    Wait for the specified number of packet getting processed, or ctrl + c
    

### Note
1. Binary files are located in `$HOME/NetBricks-GEM5/target/x86_64-unknown-linux-musl/release` or `$HOME/NetBricks-GEM5/target/x86_64-unknown-linux-musl/debug`. 
1. If you encountered error like `invalid serialized PackageId for key package.dependencies`, you just need to remove the Cargo.lock


## Setting up your local Ubuntu-16.04 environment

1. Clone our [utils](//github.com/YangZhou1997/utils) and [moonGen](//github.com/YangZhou1997/MoonGen)
   repositories into the same parent directory.
   ```shell
   host$ for repo in utils moonGen NetBricks; do \
           git clone --recurse-submodules git@github.com:YangZhou1997/${repo}.git; \
         done
   ```

2. Update and install packages. Any kernel should be generally okay -- we have tested on 4.4.0-131-generic, 4.4.0-142-generic, 4.4.0-145-generic, and 4.15.0-15-generic. 
     
    ```shell
    host$ sudo bash ./utils/vm-setup.sh
    ```
    <!-- ```shell
    host$ sudo bash ../utils/vm-kernel-upgrade.sh #require rebooting
    host$ sudo shutdown -r now
    host$ sudo bash ../utils/vm-setup.sh
    ``` -->

3. After step 2, you machine meets the basic requirements of running NetBricks. Now you need to build and bind DPDK using [setupDpdk.sh](./setupDpdk.sh). 
    ```shell
    host$ mkdir $HOME/trash
    host$ mkdir $HOME/tools
    host$ ./setupDpdk.sh
    ```
    
    **Note**: you need to change the dpdk device number in the last line of setupDpdk.sh.

4. Run the `sandbox` container from NetBricks/:
   ```shell
   host$ make -f docker.mk run
   ```

5. After step 4, you'll be in the container and then can compile and test NetBricks via
   ```shell
   docker$ cd netbricks
   docker$ make build
   ...
   docker$ make test
   ...
   ```

   **Note**: you need to change the dpdk device number in the first line of [Makefile](./Makefile).

## Developing in local Ubuntu-16.04 environment

1. Make sure that you have gone though step 1-3 of last section successfully. Current version of NetBricks will read some DPDK lib from /opt/dpdk/build/ during runtime, you need to copy include/ and lib/ directory from $RTE_SDK/build to /opt/dpdk/build/. Note that soft links need to be considered carefully. We provide [setupDpdkCopy.sh](./setupDpdkCopy.sh) for that (actually, `setupDpdkCopy.sh` has been executed in `setupDpdk.sh`): 
    ```shell
    host$ ./setupDpdkCopy.sh
    ```

2. As far as I know, NetBricks assumes you are root when running it. So you need to switch to root now. 
    ```shell
    host$ sudo su
    root$ ./setupBuild.sh 
    ```
    
    [setupBuild.sh](./setupBuild.sh) will install the rust nightly, clang, and etc for running NetBricks. 

    This NetBricks codebase works on rust nightly-2019-05-22. You can override the rust version in current directory to nightly-2019-05-22 by:
    ```shell
    rustup install nightly-2019-05-22
    rustup override set nightly-2019-05-22
    ```

3. After step 2, you need to set ```RTE_SDK``` to the dpdk directory, and load cargo environment. Then you'll be able to compile and test NetBricks:
   ```shell
    root$ export RTE_SDK=$HOME/tools/dpdk-stable-17.08.1 # for instance.
    root$ source $HOME/.cargo/env
    root$ make build
    ...
    root$ make test
    ...
   ```

    We also provide some commands that might be helpful when dealing with DPDK hugepages in [setupHuge.sh](./setupHuge.sh).
    
    **Note**: when you switch between local deployment and container deployment, you need to ```sudo make clean``` to rebuild the dependencies in native/ (especially .make.dep).  

    **Note**: if you find numerous error printed during `make build`, it is caused by the bindgen (generating rust binding for dpdk); you can solve it by deleting `~/tools/dpdk-stable-17.08.1` and run `./setupDpdk.sh`. The specific reason is that you might download my hacked version of dpdk, which will fail the bindgen binding. 


## Enabling SGX if `Software Controlled` set

Clone linux-sgx and build in your home directory:
```shell
git clone git@github.com:intel/linux-sgx.git
sudo apt-get -y install build-essential ocaml automake autoconf libtool wget python libssl-dev
sudo apt-get -y install libssl-dev libcurl4-openssl-dev protobuf-compiler libprotobuf-dev debhelper cmake
cd linux-sgx
./download_prebuilt.sh
make -j16
```

Enable SGX in your machine which set **Software Controlled**: 
```shell
gcc enable_sgx.cpp -o enable_sgx -L/home/yangz/linux-sgx/sdk/libcapable/linux -lsgx_capable -I/home/yangz/linux-sgx/common/inc/
sudo LD_LIBRARY_PATH=/home/yangz/linux-sgx/sdk/libcapable/linux ./enable_sgx
```

From https://github.com/intel/linux-sgx/issues/354: 
is_sgx_capable has to come back a 1 to be able to be enabled.
If so, then status should come back a 1 also, which means "SGX_DISABLED_REBOOT_REQUIRED". Once you reboot, you should get a 0 back for the second.
Yes! Zero means "SGX_ENABLED". :-) 

Install SGX driver and Fortanix EDP following: https://edp.fortanix.com/docs/installation/guide/. 


