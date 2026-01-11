#!/bin/bash

NUM_THINKERS=2
NUM_FORKS=3

cd ~/dev/projects/fun/algorithms-for-distributed-systems-philosopher-nom-nom
rm -r ./config/
mkdir ./config/
cargo build --release

for ((i = 0; i < NUM_THINKERS; i++)); do
  cd ~/dev/projects/fun/algorithms-for-distributed-systems-philosopher-nom-nom
  open demo-schmac-philosopher.command
done
for ((i = 0; i < NUM_FORKS; i++)); do
  cd ~/dev/projects/fun/algorithms-for-distributed-systems-philosopher-nom-nom
  open demo-schmac-fork.command
done
