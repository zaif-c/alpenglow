#!/usr/bin/env bash
set -euo pipefail

# Simple local benchmark harness for Rotor sampling strategies.
# Runs a multi-node cluster, sends transactions, and records basic metrics.
#
# Usage:
#   ./bench_rotor.sh [duration_secs] [tps] [tx_size_bytes]
# Example:
#   ./bench_rotor.sh 60 200 512
#
# Assumptions:
# - You have set up ip_list file with the addresses of the nodes to be used in the benchmark
# - The desired RotorSampler is set in src/bin/node.rs and the code is built in --release.

DURATION="${1:-60}"
TPS="${2:-200}"
TX_SIZE="${3:-512}"
NUM_NODES=4
LOG_DIR="src/disseminator/rotor/bench_logs"
echo Logs will be written to ${LOG_DIR}/
mkdir -p "${LOG_DIR}"

echo "Benchmarking Rotor for ${DURATION}s at ~${TPS} TPS, tx size ${TX_SIZE} bytes"
echo "Logs will be written to ${LOG_DIR}/"

echo "Generating node config files from ip_list..."
cargo run --release --bin node -- --generate-config-files ip_list --config-name ag_node >/dev/null

echo "Building binaries..."
cargo build --release --bin node --bin workload_generator >/dev/null

node_pids=()

start_node() {
  local idx="$1"
  local cfg="ag_node_${idx}.toml"
  local log_file="${LOG_DIR}/node_${idx}.log"
  echo "Starting node ${idx} with config ${cfg} (log: ${log_file})"
  RUST_LOG=info target/release/node --config-name "${cfg}" \
    >"${log_file}" 2>&1 &
  node_pids+=("$!")
}

echo "Starting ${NUM_NODES} nodes..."
for i in $(seq 0 $((NUM_NODES - 1))); do
  start_node "${i}"
done

echo "Waiting 5s for nodes to boot..."
sleep 5

# Derive tx receiver port for node 0: base port + 4 (see create_node in node.rs)
BASE_PORT_0=$(grep '^port' ag_node_0.toml | head -1 | awk -F'= ' '{print $2}' | tr -d ' ')
TX_PORT_0=$((BASE_PORT_0 + 4))
VALIDATOR_ADDR="127.0.0.1:${TX_PORT_0}"

echo "Sending transactions to ${VALIDATOR_ADDR} using workload_generator..."
WG_LOG="${LOG_DIR}/workload_generator.log"
RUST_LOG=info target/release/workload_generator \
  --validator "${VALIDATOR_ADDR}" \
  --transactions-per-second "${TPS}" \
  --transaction-size "${TX_SIZE}" \
  --initial-delay-secs 5 \
  >"${WG_LOG}" 2>&1 &
WG_PID=$!

echo "Running benchmark for ${DURATION}s..."
sleep "${DURATION}"

echo "Stopping workload generator and nodes..."
kill "${WG_PID}" 2>/dev/null || true
for pid in "${node_pids[@]}"; do
  kill "${pid}" 2>/dev/null || true
done

wait || true

echo "Computing benchmark metrics..."
RESULTS="${LOG_DIR}/results.txt"
rm -f "${RESULTS}"

echo "Alpenglow Rotor benchmark summary" | tee -a "${RESULTS}"
echo "" | tee -a "${RESULTS}"

# ---------------------------------------------------------------------------
# Section 1: Offered load / throughput (what we asked the generator to send).
# We compute the average achieved TPS (tx/s) and data rate (Mb/s) from the
# workload generator to node_0 over the benchmark duration.
# ---------------------------------------------------------------------------
echo "1) Offered transaction throughput (workload generator → node_0 tx port)" | tee -a "${RESULTS}"
read avg_tps avg_mbs <<<"$(awk '
  /sent / {
    count++;
    tps = 0; mbs = 0;
    for (i = 1; i <= NF; i++) {
      if ($i ~ /TPS:/) {
        val = $(i + 1);
        gsub(/[^0-9.]/, "", val);
        tps = val + 0;
      }
      if ($i ~ /Mb\/s\)/) {
        val = $(i - 1);
        gsub(/[^0-9.]/, "", val);
        mbs = val + 0;
      }
    }
    sum_tps += tps;
    sum_mbs += mbs;
  }
  END {
    if (count > 0) {
      printf "%.2f %.2f\n", sum_tps / count, sum_mbs / count;
    } else {
      print "n/a n/a";
    }
  }
' "${WG_LOG}")"

