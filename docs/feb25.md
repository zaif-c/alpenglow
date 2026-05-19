```
cargo run --release --bin node -- --generate-config-files ip_list --config-name ag_node
```
```
RUST_LOG=error cargo run --release --bin node -- --config-name ag_node_0.toml &
RUST_LOG=error cargo run --release --bin node -- --config-name ag_node_1.toml &
RUST_LOG=error cargo run --release --bin node -- --config-name ag_node_2.toml &
RUST_LOG=error cargo run --release --bin node -- --config-name ag_node_3.toml
```

- commented out:
    - node.rs: open telemetry for local runs, along with the corresponding imports
    - pool.rs: notarized(-fallback), slow/fast finalized slot
    - udp.rs, simulated.rs: deserializing failed with

- added constructors for the rest of the sampling strategies in rotor.rs. 
    - easy to switch between sampling strategies by passing the desired sampler to the node.rs constructor
        - added helper functions in rotor.rs to call correct constructor
        - only type alias needs to be specified
    - can't switch sampling strategies without recompiling

- use src/bin/workload_generator.rs to send transactions
    - current metrics: 
        - TPS (send throughput): tps = txs_sent / elapsed
        - MB (total data sent)
        - Mb/s (data rate)
    - current metrics measure from generator to leader node, not between nodes

```
RUST_LOG=info cargo run --release --bin workload_generator \
  -- --validator 127.0.0.1:3004 \
     --transactions-per-second 200 \
     --transaction-size 512 \
     --initial-delay-secs 5
```

- manually adjust stakes in ag_node_*.toml files. 
    - make sure to update stakes in other node's gossips

    
- METRICS
    - Throughput: number of generated transactions finalized within given interval of time
    - Latency: time from when a block is produced to when its finalized (consensus + relay time)

