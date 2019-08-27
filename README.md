[![Build Status](https://travis-ci.org/williamofockham/NetBricks.svg?branch=master)](https://travis-ci.org/williamofockham/NetBricks)

[NetBricks](http://netbricks.io/) is a Rust based framework for NFV development. Please refer to the
[paper](https://people.eecs.berkeley.edu/~apanda/assets/papers/osdi16.pdf) for information
about the architecture and design. Currently NetBricks requires a relatively modern Linux version.


## How to build Netbricks for GEM5 simulation
GEM5 does not support sigaltstack() (131) and sys_getrandom() (318). 
Thus, 
* We remove the stack overflow check in Rust runtime by modifying its rustc compiler, and get toolchain `stage1`. 
* We also remove all rand related crates, and implement our own random number generator using hash with seed following srand() and rand() in C.

1. Enter into root and export necessary env: 
    ```shell
    sudo su 
    export RTE_SDK=/users/yangzhou/tools/dpdk-stable-17.08.1
    source ~/.cargo/env
    cd SafeBricks
    ```

2. In the `SafeBricks` folder, you can change configuration in `config.sh`.
    * `MODE` decides whether you will run NF in debug mode or release mode

3. Build NetBricks (release or debug determined by `config.sh`):
    ```shell
    # release might require 10mins for the first time. 
    ./build_gem5.sh
    ```
    
4. Run the NF instances; currently, we support 7*2 NFs:
    ```shell
    ./run_static_binary.sh acl-fw 1048576
    # dpi lpm macswap maglev monitoring nat-tcp-v4 acl-fw-ipsec dpi-ipsec lpm-ipsec macswap-ipsec maglev-ipsec monitoring-ipsec nat-tcp-v4-ipsec
    ```
    You can specify the number of packet using commandline as shown above. 

    * **Note1:** Please use 2097152 (2M) packets for monitoring, and 1048576 (1M) packets for the others. 
    * **Note2:** NFs with IPSec might be too slow to run in GEM5; currently, we just test the NFs without IPSec. 
    <!-- Due to some unknown reason, GEM5-NFs will have "memory double free error", if we specify the number of packets in command line.  
    Instead, you can change the packet number (const PKT_NUM) in `/users/yangzhou/SafeBricks/framework-inside/src/scheduler/mod.rs` for NFs without IPSec and `/users/yangzhou/SafeBricks/framework-inside-ipsec/src/scheduler/mod.rs` for NFs with IPSec, and then **rebuild**. 
    Note that this error still appears in rustc 1.38.0-nightly, not only in our hacked toolchain (1.39.0-dev) (`rustc +stage1 -vV`) -->

5. Kill the running NF instances: 
    Wait for the specified number of packet getting processed, or ctrl + c
    
### Note
1. Binary files are located in `/users/yangzhou/SafeBricks/target/release` or `/users/yangzhou/SafeBricks/target/debug`. 
2. 
3. Since we are generating packets (both header and payload) with hash, the packet processing speed is pretty slow. 
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

1. Enter into root and export necessary env: 
    ```shell
    sudo su 
    export RTE_SDK=/users/yangzhou/tools/dpdk-stable-17.08.1
    source ~/.cargo/env
    cd SafeBricks
    ```

2. In the `SafeBricks` folder, you can change configuration in `config.sh`.
    *. `MODE` decides whether you will run NF in debug mode or release mode

3. Build NetBricks (release or debug determined by `config.sh`):
    ```shell
    # release might require 10mins for the first time. 
    ./build_musl.sh
    ```
    
4. Run the NF instances; currently, we support 7*2 NFs:
    ```shell
    ./run_musl.sh acl-fw 2097152
    # dpi lpm macswap maglev monitoring nat-tcp-v4 acl-fw-ipsec dpi-ipsec lpm-ipsec macswap-ipsec maglev-ipsec monitoring-ipsec nat-tcp-v4-ipsec
    ```
    You can specify the number of packet using commandline as shown above. 
    
5. Kill the running NF instances: 
    Wait for the specified number of packet getting processed, or ctrl + c
    

### Note
1. Binary files are located in `/users/yangzhou/SafeBricks/target/x86_64-unknown-linux-musl/release` or `/users/yangzhou/SafeBricks/target/x86_64-unknown-linux-musl/debug`. 

Up and Running
----------------

NetBricks can built within a Docker container. In this case, you do not need to
install any of the dependencies, and the final product can be run the same.
However to run NetBricks you still need to be on a machine or VM that is
correctly configured to run [DPDK](https://www.dpdk.org/) (version 17.08.1).

If you use our vagrant-based Developer environment, all DPDK configuration is
done for you. We also include the [MoonGen](//github.com/williamofockham/MoonGen) traffic generator and the
[Containernet](//github.com/containernet/containernet) virtual network simulator for testing and development.

## Setting up your local Ubuntu-16.04 environment

1. Clone our [utils](//github.com/YangZhou1997/utils) and [moonGen](//github.com/YangZhou1997/MoonGen)
   repositories into the same parent directory.
   ```shell
   host$ for repo in utils moonGen; do \
           git clone --recurse-submodules git@github.com:YangZhou1997/${repo}.git; \
         done
   ```

2. Update and install packages. Any kernel should be generally okay -- we have tested on 4.4.0-131-generic, 4.4.0-142-generic, 4.4.0-145-generic, and 4.15.0-15-generic. 
     
    ```shell
    host$ sudo bash ../utils/vm-setup.sh
    ```
    <!-- ```shell
    host$ sudo bash ../utils/vm-kernel-upgrade.sh #require rebooting
    host$ sudo shutdown -r now
    host$ sudo bash ../utils/vm-setup.sh
    ``` -->

3. After step 2, you machine meets the basic requirements of running NetBricks. Now you need to build and bind DPDK using [setupDpdk.sh](./setupDpdk.sh). 
    ```shell
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
    root$ source ~/.cargo/env
    ```
    
    [setupBuild.sh](./setupBuild.sh) will install the rust nightly, clang, and etc for running NetBricks. 

    This NetBricks codebase works on rust nightly-2019-05-22. You can override the rust version in current directory to nightly-2019-05-22 by:
    ```shell
    rustup install nightly-2019-05-22
    rustup override set nightly-2019-05-22
    ```

3. After step 2, you need to set ```RTE_SDK``` to the dpdk directory, and load cargo environment. Then you'll be able to compile and test NetBricks:
   ```shell
    root$ export RTE_SDK=/home/yangz/tools/dpdk-stable-17.08.1 # for instance.
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

## Creating a Developer environment with `vagrant`

1. Clone our [utils](//github.com/YangZhou1997/utils) and [moonGen](//github.com/YangZhou1997/MoonGen)
   repositories into the same parent directory.
   ```shell
   host$ for repo in utils moonGen; do \
           git clone --recurse-submodules git@github.com:YangZhou1997/${repo}.git; \
         done
   ```

2. [Install Vagrant](https://www.vagrantup.com/docs/installation/) and
   [VirtualBox](https://www.virtualbox.org/wiki/Downloads). You can check this [Google Doc](https://docs.google.com/document/d/1_QD0dZPr8JploJ-0CrKMJVc1TbGH7HhAFG96mzDXhPc/edit?usp=sharing) if any error happens.

3. Install the `vagrant-disksize` (required) and `vagrant-vbguest` (recommended)
   `vagrant-reload` (required) plugins:
   ```shell
   host$ vagrant plugin install vagrant-disksize vagrant-vbguest vagrant-reload
   ```

4. Symlink the Vagrantfile into the parent directory.
   ```shell
   host$ ln -s utils/Vagrantfile
   ```

**Note**: If you want, you can update the VirtualBox machine name (`vb.name`) or any other
VM settings within the `Vagrantfile` once it has been symlinked.

5. Boot the VM:
   ```shell
   host$ vagrant up
   ```

6. SSH into the running Vagrant VM,
   ```shell
   host$ vagrant ssh
   ```

7. Once you're within the Vagrant instance, run the `sandbox` container from NetBricks/ within Vagrant:
   ```shell
   vagrant$ make -f docker.mk run
   ```

8. After step 6, you'll be in the container and then can compile and test NetBricks via
   ```shell
   docker$ cd netbricks
   docker$ make build
   ...
   docker$ make test
   ...
   ```

For faster setup, you can run `make init` to handle steps *0* and *1* and *4*
for you.

The above steps will prepare your virtual machine with all of the appropriate
DPDK settings (multiple secondary NICs, install kernel modules, enable huge
pages, bind the extra interfaces to DPDK drivers) and install
[Containernet](https://containernet.github.io/) if you want to set up
simulations with your NFs.

If you have utils and MoonGen cloned as described in the steps above, those
repositories will be shared into the VM at `/vagrant/utils` and
`/vagrant/moongen` respectively.

## Developing with NetBricks within a Docker container

For development of NFs with NetBricks, we use a set of Docker containers to
install and bind ports use with DPDK, as well other dependencies. All of this
exists in our [utils sandbox](//github.com/YangZhou1997/utils), which can be
cloned accordingly and is part of our Developer environment above. As mentioned
in steps 6 and 7 above, you can run our sandbox container and develop and test
NetBricks via

```shell
$ make -f docker.mk run
```

and

```shell
docker$ cd netbricks
docker$ make build
...
docker$ make test
...
```

And you can run an example via:

```shell
docker$ make -e EXAMPLE=mtu-too-big run
`****

**Note**: Though be aware some of the examples contain `asserts` for testing NFs.

From within the container, you can also use [cargo-watch](https://github.com/passcod/cargo-watch) to handle compilation changes as you are developing:

```shell
docker$ cargo watch -x build --poll -c
```

Note: you can open additional terminals by getting the running container's container ID from `sudo docker ps`, and then get to the container with

```shell
$ docker exec -it <CONTAINER_ID> /bin/bash
```

## Environment

If you will be doing development work in this repo, you will need to have [Rust](https://www.rust-lang.org/en-US/install.html) and [rustfmt](https://github.com/rust-lang-nursery/rustfmt) (latest with nightly) installed, as well as [clang](https://clang.llvm.org/get_started.html) and [clang-format](https://clang.llvm.org/docs/ClangFormat.html).

Install these by doing the following:

```shell
host$ brew install clang-format rustup && \
  rustup-init -y && \
  rustup default nightly && \
  rustup component add rustfmt-preview --toolchain nightly
```

Then add the git pre-commit hook to your cloned repo for automatic source code formatting (if you didn't run `make init` earlier).

```shell
host$ mkdir -p .git/hooks && ln -s -f .hooks/pre-commit $(BASE_DIR)/.git/hooks/pre-commit
```

Dependencies
--------------

Building NetBricks requires the following dependency packages (on Debian):

```
apt-get install libcurl4-gnutls-dev libgnutls30 libgnutls-openssl-dev tcpdump libclang-dev libpcap-dev libnuma-dev
```

NetBricks also supports using SCTP as a control protocol. SCTP support requires
the use of `libsctp` (this is an optional dependency) which can be installed on
Debian using:

```
apt-get install libsctp-dev
```

Look further at the our [utils README](//github.com/williamofockham/utils/blob/master/README.md)
to understand the layout of our sandbox and design of our Docker images. If
you're building NetBricks locally, take a look at how we set out or [development VM](https://github.com/williamofockham/utils/blob/master/vm-setup.sh)
around transparent hugepages and the loading of modules. Read more about how
different PMDs (poll-mode drivers) require varying kernel drivers on the [DPDK site](https://doc.dpdk.org/guides/linux_gsg/linux_drivers.html).

Tuning
------
Changing some Linux parameters, including disabling C-State, and P-State; and isolating CPUs can greatly benefit NF
performance. In addition to these boot-time settings, runtime settings (e.g., disabling uncore frequency scaling and
setting the appropriate flags for Linux power management QoS) can greatly improve performance. The
[energy.sh](scripts/tuning/energy.sh) in [scripts/tuning](scripts/tuning) will set these parameter appropriately, and
it is recommended you run this before running the system.
