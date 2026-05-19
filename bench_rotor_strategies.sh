#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${ROOT}"

# bench_rotor_strategies.sh — sweep multiple Rotor samplers without editing src/bin/node.rs
#
# WHAT EACH ITERATION DOES:
#   For each strategy short name you pass on the command line:
#     1. Sets BENCH_NODE_FEATURES=sampler_<name> (see short_to_feature mapping below).
#     2. Runs ./bench_rotor.sh end-to-end:
#          release rebuild of `node` (and workload_generator) with that feature set,
#          regenerates configs from ip_list, starts all nodes fresh, runs workload, stops cluster,
#          writes/overwrites LOG_DIR/results.txt for this strategy’s run.
#     3. If python3 is available and SKIP_JSON is unset: plot_bench_run.py writes
#          JSON_RUNS_DIR/<short>.json and bench_run_<short>.png under bench_logs/.
#   After all strategies: if at least two JSON files were produced, runs plot_strategy_comparison.py
#   unless SKIP_COMPARE=1.
#
# WHY FULL RESTART + REBUILD EACH TIME:
#   The sampler type is a Rust generic on Rotor; switching strategies requires a different binary.
#   Pros: matches current architecture, no runtime dispatch refactor, reproducible per-strategy logs.
#   Cons: cargo release build per strategy is often the slow part; cluster boot + bench duration
#   repeats every time. For many strategies, total wall time adds up.
#
# Usage:
#   ./bench_rotor_strategies.sh uniform turbine stake_weighted
#
# Short names (CLI argument → Cargo feature passed as BENCH_NODE_FEATURES):
#   uniform  stake_weighted  decaying  turbine  partition
#   fa1_partition  fa1_stake_weighted  fa2  all_same
#
# Env (optional):
#   BR_DURATION   BR_TPS   BR_TX     — forwarded to bench_rotor.sh (defaults 60 200 512)
#   JSON_RUNS_DIR — per-strategy JSON + inputs for level-2 (default: …/bench_logs/runs)
#   BENCH_CONFIGURATION_ID — passed through to plot_bench_run.py --configuration-id for comparisons
#   SKIP_COMPARE=1 — skip plot_strategy_comparison.py after the loop
#   SKIP_JSON=1 — skip plot_bench_run.py (no JSON/PNG per strategy; comparison step skipped)
#
# Example with env overrides:
#   BR_DURATION=45 BR_TPS=300 BR_TX=512 BENCH_CONFIGURATION_ID=4nodes_even_stake #   JSON_RUNS_DIR=src/disseminator/rotor/bench_logs/runs_demo ./bench_rotor_strategies.sh #   uniform turbine stake_weighted fa2

BR_DURATION="${BR_DURATION:-60}"
BR_TPS="${BR_TPS:-200}"
BR_TX="${BR_TX:-512}"
JSON_RUNS_DIR="${JSON_RUNS_DIR:-src/disseminator/rotor/bench_logs/runs}"
LOG_DIR="src/disseminator/rotor/bench_logs"
RESULTS="${LOG_DIR}/results.txt"

# CLI short name (e.g. uniform) → full Cargo feature flag for cargo --features.
short_to_feature() {
  case "$1" in
    uniform) echo "sampler_uniform" ;;
    stake_weighted) echo "sampler_stake_weighted" ;;
    decaying) echo "sampler_decaying" ;;
    turbine) echo "sampler_turbine" ;;
    partition) echo "sampler_partition" ;;
    fa1_partition) echo "sampler_fa1_partition" ;;
    fa1_stake_weighted) echo "sampler_fa1_stake_weighted" ;;
    fa2) echo "sampler_fa2" ;;
    all_same) echo "sampler_all_same" ;;
    *)
      echo ""
      ;;
  esac
}

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <strategy_short> [<strategy_short> ...]" >&2
  echo "  strategies: uniform stake_weighted decaying turbine partition fa1_partition fa1_stake_weighted fa2 all_same" >&2
  exit 1
fi

STRATEGIES=("$@")
for s in "${STRATEGIES[@]}"; do
  feat="$(short_to_feature "${s}")"
  if [[ -z "${feat}" ]]; then
    echo "error: unknown strategy short name '${s}'" >&2
    exit 1
  fi
done

mkdir -p "${JSON_RUNS_DIR}"

json_paths=()
for s in "${STRATEGIES[@]}"; do
  feat="$(short_to_feature "${s}")"
  echo ""
  echo "========== bench: ${s} (${feat}) =========="
  export BENCH_NODE_FEATURES="${feat}"
  ./bench_rotor.sh "${BR_DURATION}" "${BR_TPS}" "${BR_TX}"

  if [[ "${SKIP_JSON:-0}" != "1" ]]; then
    json_out="${JSON_RUNS_DIR}/${s}.json"
    json_extra=()
    if [[ -n "${BENCH_CONFIGURATION_ID:-}" ]]; then
      json_extra+=(--configuration-id "${BENCH_CONFIGURATION_ID}")
    fi
    if command -v python3 >/dev/null 2>&1; then
      if [[ ${#json_extra[@]} -eq 0 ]]; then
        python3 "${ROOT}/scripts/plot_bench_run.py" "${RESULTS}" \
          --duration "${BR_DURATION}" --target-tps "${BR_TPS}" --tx-size "${BR_TX}" \
          --json-out "${json_out}" \
          --out-name "bench_run_${s}.png"
      else
        python3 "${ROOT}/scripts/plot_bench_run.py" "${RESULTS}" \
          --duration "${BR_DURATION}" --target-tps "${BR_TPS}" --tx-size "${BR_TX}" \
          --json-out "${json_out}" \
          "${json_extra[@]}" \
          --out-name "bench_run_${s}.png"
      fi
      json_paths+=("${json_out}")
      echo "  wrote ${json_out}"
    else
      echo "  (python3 missing; skipped JSON and plot for ${s})" >&2
    fi
  fi
done

if [[ "${SKIP_JSON:-0}" != "1" && "${SKIP_COMPARE:-0}" != "1" && ${#json_paths[@]} -ge 2 ]] && command -v python3 >/dev/null 2>&1; then
  compare_out="${LOG_DIR}/strategy_comparison.png"
  python3 "${ROOT}/scripts/plot_strategy_comparison.py" "${json_paths[@]}" --out "${compare_out}"
  echo ""
  echo "Level-2 comparison: ${compare_out}"
fi

unset BENCH_NODE_FEATURES
