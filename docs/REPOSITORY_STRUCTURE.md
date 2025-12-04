# Repository Structure

This document explains the organization of the Alpenglow codebase and what each directory/file contains.

## Directory Tree

```
alpenglow/
├── src/                    # Main source code
│   ├── lib.rs             # Library entry point, public API
│   ├── main.rs            # Main binary (2-node demo)
│   │
│   ├── consensus/         # Core consensus protocol
│   ├── disseminator/      # Block dissemination protocols
│   ├── shredder/         # Erasure coding
│   ├── network/          # Network abstractions
│   ├── crypto/           # Cryptographic primitives
│   ├── all2all/          # All-to-all broadcast
│   ├── types/            # Core data types
│   ├── bin/              # Executable binaries
│   └── ...
│
├── benches/              # Benchmark code
├── tests/                # Integration tests
├── data/                 # Data files for simulations
├── docs/                 # Documentation files
├── *.sh                  # Shell scripts
└── README.md             # Main project README
```

## Source Code (`src/`)

### Core Library Files

- **`lib.rs`** - Main library entry point
  - Defines public API
  - Re-exports main types and traits
  - Contains `create_test_nodes()` helper for testing

- **`main.rs`** - Default binary executable
  - Runs 2-node local cluster demo
  - Entry point for `cargo run --bin=alpenglow`

- **`logging.rs`** - Logging configuration
  - Sets up `logforth` logger
  - Configures log levels and formatting

### Consensus Module (`src/consensus/`)

The heart of the protocol - handles all consensus logic.

```
consensus/
├── consensus.rs          # Main Alpenglow struct, orchestrates everything
├── votor.rs             # Voting logic (decides when to vote)
├── pool.rs              # Vote/certificate storage and tracking
├── blockstore.rs        # Block data storage
├── block_producer.rs    # Leader functionality (produces blocks)
├── cert.rs              # Certificate types (Notar, Final, Skip, etc.)
├── vote.rs              # Vote types
├── epoch_info.rs        # Validator information and epoch data
│
├── blockstore/
│   └── slot_block_data.rs  # Per-slot block reconstruction logic
│
└── pool/
    ├── slot_state.rs           # Per-slot vote/cert state
    ├── finality_tracker.rs     # Tracks finalized slots
    └── parent_ready_tracker/   # Tracks when parent blocks are ready
        └── parent_ready_state.rs
```

**Key Files:**
- **`consensus.rs`** - Main `Alpenglow` struct that coordinates everything
- **`votor.rs`** - Voting decision logic
- **`pool.rs`** - Central data structure for consensus state
- **`blockstore.rs`** - Stores and reconstructs blocks from shreds

### Disseminator Module (`src/disseminator/`)

Block dissemination protocols for efficiently spreading blocks across the network.

```
disseminator/
├── disseminator.rs      # Trait definition
├── rotor.rs            # Alpenglow's Rotor protocol (primary)
├── turbine.rs          # Solana's Turbine protocol
├── trivial.rs          # Simple broadcast (for testing)
│
└── rotor/
    └── sampling_strategy.rs  # Relayer sampling strategies
```

**Key Files:**
- **`rotor.rs`** - Rotor dissemination (single-layer relay system)
- **`turbine.rs`** - Turbine dissemination (multi-layer tree)
- **`sampling_strategy.rs`** - Different strategies for selecting relayers

### Shredder Module (`src/shredder/`)

Erasure coding - splits blocks into shreds and reconstructs them.

```
shredder/
├── shredder.rs         # Trait definition, main shredder types
├── reed_solomon.rs     # Reed-Solomon erasure coding implementation
├── validated_shred.rs  # Shred validation logic
├── validated_shreds.rs # Collection of validated shreds
├── shred_index.rs     # Shred indexing
└── pool.rs            # Shredder pool for concurrent access
```

