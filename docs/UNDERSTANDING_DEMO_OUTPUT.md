# Understanding the Demo Output

When you run `./run.sh`, you'll see a lot of log messages. This guide explains what they mean and how to filter them.

## What You're Seeing

### DEBUG Messages (Most Common)

**"dropping duplicate shred"** - This is **normal and expected**!
- Shreds (block fragments) can arrive multiple times through the network
  - Rotor sends shreds to relayers, relayers broadcast to all validators
  - Network paths can overlap, causing the same shred to arrive multiple times
- The system automatically drops duplicates (this is correct behavior)
- **Why it's logged**: Useful for debugging network issues
  - High duplicate rate might indicate network problems
  - Helps trace network topology and routing
- **Why it's DEBUG level**: It's expected behavior, not an error
- **When to care**: Only if you're debugging network issues or want full trace

**Other DEBUG messages:**
- `voted notar for slot X` - A validator voted to notarize a block
- `voted skip for slot X` - A validator voted to skip an empty slot
- Network communication details

### INFO Messages (Important Events)

These are the **key consensus events** you should watch for:

1. **`notarized(-fallback) block XXXXXXXX in slot Y`**
   - A block has been notarized (enough validators agreed on it)
   - `XXXXXXXX` is the first 8 hex characters of the block hash
   - This means the block is confirmed by the network

2. **`fast finalized slot Y`**
   - A slot was finalized quickly (optimistic path)
   - The block is now permanently part of the chain

3. **`slow finalized slot Y`**
   - A slot was finalized via the slower path
   - Still permanent, just took longer

4. **`skipped slot Y`**
   - A slot was skipped (no block was produced)
   - This is normal when a leader is offline or slow

5. **`produced block in slot Y`**
   - The leader produced a new block
   - This node is the leader for this slot

## Filtering the Output

### Option 1: Show Only Important Events (Recommended for Observation)

Run with INFO level only:

```bash
RUST_LOG="alpenglow=info" cargo run --release --bin=alpenglow
# Or use the convenience script:
./run_info.sh
```

This will show only:
- Notarizations
- Finalizations
- Block production
- Skips
- Warnings/Errors

**Note**: This filters out DEBUG messages like "dropping duplicate shred", which are:
- **Expected behavior** (duplicates are normal in distributed networks)
- **Useful for debugging** network topology and routing issues
- **Noisy for general observation** (can be hundreds per second)

**When to use**: When you want to see consensus progress without network noise

### Option 2: Show Consensus Events Only

Filter to see only consensus-related messages:

```bash
RUST_LOG="alpenglow::consensus=info" cargo run --release --bin=alpenglow
```

### Option 3: See Everything (Full Tracing)

The `run.sh` script uses DEBUG level for complete traceability:

```bash
./run.sh  # Shows DEBUG + INFO + WARN + ERROR
```

**When to use**: 
- Debugging network issues
- Understanding network topology
- Tracing exactly what's happening at the network level
- Investigating why blocks aren't arriving

**What you'll see**:
- Every duplicate shred drop (useful for network analysis)
- All vote broadcasts
- All network messages
- Complete consensus state transitions

### Option 4: Custom Filter

You can combine filters:

```bash
# Show consensus info, but blockstore debug
RUST_LOG="alpenglow::consensus=info,alpenglow::consensus::blockstore=debug" \
  cargo run --release --bin=alpenglow
```

## What a Healthy Run Looks Like

You should see a pattern like this:

```
 INFO notarized(-fallback) block a1b2c3d4 in slot 0
 INFO fast finalized slot 0
 INFO produced block in slot 1
 INFO notarized(-fallback) block e5f6g7h8 in slot 1
 INFO fast finalized slot 1
 INFO produced block in slot 2
 ...
```

**What this means:**
1. Blocks are being produced (by the leader)
2. Blocks are being notarized (network agrees)
3. Blocks are being finalized (permanently added to chain)
4. The chain is making progress!

## Understanding the Consensus Flow

Here's what happens behind the scenes:

1. **Leader produces block** → Splits into shreds → Sends via Rotor
2. **Validators receive shreds** → Reconstruct block → Verify
3. **Validators vote** → Broadcast votes via All2All
4. **Votes aggregate** → Form certificates (notarization)
5. **Finalization** → Block becomes permanent

The DEBUG messages show steps 1-3 in detail.
The INFO messages show steps 4-5 (the important outcomes).

## Common Patterns

### Normal Operation
- Lots of "dropping duplicate shred" (expected)
- Regular "notarized" and "finalized" messages
- Blocks being produced every few seconds

### Slow Progress
- Fewer "notarized" messages
- More "skipped slot" messages
- Might indicate network issues or slow leader

### Errors to Watch For
- `WARN` messages (yellow) - Usually recoverable issues
- `ERROR` messages (red) - Serious problems

## Understanding the Trade-off

**DEBUG level (`./run.sh`)**:
- ✅ Complete traceability - see every network event
- ✅ Useful for debugging network issues
- ✅ Shows duplicate shred drops (helpful for network analysis)
- ❌ Very noisy (hundreds of messages per second)
- ❌ Hard to see the "big picture" of consensus progress

**INFO level (`./run_info.sh`)**:
- ✅ Clean output - see consensus progress clearly
- ✅ Easy to spot patterns (notarizations, finalizations)
- ✅ Good for understanding the protocol flow
- ❌ Missing network-level details
- ❌ Can't trace individual shred arrivals

**Recommendation**: 
- Start with INFO level to understand consensus flow
- Switch to DEBUG when debugging network issues or investigating problems
- Both are valid - it's about what you're trying to observe

## Tips for Observing

1. **Start with INFO level** to see the big picture:
   ```bash
   ./run_info.sh  # or RUST_LOG="alpenglow=info" cargo run --release --bin=alpenglow
   ```

2. **Watch for patterns:**
   - Are blocks being finalized regularly?
   - Are there many skipped slots?
   - Is the chain making progress?

3. **If you want details**, switch to DEBUG:
   ```bash
   ./run.sh  # Uses DEBUG level for full tracing
   ```

4. **Use grep to filter** while running:
   ```bash
   ./run.sh | grep -E "(INFO|WARN|ERROR|notarized|finalized)"
   ```

5. **For network debugging**, use DEBUG and look for:
   - Duplicate shred patterns (indicates network topology)
   - Missing shreds (might indicate network issues)
   - Vote propagation delays

## Example: Clean Output

Here's what you'd see with INFO level:

```
 INFO notarized(-fallback) block a1b2c3d4 in slot 0
 INFO fast finalized slot 0
 INFO produced block in slot 1
 INFO notarized(-fallback) block e5f6g7h8 in slot 1
 INFO fast finalized slot 1
 INFO produced block in slot 2
 INFO notarized(-fallback) block i9j0k1l2 in slot 2
 INFO fast finalized slot 2
```

Much cleaner! This shows the consensus is working.

## Understanding Slot Numbers

- **Slot 0** = First slot (genesis)
- **Slot 1, 2, 3...** = Subsequent slots
- Each slot should have either:
  - A block (notarized → finalized)
  - A skip (no block produced)

## Next Steps

1. **Try running with INFO level** to see the important events
2. **Watch for the consensus pattern** (produce → notarize → finalize)
3. **Read the Architecture Overview** to understand what's happening
4. **Experiment with different log levels** to see what interests you

The "dropping duplicate shred" messages are just noise - the real action is in the INFO messages!

