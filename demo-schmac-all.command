#!/bin/bash

source ./config.sh

rm -r ./config/
mkdir ./config/
cargo build --release

for ((i = 0; i < NUM_THINKERS_MAC; i++)); do
  open ./demo-schmac-philosopher.command
done
for ((i = 0; i < NUM_FORKS_MAC; i++)); do
  open ./demo-schmac-fork.command
done
