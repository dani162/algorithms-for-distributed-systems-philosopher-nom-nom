#!/bin/bash

INIT_SERVER_ADDRESS=192.168.51.10:3333
cd ~/dev/projects/fun/algorithms-for-distributed-systems-philosopher-nom-nom
./target/release/thinker init-server 0.0.0.0:0 --init-server $INIT_SERVER_ADDRESS --save-config-dir ./config/ || sleep 100

