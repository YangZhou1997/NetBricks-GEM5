
#!/bin/bash
TRAFFIC=caida16_eth
# TRAFFIC=ictf2000_raw_eth

PORT_OPTIONS="dpdk:eth_pcap0,rx_pcap=/users/yangzhou/traffic/$TRAFFIC/chunck0.pcap,tx_pcap=/tmp/out.pcap"
# PORT_OPTIONS="0000:02:00.0"

MODE=release
# MODE=debug

TARGET=aarch64-unknown-linux-gnu
# TARGET=x86_64-unknown-linux-musl

# TARGET=x86_64-unknown-linux-gnu
# TARGET=mipsel-unknown-linux-gnu
# TARGET=mipsel-unknown-linux-musl
# TARGET=aarch64-unknown-linux-musl
# TARGET=arm-unknown-linux-gnueabi
# TARGET=arm-unknown-linux-musleabi
# TARGET=arm-unknown-linux-musleabihf
# TARGET=armv5te-unknown-linux-musleabi
# TARGET=armv7-unknown-linux-musleabihf
# TARGET=mipsel-unknown-linux-musl
