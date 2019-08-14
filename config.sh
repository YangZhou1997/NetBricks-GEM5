
#!/bin/bash

MODE=release
# PORT_OPTIONS="dpdk:eth_pcap0,rx_pcap=/users/yangzhou/traffic/ictf2000_raw/chunck0.pcap,tx_pcap=/tmp/out.pcap"
PORT_OPTIONS="dpdk:eth_pcap0,rx_pcap=/users/yangzhou/traffic/caida16/chunck0.pcap,tx_pcap=/tmp/out.pcap"

# PORT_OPTIONS="0000:02:00.0"