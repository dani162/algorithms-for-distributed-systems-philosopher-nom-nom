#!/bin/bash

source ./config.sh
START_THINKER_COMMAND="./target/release/init $INIT_SERVER_ADDRESS --thinker $NUM_THINKERS --next-thinkers-amount $NUM_NEXT_THINKERS --tokens $NUM_TOKENS --visualizer || sleep 100"

rm -r ./config/
mkdir ./config/
cargo build --release
ptyxis --new-window -- bash -c "$START_THINKER_COMMAND"
sleep 1
ptyxis --new-window -- bash -c "./demo-fedora-thinkers.sh $NUM_THINKERS_FEDORA"
sleep 1
ptyxis --new-window -- bash -c "./demo-fedora-forks.sh $NUM_FORKS_FEDORA"
sleep 1
ptyxis --new-window -- bash -c "./target/release/visualizer $VISUALIZER_ADDRESS --init-server $INIT_SERVER_ADDRESS"
