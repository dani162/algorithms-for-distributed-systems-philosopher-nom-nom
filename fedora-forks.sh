#!/bin/bash

NUM_FORKS=$1
INIT_SERVER_ADDRESS=127.0.0.1:3333
START_FORK_COMMAND="./target/release/fork 0.0.0.0:0 --init-server $INIT_SERVER_ADDRESS"

cargo build --release
ptyxis --new-window -- bash -c "
NUM_FORKS=$NUM_FORKS
for ((i = 0; i < NUM_FORKS; i++)); do
  ptyxis --tab --title \"Fork $i\" -- bash -c \"$START_FORK_COMMAND\"
done
"
