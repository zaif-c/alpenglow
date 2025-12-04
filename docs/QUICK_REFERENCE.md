# Alpenglow Quick Reference

## 🚀 Quick Start Commands

```bash
# 1. Verify your setup
./verify_setup.sh

# 2. Build the project
cargo build

# 3. Run tests
./test.sh

# 4. Run 2-node demo
./run.sh
```

## 📚 Documentation Files

- **`docs/ARCHITECTURE_OVERVIEW.md`** - Detailed architecture explanation
- **`docs/GETTING_STARTED.md`** - Complete setup and testing guide
- **`docs/QUICK_REFERENCE.md`** - This file (quick command reference)

## 🔧 Common Commands

### Building
```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo check              # Quick syntax check
```

### Testing
```bash
./test.sh                # Fast tests
./test.sh slow           # Comprehensive tests (10+ min)
./test.sh doc            # Documentation tests
./test.sh sequential     # Sequential tests
cargo test               # Standard cargo test
```

### Running
```bash
./run.sh                 # 2-node demo cluster
cargo run --release --bin=alpenglow  # Same as above
cargo run --release --bin=simulations  # Run simulations
cargo run --release --bin=node -- --help  # Standalone node
```

### Code Quality
```bash
./check.sh               # Full check (lint, fmt, doc, test)
cargo clippy             # Linting
cargo fmt                # Format code
cargo doc                # Generate docs
```

### Benchmarks
```bash
cargo bench              # Run benchmarks
```

## 📁 Key Directories

```
src/
├── consensus/          # Core consensus (Votor, Pool, Blockstore)
├── disseminator/       # Block dissemination (Rotor, Turbine)
├── shredder/           # Erasure coding
├── network/            # Network layer
├── crypto/             # Cryptography
└── bin/                # Executables
```

## 🎯 Key Components

| Component | Purpose | Location |
|-----------|---------|----------|
| **Alpenglow** | Main consensus engine | `src/consensus.rs` |
| **Votor** | Voting logic | `src/consensus/votor.rs` |
| **Pool** | Vote/certificate storage | `src/consensus/pool.rs` |
| **Rotor** | Block dissemination | `src/disseminator/rotor.rs` |
| **Shredder** | Erasure coding | `src/shredder.rs` |
| **Repair** | Block recovery | `src/repair.rs` |

## 🐛 Troubleshooting

**Rust not found?**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Build fails?**
```bash
cargo clean
cargo build
```

**Tests fail?**
```bash
# Try sequential tests
./test.sh sequential

# Or run specific test
cargo test test_name
```

**Scripts not executable?**
```bash
chmod +x *.sh
```

## 📖 Learning Path

1. **Read** `docs/ARCHITECTURE_OVERVIEW.md` for high-level understanding
2. **Run** `./verify_setup.sh` to check your environment
3. **Build** with `cargo build`
4. **Test** with `./test.sh`
5. **Explore** the code starting with `src/lib.rs`
6. **Run** the demo with `./run.sh`
7. **Read** the whitepaper (links in README.md)

## 🔗 Important Links

- **Whitepaper**: See README.md
- **Presentations**: See README.md
- **GitHub**: https://github.com/qkniep/alpenglow

