#!/bin/bash

# Run benchmarks and show only the benchmark results
# This filters out the unit test output

cargo bench 2>&1 | grep -A 1000 "Running benches"
