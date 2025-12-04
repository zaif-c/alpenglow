# Getting Started with Alpenglow

This guide will help you set up and test the Alpenglow codebase on your computer.

## Prerequisites

### 1. Install Rust

Alpenglow requires Rust 1.90 or later. Install Rust using rustup:

```bash
# Install rustup (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the prompts, then reload your shell
source $HOME/.cargo/env

# Verify installation
rustc --version  # Should show 1.90 or later
cargo --version
```

### 2. Install Additional Tools (Optional but Recommended)

For running all tests, you may need `cargo-nextest`:

```bash
cargo install cargo-nextest
```

For code quality checks:

```bash
cargo install cargo-machete
```

## Quick Start

### Step 1: Verify the Build

First, make sure the project compiles:

```bash
cd "/Users/zaifc/Programming/Solana Alpenglow/alpenglow"
cargo build
```

This will download dependencies and compile the project. The first build may take several minutes.

### Step 2: Run Basic Tests

Run the fast test suite:

```bash
./test.sh
```

Or if you don't have `cargo-nextest` installed:

```bash
cargo test
```

### Step 3: Run the Demo Cluster

Run a 2-node local cluster:

```bash
./run.sh
```

This will:
- Start 2 nodes communicating via UDP on localhost
- Show console output with consensus progress
- Run until you press Ctrl+C

**Note**: The demo runs indefinitely. Press `Ctrl+C` to stop it.

### Step 4: Run Code Quality Checks

Run the full check suite (linting, formatting, docs, tests):

```bash
./check.sh
```

This runs:
- `cargo clippy` - Linting
- `cargo fmt --check` - Format checking
- `cargo doc` - Documentation generation
- `cargo machete` - Unused dependency checking
- Full test suite

## Testing Options

### Fast Tests (Default)
```bash
./test.sh
# or
cargo test
```

### Slow Tests (More Comprehensive)
```bash
./test.sh slow
```

### Documentation Tests
```bash
./test.sh doc
```

### Sequential Tests (Some tests need to run sequentially)
```bash
./test.sh sequential
```

### Full CI Suite
```bash
./check.sh
```

## Running Simulations

The codebase includes simulation binaries for protocol analysis:

### 1. Download Required Data (for latency simulations)

```bash
./download_data.sh
```

### 2. Run Simulations

```bash
RUST_LOG="simulations=debug" cargo run --release --bin=simulations
```

Configuration is done via constants in `src/bin/simulations/main.rs`.

## Running Benchmarks

Run micro-benchmarks:

```bash
cargo bench
```

## Running Standalone Nodes

For a more realistic multi-node setup:

### 1. Create IP List File

Create a file (e.g., `ip_list`) with node addresses:

```
127.0.0.1:3000
127.0.0.1:3005
127.0.0.1:3010
127.0.0.1:3015
```

### 2. Generate Config Files

```bash
cargo run --release --bin node -- --generate-config-files ip_list --config-name ag_node
```

This creates config files: `ag_node_0.toml`, `ag_node_1.toml`, etc.

### 3. Run Nodes

In separate terminals or as background processes:

```bash
cargo run --release --bin node -- --config-name ag_node_0.toml &
cargo run --release --bin node -- --config-name ag_node_1.toml &
cargo run --release --bin node -- --config-name ag_node_2.toml &
cargo run --release --bin node -- --config-name ag_node_3.toml
```

## Troubleshooting

### Build Errors

**Error: "rustc version too old"**
- Update Rust: `rustup update stable`
- The project requires Rust 1.90+

**Error: "cannot find crate"**
- Run `cargo clean` and `cargo build` again
- Check your internet connection (dependencies need to be downloaded)

### Test Failures

**Tests fail with network errors**
- Make sure no other process is using the ports
- Try running tests sequentially: `./test.sh sequential`

**Tests timeout**
- Some tests may take longer on slower machines
- Try running slow tests: `./test.sh slow`

### Permission Errors

**Scripts won't execute**
- Make scripts executable: `chmod +x *.sh`

## Understanding the Output

### Demo Output (`./run.sh`)

When running the demo, you'll see:
- Node startup messages
- Consensus progress (votes, certificates)
- Block production messages
- Network communication logs

The output uses the `logforth` crate for structured logging.

### Test Output

Tests show:
- ✅ Passing tests
- ❌ Failing tests with error messages
- Test execution time

## Next Steps

1. **Read the Architecture Overview**: See `docs/ARCHITECTURE_OVERVIEW.md` for a detailed explanation of the codebase structure.

2. **Explore the Code**:
   - Start with `src/lib.rs` - main library entry point
   - Check `src/consensus.rs` - core consensus logic
   - Look at `src/disseminator/rotor.rs` - block dissemination
   - Review `src/shredder.rs` - erasure coding

3. **Read the Whitepaper**: See `README.md` for links to the Alpenglow whitepaper and related research.

4. **Run Simulations**: Experiment with different protocol parameters in the simulations.

## Project Structure Quick Reference

```
alpenglow/
├── src/
│   ├── consensus/        # Core consensus protocol
│   ├── disseminator/     # Block dissemination (Rotor, Turbine)
│   ├── shredder/         # Erasure coding
│   ├── network/          # Network abstractions
│   ├── crypto/           # Cryptographic primitives
│   └── bin/              # Executable binaries
├── tests/                # Integration tests
├── benches/              # Benchmarks
├── run.sh               # Run 2-node demo
├── test.sh              # Run tests
└── check.sh             # Full code quality check
```

## Getting Help

- **Code Issues**: Check the code comments and documentation
- **Protocol Questions**: See the whitepaper and references in `README.md`
- **Security Issues**: Contact quentin@anza.xyz (see README)

