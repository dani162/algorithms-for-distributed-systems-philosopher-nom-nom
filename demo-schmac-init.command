#!/bin/bash

cd "$(dirname "$0")"
source ./config.sh
./target/release/init $INIT_SERVER_ADDRESS --thinker $NUM_THINKERS --next-thinkers-amount $NUM_NEXT_THINKERS --tokens $NUM_TOKENS || sleep 100
