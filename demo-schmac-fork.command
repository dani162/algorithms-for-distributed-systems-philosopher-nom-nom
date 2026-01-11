#!/bin/bash

cd "$(dirname "$0")"
source ./config.sh
./target/release/fork init-server 0.0.0.0:0 --init-server $INIT_SERVER_ADDRESS --save-config-dir ./config/ || sleep 100
