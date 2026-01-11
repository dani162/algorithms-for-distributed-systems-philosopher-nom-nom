#!/bin/bash

source ./config.sh
NUM_FORKS=$1
START_FORK_COMMAND="./target/release/fork init-server 0.0.0.0:0 --init-server $INIT_SERVER_ADDRESS --save-config-dir ./config/ || sleep 100"

ptyxis --new-window -- bash -c "
NUM_FORKS=$NUM_FORKS
for ((i = 0; i < NUM_FORKS; i++)); do
  ptyxis --tab --title \"Fork $i\" -- bash -c \"$START_FORK_COMMAND\"
done
"
