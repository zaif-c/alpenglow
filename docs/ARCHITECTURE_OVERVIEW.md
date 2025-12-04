# Alpenglow Protocol - Architecture Overview

## Introduction

Alpenglow is a new consensus protocol for Solana blockchain, designed as a research reference implementation. It's a global high-performance Proof-of-Stake blockchain that uses erasure coding for efficient block dissemination.

## High-Level Architecture

The Alpenglow protocol consists of several key components working together:

```
┌─────────────────────────────────────────────────────────────┐
│                    Alpenglow Node                            │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Consensus  │  │ Disseminator │  │   Shredder   │      │
│  │   (Votor)    │  │   (Rotor)    │  │              │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│         │                 │                  │              │
│         └─────────────────┼──────────────────┘              │
│                           │                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │     Pool     │  │  Blockstore │  │    Repair    │      │
│  │  (Votes &    │  │   (Shreds)  │  │              │      │
│  │  Certificates)│  │             │  │              │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│         │                 │                  │              │
│         └─────────────────┼──────────────────┘              │
│                           │                                  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Network Layer (UDP/TCP)                  │  │
│  │  - All2All (Consensus Messages)                      │  │
│  │  - Disseminator (Shreds)                             │  │
│  │  - Repair (Block Recovery)                           │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Consensus Module (`src/consensus/`)

The heart of the Alpenglow protocol, handling all consensus logic.

#### **Alpenglow** (Main Consensus Engine)
- **Purpose**: Orchestrates the entire consensus protocol
- **Key Responsibilities**:
  - Coordinates between all sub-components
  - Handles incoming messages (votes, certificates, shreds)
  - Manages block production (when this node is leader)
  - Detects standstill and triggers recovery

#### **Votor** (Voting Logic)
- **Purpose**: Implements the voting decision process
- **Key Responsibilities**:
  - Decides when to vote (notarize, skip, finalize)
  - Tracks voting state for each slot
  - Broadcasts votes via All2All network
  - Handles timeouts and crashed leaders
  - Manages parent-child block relationships

#### **Pool** (Vote & Certificate Storage)
- **Purpose**: Central data structure for consensus state
- **Key Responsibilities**:
  - Stores all votes and certificates for each slot
  - Tracks which blocks are notarized/finalized
  - Detects slashable offences (double voting, etc.)
  - Manages parent-ready tracking
  - Emits events to Votor when consensus conditions are met

#### **Blockstore** (Block Data Storage)
- **Purpose**: Stores and reconstructs blocks from shreds
- **Key Responsibilities**:
  - Receives shreds from disseminator
  - Reconstructs complete blocks using erasure coding
  - Verifies Merkle proofs
  - Notifies Pool when blocks are complete

#### **BlockProducer** (Leader Functionality)
- **Purpose**: Produces blocks when this node is the leader
- **Key Responsibilities**:
  - Collects transactions
  - Creates slices and shreds them
  - Sends shreds via disseminator
  - Manages block production timing

### 2. Disseminator Module (`src/disseminator/`)

Handles fast block dissemination across the network.

#### **Rotor** (Primary Dissemination Protocol)
- **Purpose**: Alpenglow's new block dissemination protocol
- **Key Features**:
  - Single-layer relay system (improvement over Turbine's multi-layer)
  - Uses stake-weighted sampling to select relayers
  - Leader sends shreds to sampled relayers
  - Relayers broadcast to all other validators
  - More efficient than simple broadcast

#### **Turbine** (Solana's Original Protocol)
- **Purpose**: Reference implementation of Solana's Turbine
- **Key Features**:
  - Multi-layered tree structure
  - Each layer forwards to next layer

#### **TrivialDisseminator**
- **Purpose**: Simple leader-to-everyone broadcast
- **Use Case**: Testing and benchmarking

### 3. Shredder Module (`src/shredder/`)

Handles erasure coding - splitting blocks into shreds and reconstructing them.

#### **Key Concepts**:
- **Slice**: A portion of a block (max ~32KB)
- **Shred**: Smallest unit (fits in UDP packet, ~1KB)
- **Data Shreds**: 32 data shreds per slice
- **Coding Shreds**: 32 coding shreds (erasure coding redundancy)
- **Total Shreds**: 64 shreds per slice

#### **Shredder Types**:
1. **RegularShredder**: Standard Reed-Solomon erasure coding
2. **CodingOnlyShredder**: Only produces coding shreds
3. **AontShredder**: Uses RAONT-RS all-or-nothing construction (encryption)
4. **PetsShredder**: Uses PETS all-or-nothing construction (encryption)

#### **Features**:
- Merkle tree verification for each shred
- Leader signature on Merkle root
- Can reconstruct slice from any 32 shreds (data or coding)

### 4. All2All Module (`src/all2all/`)

Handles consensus message broadcasting.

#### **TrivialAll2All**
- Simple best-effort broadcast to all validators
- Used for votes and certificates

#### **RobustAll2All**
- More robust implementation with retransmission
- Better reliability guarantees

### 5. Network Module (`src/network/`)

Abstracts network communication.

#### **UdpNetwork**
- UDP-based network implementation
- Used for all communication (consensus, shreds, repair)

#### **TcpNetwork**
- TCP-based network implementation (available but not primary)

#### **SimulatedNetwork**
- For testing and simulations
- Allows controlled network conditions

### 6. Repair Module (`src/repair.rs`)

Handles block recovery when blocks are missing.

#### **Key Features**:
- Double-Merkle tree based repair protocol
- Requests missing shreds from other validators
- Verifies responses using Merkle proofs
- Handles timeouts and retries

### 7. Crypto Module (`src/crypto/`)

Cryptographic primitives.

#### **Components**:
- **Signature**: Ed25519 signatures
- **AggSig**: Aggregate signatures for certificates
- **Merkle**: Merkle tree construction and verification
- **Hash**: SHA-256 hashing

## Data Flow

### Block Production (Leader)
1. **BlockProducer** collects transactions
2. Creates **Slice** from transactions
3. **Shredder** splits slice into 64 shreds (32 data + 32 coding)
4. Leader signs Merkle root of shreds
5. **Rotor** disseminates shreds:
   - Leader sends each shred to a sampled relayer
   - Relayer broadcasts to all other validators
6. Validators receive shreds and reconstruct blocks

### Consensus Flow
1. Validators receive shreds via **Disseminator**
2. **Blockstore** reconstructs blocks from shreds
3. When block is complete, **Blockstore** notifies **Pool**
4. **Pool** tracks votes and certificates
5. **Votor** receives events from **Pool** and **Blockstore**
6. **Votor** decides when to vote (notarize/skip/finalize)
7. Votes broadcast via **All2All** network
8. **Pool** aggregates votes into certificates
9. When enough votes, block is notarized/finalized

### Repair Flow
1. **Pool** detects missing block
2. Sends repair request via **Repair** module
3. **Repair** requests shreds from other validators
4. Receives shreds with Merkle proofs
5. Verifies and adds to **Blockstore**
6. Reconstructs block

## Key Protocol Parameters

- **DELTA**: Network transmission delay bound (250ms)
- **DELTA_BLOCK**: Time leader has for block production (400ms)
- **DELTA_FIRST_SLICE**: Time for first slice (10ms)
- **DELTA_TIMEOUT**: Timeout for leader's first slice (750ms)
- **DELTA_STANDSTILL**: Standstill detection timeout (10s)
- **DATA_SHREDS**: 32 data shreds per slice
- **TOTAL_SHREDS**: 64 total shreds per slice
- **MAX_DATA_PER_SHRED**: 1024 bytes
- **MAX_DATA_PER_SLICE**: ~32KB (after padding)

## Testing & Simulation

The codebase includes:
- **Unit tests**: In each module
- **Integration tests**: `tests/liveness.rs`
- **Simulations**: `src/bin/simulations/`
  - Latency simulations
  - Bandwidth analysis
  - Robustness testing
  - Comparison with other protocols (Pyjama, Ryse, Rotor)

## Entry Points

1. **`src/main.rs`**: Simple 2-node cluster demo
2. **`src/bin/node.rs`**: Standalone node implementation
3. **`src/bin/simulations/main.rs`**: Various protocol simulations
4. **`src/bin/performance_test.rs`**: Performance benchmarking

## Key Design Principles

1. **Erasure Coding**: Blocks split into shreds with redundancy
2. **Merkle Trees**: Cryptographic verification at multiple levels
3. **Stake-Weighted Sampling**: Rotor uses stake for relayer selection
4. **Event-Driven**: Components communicate via events/channels
5. **Modular**: Clear separation of concerns with trait abstractions
6. **Async**: Built on Tokio for high-performance async I/O

## References

See `README.md` for links to:
- Alpenglow Whitepaper
- Related research papers (Kudzu, DispersedSimplex, Simplex, Banyan)
- Solana TowerBFT Whitepaper
- Presentation videos and slides