if [[ "${avg_tps}" != "n/a" ]]; then
  echo "   average send rate: ${avg_tps} tx/s" | tee -a "${RESULTS}"
  echo "   average data rate: ${avg_mbs} Mb/s" | tee -a "${RESULTS}"
else
  echo "   No workload generator stats found" | tee -a "${RESULTS}"
fi

# ---------------------------------------------------------------------------
# Section 2: Block production rate per node.
# This counts how many times each node logged \"producing block in slot\"
# during the benchmark window and converts it to blocks per second.
# Only leaders for a slot log production, so this reflects leader activity.
# ---------------------------------------------------------------------------
echo "" | tee -a "${RESULTS}"
echo "2) Block production rate per node (leader slots only)" | tee -a "${RESULTS}"
total_blocks=0
for i in $(seq 0 $((NUM_NODES - 1))); do
  log_file="${LOG_DIR}/node_${i}.log"
  if [[ -f "${log_file}" ]]; then
    blocks=$(grep -c "producing block in slot" "${log_file}" || true)
    rate=$(awk -v b="${blocks}" -v d="${DURATION}" 'BEGIN{ if (d>0) printf "%.2f", b/d; else print "0.00"; }')
    echo "   node_${i}: ${blocks} blocks (~${rate} blocks/sec)" | tee -a "${RESULTS}"
    total_blocks=$((total_blocks + blocks))
  else
    echo "   node_${i}: log not found" | tee -a "${RESULTS}"
  fi
done

if [[ "${NUM_NODES}" -gt 0 ]]; then
  avg_blocks_per_node=$((total_blocks / NUM_NODES))
  avg_rate=$(awk -v b="${total_blocks}" -v n="${NUM_NODES}" -v d="${DURATION}" 'BEGIN{
    if (d>0 && n>0) printf "%.2f", (b / n) / d; else print "0.00";
  }')
  echo "   AVERAGE BLOCK PRODUCTION: ${avg_blocks_per_node} blocks per node (~${avg_rate} blocks/sec per node)" | tee -a "${RESULTS}"
fi

# ---------------------------------------------------------------------------
# Section 3: End-to-end latency (block production → fast finalization).
# For each node we log:
#   - METRIC_BLOCK_PRODUCED slot=S ts_ms=...
#   - METRIC_FAST_FINAL     slot=S ts_ms=...
# Latency per slot is (fast_final_ts_ms - produced_ts_ms). We report the
# average over all slots observed in this run, in milliseconds, per node,
# plus an overall average across nodes.
# ---------------------------------------------------------------------------
echo "" | tee -a "${RESULTS}"
echo "3) End-to-end finalization latency (block production → fast finalization)" | tee -a "${RESULTS}"

overall_sum=0
overall_count=0

