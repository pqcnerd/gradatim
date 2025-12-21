#!/bin/bash
# Shell script to build/run Rust in WSL/Linux
cd "$(dirname "$0")" || exit 1

# Run cargo with any passed arguments, default to "run" if no args
if [ $# -eq 0 ]; then
    cargo run
else
    cargo "$@"
fi
