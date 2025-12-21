#!/bin/bash

NUM_THINKER=$1
INIT_SERVER_ADDRESS=127.0.0.1:3333
START_THINKER_COMMAND="./target/release/thinker 0.0.0.0:0 --init-server $INIT_SERVER_ADDRESS"

cargo build --release
ptyxis --new-window -- bash -c "
NUM_THINKER=$NUM_THINKER
for ((i = 0; i < NUM_THINKER; i++)); do
  ptyxis --tab --title \"Thinker $i\" -- bash -c \"$START_THINKER_COMMAND\"
done
"