for i in $(seq 0 $((NUM_NODES - 1))); do
  LOG_FILE="${LOG_DIR}/node_${i}.log"
  if [[ -f "${LOG_FILE}" ]]; then
    avg_fast=$(awk '
      /METRIC_BLOCK_PRODUCED slot=/ {
        slot = ""; ts = "";
        for (i = 1; i <= NF; i++) {
          if ($i ~ /^slot=/)   { split($i, a, "="); slot = a[2]; }
          if ($i ~ /^ts_ms=/)  { split($i, b, "="); ts   = b[2] + 0; }
        }
        if (slot != "" && ts != "") produced[slot] = ts;
      }
      /METRIC_FAST_FINAL slot=/ {
        slot = ""; ts = "";
        for (i = 1; i <= NF; i++) {
          if ($i ~ /^slot=/)   { split($i, a, "="); slot = a[2]; }
          if ($i ~ /^ts_ms=/)  { split($i, b, "="); ts   = b[2] + 0; }
        }
        if (slot != "" && ts != "") final[slot] = ts;
      }
      END {
        count = 0; sum = 0;
        for (s in produced) {
          if (s in final) {
            dt = final[s] - produced[s]; # ms
            sum += dt;
            count++;
          }
        }
        if (count > 0) {
          printf "%.2f\n", sum / count; # average latency in ms
        } else {
          print "n/a";
        }
      }' "${LOG_FILE}")

    echo "   node_${i}: average fast-finalization latency: ${avg_fast} ms" | tee -a "${RESULTS}"

    if [[ "${avg_fast}" != "n/a" ]]; then
      overall_sum=$(awk -v a="${overall_sum}" -v b="${avg_fast}" 'BEGIN{printf "%.6f", a + b}')
      overall_count=$((overall_count + 1))
    fi
  else
    echo "   node_${i}: log not found, skipping latency computation" | tee -a "${RESULTS}"
  fi
done

if [[ "${overall_count}" -gt 0 ]]; then
  overall_avg=$(awk -v s="${overall_sum}" -v c="${overall_count}" 'BEGIN{printf "%.2f", s / c}')
  echo "   AVERAGE FAST-FINALIZATION LATENCY: ${overall_avg} ms" | tee -a "${RESULTS}"
else
  echo "   overall average fast-finalization latency: n/a" | tee -a "${RESULTS}"
fi

# ---------------------------------------------------------------------------
# Section 4: Slice payload statistics (how many transactions per slice).
# For node_0 we log METRIC_SLICE_PAYLOAD txs=N every time it builds a slice.
# From that we compute how many slices were built and the average / min / max
# transactions per slice. This, together with the offered TPS, helps you
# reason about how full the slices/blocks are under this sampling strategy.
# ---------------------------------------------------------------------------
echo "" | tee -a "${RESULTS}"
echo "4) Slice payload statistics (transactions per slice, node_0)" | tee -a "${RESULTS}"
NODE0_LOG="${LOG_DIR}/node_0.log"
if [[ -f "${NODE0_LOG}" ]]; then
  awk '
    /METRIC_SLICE_PAYLOAD txs=/ {
      for (i = 1; i <= NF; i++) {
        if ($i ~ /^txs=/) {
          split($i, a, "=");
          n = a[2] + 0;
          count++;
          sum += n;
          if (n > max) max = n;
          if (n < min || count == 1) min = n;
        }
      }
    }
    END {
      if (count > 0) {
        avg = sum / count;
        printf "   slices built: %d\n", count;
        printf "   minimum transactions in a slice: %d\n", min;
        printf "   maximum transactions in a slice: %d\n", max;
        printf "   AVERAGE TRANSACTIONS PER SLICE: %.2f\n", avg;
      } else {
        print "   no METRIC_SLICE_PAYLOAD entries (no slices built?)";
      }
    }' "${NODE0_LOG}" | tee -a "${RESULTS}"
else
  echo "   node_0 log not found, skipping slice payload metrics" | tee -a "${RESULTS}"
fi

# ---------------------------------------------------------------------------
# Section 5: Rotor relay throughput (how much data each node relays).
# We instrument Rotor to log METRIC_RELAY lines:
#   - leader_shreds=K  when the leader sends shreds to a relay
#   - relay_shreds=K   when a relay forwards a shred to K peers
# Summing these per node and dividing by benchmark duration gives an
# approximate relay throughput in shreds/sec.
# ---------------------------------------------------------------------------
echo "" | tee -a "${RESULTS}"
echo "5) Rotor relay throughput (shreds relayed per second, per node)" | tee -a "${RESULTS}"
printf "   %-8s %-20s %-20s %-20s %-14s\n" "NODE" "LEADER_TX" "RELAY_TX" "TOTAL_TX" "SHREDS/SEC" | tee -a "${RESULTS}"

for i in $(seq 0 $((NUM_NODES - 1))); do
  log_file="${LOG_DIR}/node_${i}.log"
  if [[ -f "${log_file}" ]]; then
    read leader relay total rate <<<"$(awk -v dur="${DURATION}" '
      /METRIC_RELAY/ {
        for (i = 1; i <= NF; i++) {
          if ($i ~ /^leader_shreds=/) {
            split($i, a, "=");
            leader += a[2] + 0;
          }
          if ($i ~ /^relay_shreds=/) {
            split($i, b, "=");
            relay += b[2] + 0;
          }
        }
      }
      END {
        total = leader + relay;
        if (dur > 0) {
          rate = total / dur;
        } else {
          rate = 0.0;
        }
        printf "%d %d %d %.2f\n", leader, relay, total, rate;
      }' "${log_file}")"
    printf "   %-8s %-20d %-20d %-20d %-14.2f\n" "node_${i}" "${leader}" "${relay}" "${total}" "${rate}" | tee -a "${RESULTS}"
  else
    printf "   %-8s %-20s %-20s %-20s %-14s\n" "node_${i}" "-" "-" "-" "-" | tee -a "${RESULTS}"
  fi
done

echo ""
echo "Benchmark complete. See:"
echo "  - ${WG_LOG}          for detailed offered load (per-second TPS / data rate)"
echo "  - ${LOG_DIR}/node_*.log for raw per-node logs"
echo "  - ${RESULTS}         for a human-readable summary of this run"

