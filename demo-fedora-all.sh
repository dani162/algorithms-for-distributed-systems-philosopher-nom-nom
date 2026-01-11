#!/bin/bash

NUM_THINKER=7
NUM_NEXT_THINKERS=4
NUM_TOKENS=3
VISUALIZER_ADDRESS=192.168.51.10:3334
INIT_SERVER_ADDRESS=192.168.51.10:3333
START_THINKER_COMMAND="./target/release/init 192.168.50.10:3333 --thinker $NUM_THINKER --next-thinkers-amount $NUM_NEXT_THINKERS --tokens $NUM_TOKENS --visualizer || sleep 100"

rm -r ./config/
mkdir ./config/
cargo build --release
ptyxis --new-window -- bash -c "$START_THINKER_COMMAND"
sleep 0.5
ptyxis --new-window -- bash -c "./fedora-thinkers-demo.sh 5"
sleep 0.5
ptyxis --new-window -- bash -c "./fedora-forkss-demo.sh 4"
sleep 0.5
ptyxis --new-window -- bash -c "./target/release/visualizer $VISUALIZER_ADDRESS --init-server $INIT_SERVER_ADDRESS"
