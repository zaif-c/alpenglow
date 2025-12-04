#!/bin/bash

# Run the demo with INFO level logging (shows only important consensus events)
#
# This shows:
#   - Block production
#   - Notarizations
#   - Finalizations
#   - Skips
#   - Warnings/Errors
#
# It filters out DEBUG messages like "dropping duplicate shred" which are:
#   - Expected behavior (duplicates are normal in distributed networks)
#   - Useful for debugging network issues, but noisy for general observation
#
# For full tracing including all network details, use ./run.sh instead

RUST_LOG="alpenglow=info" RUST_BACKTRACE=1 cargo run --release --bin=alpenglow