**Key Files:**
- **`shredder.rs`** - Defines `Shredder` trait and implementations:
  - `RegularShredder` - Standard Reed-Solomon
  - `AontShredder` - RAONT-RS all-or-nothing
  - `PetsShredder` - PETS all-or-nothing
  - `CodingOnlyShredder` - Only coding shreds

### Network Module (`src/network/`)

Network communication abstractions.

```
network/
├── network.rs          # Trait definitions
├── udp.rs              # UDP network implementation
├── tcp.rs              # TCP network implementation
├── simulated.rs        # Simulated network (for testing)
│
└── simulated/
    ├── core.rs              # Core simulation logic
    ├── ping_data.rs         # Real-world ping data
    ├── stake_distribution.rs # Stake distribution simulation
    └── token_bucket.rs      # Bandwidth limiting
```

**Key Files:**
- **`udp.rs`** - UDP-based network (primary)
- **`simulated.rs`** - Simulated network for testing/simulations

### Crypto Module (`src/crypto/`)

Cryptographic primitives.

```
crypto/
├── crypto.rs          # Module exports
├── signature.rs       # Ed25519 signatures
├── aggsig.rs         # Aggregate signatures (BLS)
├── hash.rs           # SHA-256 hashing
└── merkle.rs         # Merkle tree construction/verification
```

**Key Files:**
- **`signature.rs`** - Ed25519 signing/verification
- **`aggsig.rs`** - BLS aggregate signatures for certificates
- **`merkle.rs`** - Merkle trees for block/slice verification

### All2All Module (`src/all2all/`)

All-to-all broadcast for consensus messages.

```
all2all/
├── all2all.rs        # Trait definition
├── trivial.rs        # Simple broadcast
└── robust.rs         # Robust broadcast with retransmission
```

**Key Files:**
- **`trivial.rs`** - Simple best-effort broadcast
- **`robust.rs`** - More reliable with retransmission

### Types Module (`src/types/`)

Core data types used throughout the codebase.

```
types/
├── types.rs          # Module exports
├── slot.rs           # Slot number type
├── slice.rs          # Slice (portion of block)
└── slice_index.rs   # Slice indexing
```

**Key Files:**
- **`slot.rs`** - Slot number abstraction
- **`slice.rs`** - Slice data structure

### Binaries (`src/bin/`)

Executable programs.

```
bin/
├── all2all_test.rs        # All2All protocol testing
├── node.rs                # Standalone node implementation
├── performance_test.rs    # Performance benchmarks
├── workload_generator.rs  # Transaction workload generation
│
└── simulations/           # Protocol simulations
    ├── main.rs                    # Main simulation entry point
    ├── alpenglow.rs              # Alpenglow protocol simulation
    ├── rotor.rs                  # Rotor dissemination simulation
    ├── pyjama.rs                 # Pyjama protocol comparison
    ├── ryse.rs                   # Ryse protocol comparison
    ├── quorum_robustness.rs      # Quorum robustness analysis
    └── discrete_event_simulator/ # Event-based simulator
```

**Key Binaries:**
- **`main.rs`** (via `src/main.rs`) - 2-node demo cluster
- **`node.rs`** - Standalone node for multi-node clusters
- **`simulations/main.rs`** - Protocol simulations and analysis

### Other Source Files

- **`repair.rs`** - Block repair protocol (recover missing blocks)
- **`validator.rs`** - Validator abstraction (currently minimal)
- **`test_utils.rs`** - Testing utilities and helpers

## Benchmarks (`benches/`)

Micro-benchmarks for performance analysis.

```
benches/
├── crypto.rs         # Cryptographic operations
├── network.rs        # Network serialization
├── shredder.rs      # Erasure coding operations
└── disseminator.rs  # Block dissemination
```

Run with: `cargo bench`

## Tests (`tests/`)

Integration tests.

```
tests/
└── liveness.rs      # Liveness tests (crash recovery, etc.)
```

Run with: `./test.sh`

## Data Files (`data/`)

Data files used by simulations.

