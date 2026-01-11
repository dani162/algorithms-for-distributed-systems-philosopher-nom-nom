#!/bin/bash

source ./config.sh
NUM_THINKERS=$1
START_THINKER_COMMAND="./target/release/thinker init-server 0.0.0.0:0 --init-server $INIT_SERVER_ADDRESS --save-config-dir ./config/ || sleep 100"

cargo build --release
ptyxis --new-window -- bash -c "
NUM_THINKERS=$NUM_THINKERS
for ((i = 0; i < NUM_THINKERS; i++)); do
  echo ptyxis --tab --title \"Thinker $i\" -- bash -c \"$START_THINKER_COMMAND\"
  ptyxis --tab --title \"Thinker $i\" -- bash -c \"$START_THINKER_COMMAND\"
done
"
