
#!/bin/bash
TRAFFIC=caida16
# TRAFFIC=ictf2000_raw

PORT_OPTIONS="dpdk:eth_pcap0,rx_pcap=/users/yangzhou/traffic/$TRAFFIC/chunck0.pcap,tx_pcap=/tmp/out.pcap"
# PORT_OPTIONS="0000:02:00.0"

MODE=release
# MODE=debug