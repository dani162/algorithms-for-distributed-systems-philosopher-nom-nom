#!/bin/bash

NUM_THINKER=$1
NUM_TOKENS=$2
START_THINKER_COMMAND="./target/release/init 127.0.0.1:3333 --thinker $NUM_THINKER --tokens $NUM_TOKENS"

cargo build --release
ptyxis --new-window -- bash -c "$START_THINKER_COMMAND"
sleep 1
ptyxis --new-window -- bash -c "./fedora-thinkers.sh $1"
sleep 1
ptyxis --new-window -- bash -c "./fedora-forks.sh $1"
