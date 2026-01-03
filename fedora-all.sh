#!/bin/bash

NUM_THINKER=$1
NUM_TOKENS=$2
VISUALIZER_ADDRESS=127.0.0.1:3334
INIT_SERVER_ADDRESS=127.0.0.1:3333
START_THINKER_COMMAND="./target/release/init 127.0.0.1:3333 --thinker $NUM_THINKER --tokens $NUM_TOKENS --visualizer || sleep 100"

cargo build --release
ptyxis --new-window -- bash -c "$START_THINKER_COMMAND"
sleep 0.5
ptyxis --new-window -- bash -c "./fedora-thinkers.sh $1"
sleep 0.5
ptyxis --new-window -- bash -c "./fedora-forks.sh $1"
sleep 0.5
ptyxis --new-window -- bash -c "./target/release/visualizer $VISUALIZER_ADDRESS --init-server $INIT_SERVER_ADDRESS"