```
data/
├── servers-2020-07-19.csv              # Server locations
├── pings-2020-07-19-2020-07-20.csv     # Real-world ping data
├── mainnet_validators_epoch*.json      # Solana validator data
├── sui_validators.csv                  # Sui validator data
└── countries_iso3166.csv               # Country codes
```

Download with: `./download_data.sh`

## Scripts

Shell scripts for common tasks.

- **`run.sh`** - Run 2-node demo (DEBUG logging)
- **`run_info.sh`** - Run 2-node demo (INFO logging)
- **`test.sh`** - Run test suite
- **`check.sh`** - Full code quality check (lint, fmt, doc, test)
- **`verify_setup.sh`** - Verify environment setup
- **`download_data.sh`** - Download simulation data
- **`run_bench.sh`** - Run benchmarks

## Configuration Files

- **`Cargo.toml`** - Rust project configuration and dependencies
- **`Cargo.lock`** - Locked dependency versions
- **`rustfmt.toml`** - Code formatting configuration

## Documentation (`docs/`)

All documentation files are organized in the `docs/` directory:

- **`README.md`** - Project overview and quick start (in root directory)
- **`docs/ARCHITECTURE_OVERVIEW.md`** - Protocol architecture explanation
- **`docs/REPOSITORY_STRUCTURE.md`** - This file (repo organization)
- **`docs/GETTING_STARTED.md`** - Setup and testing guide
- **`docs/QUICK_REFERENCE.md`** - Command reference
- **`docs/UNDERSTANDING_DEMO_OUTPUT.md`** - Understanding demo logs

## Key Entry Points

### For Understanding the Code

1. **Start here**: `src/lib.rs` - Public API and main types
2. **Consensus logic**: `src/consensus.rs` - Main consensus engine
3. **Voting**: `src/consensus/votor.rs` - Voting decisions
4. **Block storage**: `src/consensus/blockstore.rs` - Block reconstruction
5. **Dissemination**: `src/disseminator/rotor.rs` - Block spreading

### For Running Code

1. **Demo**: `./run.sh` or `./run_info.sh`
2. **Tests**: `./test.sh`
3. **Benchmarks**: `cargo bench`
4. **Standalone node**: `cargo run --bin node -- --help`

### For Learning

1. **Architecture**: `docs/ARCHITECTURE_OVERVIEW.md`
2. **Setup**: `docs/GETTING_STARTED.md`
3. **This file**: `docs/REPOSITORY_STRUCTURE.md`

## Module Dependencies

```
lib.rs
├── consensus.rs
│   ├── votor.rs
│   ├── pool.rs
│   ├── blockstore.rs
│   └── block_producer.rs
├── disseminator.rs
│   └── rotor.rs
├── shredder.rs
├── network.rs
├── crypto.rs
├── all2all.rs
└── types.rs
```

## File Naming Conventions

- **Module files**: `module_name.rs` (e.g., `consensus.rs`)
- **Sub-modules**: `module_name/submodule.rs` (e.g., `consensus/votor.rs`)
- **Traits**: Defined in module files, implementations in sub-modules
- **Tests**: In same file with `#[cfg(test)]` or in `tests/` directory
- **Binaries**: In `src/bin/` directory

## Code Organization Principles

1. **Modular**: Each major component is in its own module
2. **Trait-based**: Abstractions via traits (e.g., `Disseminator`, `Network`)
3. **Testable**: Tests co-located with code or in `tests/`
4. **Documented**: Extensive inline documentation
5. **Type-safe**: Strong typing throughout (Rust)

## Finding Your Way Around

**Want to understand consensus?**
→ Start with `src/consensus.rs`, then `votor.rs` and `pool.rs`

**Want to understand block dissemination?**
→ Start with `src/disseminator.rs`, then `rotor.rs`

**Want to understand erasure coding?**
→ Start with `src/shredder.rs`

**Want to run something?**
→ Check `README.md` or `docs/QUICK_REFERENCE.md`

**Want to see how it all fits together?**
→ Read `docs/ARCHITECTURE_OVERVIEW.md`

