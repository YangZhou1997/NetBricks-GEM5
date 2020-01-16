
#!/bin/bash
TRAFFIC=caida16_eth
# TRAFFIC=ictf2000_raw_eth

PORT_OPTIONS="dpdk:eth_pcap0,rx_pcap=/users/yangzhou/traffic/$TRAFFIC/chunck0.pcap,tx_pcap=/tmp/out.pcap"
# PORT_OPTIONS="0000:02:00.0"

# MODE=release
MODE=debug
# TARGET=x86_64-unknown-linux-gnu
TARGET=mipsel-unknown-linux-gnu
