#!/usr/bin/env python3
# Copyright (c) Anza Technology, Inc.
# SPDX-License-Identifier: Apache-2.0
"""Level 2: compare multiple bench_rotor runs (JSON) under the same configuration.

Typical workflow (same bench duration / TPS / tx size for every run):

  mkdir -p src/disseminator/rotor/bench_logs/runs
  # After each ./bench_rotor.sh with a different RotorSampler in node.rs:
  python3 scripts/plot_bench_run.py src/disseminator/rotor/bench_logs/results.txt \\
    --duration 60 --target-tps 200 --tx-size 512 \\
    --json-out src/disseminator/rotor/bench_logs/runs/uniform.json \\

  python3 scripts/plot_strategy_comparison.py \\
    --glob 'src/disseminator/rotor/bench_logs/runs/*.json' \\
    --out src/disseminator/rotor/bench_logs/strategy_comparison.png

Use single quotes around the glob on zsh so empty dirs do not error.
"""

from __future__ import annotations

import argparse
import glob
import json
import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt


def load_doc(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def strategy_label(doc: dict) -> str:
    return (doc.get("meta") or {}).get("strategy") or "unknown"


def short_strategy_label(label: str) -> str:
    """Compact display labels for crowded x-axes."""
    mapping = {
        "UniformSampler": "Uniform",
        "StakeWeightedSampler": "StakeWeighted",
        "DecayingAcceptanceSampler": "Decaying",
        "TurbineSampler": "Turbine",
        "PartitionSampler": "Partition",
        "FaitAccompli1Sampler_PartitionSampler": "FA1-Partition",
        "FaitAccompli1Sampler_StakeWeightedSampler": "FA1-StakeWeighted",
        "FaitAccompli2Sampler": "FA2",
        "AllSameSampler": "AllSame",
    }
    return mapping.get(label, label)


def configuration_label(doc: dict) -> str | None:
    cid = (doc.get("meta") or {}).get("configuration_id")
    return str(cid).strip() if cid else None


def extract_metrics(doc: dict) -> dict:
    lat = (doc.get("latency") or {}).get("average_fast_final_ms")
    bp = doc.get("block_production") or {}
    b_rate = bp.get("average_blocks_per_sec_per_node")
    offered = doc.get("offered") or {}
    sl = doc.get("slice_payload") or {}
    relay_rows = (doc.get("relay") or {}).get("per_node") or []
    sum_shreds_s = 0.0
    max_shreds_s = 0.0
    for r in relay_rows:
        v = r.get("shreds_per_sec")
        if v is not None:
            sum_shreds_s += float(v)
            max_shreds_s = max(max_shreds_s, float(v))
    return {
        "avg_latency_ms": lat,
        "blocks_per_sec_per_node": b_rate,
        "offered_tx_s": offered.get("avg_tx_per_s"),
        "avg_tx_per_slice": sl.get("avg_transactions_per_slice"),
        "sum_shreds_per_sec": sum_shreds_s if relay_rows else None,
        "max_shreds_per_sec": max_shreds_s if relay_rows else None,
    }


def warn_if_mismatched_meta(docs: list[dict], paths: list[Path]) -> None:
    """Print stderr warnings when meta fields differ across runs (fair comparison assumes match)."""
    keys = ("duration_secs", "target_tps", "tx_size_bytes", "configuration_id")

    def meta_val(d: dict, k: str):
        return (d.get("meta") or {}).get(k)

    for k in keys:
        vals = {meta_val(d, k) for d in docs}
        vals.discard(None)
        if len(vals) > 1:
            detail = ", ".join(f"{p.name}:{meta_val(d, k)!r}" for d, p in zip(docs, paths))
            print(f"warning: meta[{k!r}] differs across files — {detail}", file=sys.stderr)


def main() -> None:
    p = argparse.ArgumentParser(
        description=__doc__.split("\n\nTypical workflow")[0].strip(),
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__.split("Typical workflow", 1)[1].strip(),
    )
    p.add_argument(
        "results_json",
        type=Path,
        nargs="*",
        default=[],
        help="One or more JSON files (from plot_bench_run.py --json-out or bench_results_to_json.py)",
    )
    p.add_argument(
        "--glob",
        dest="glob_pattern",
        default="",
        help="Glob for JSON files (quote on zsh: 'path/*.json')",
    )
    p.add_argument(
        "--out",
        type=Path,
        default=Path("strategy_comparison.png"),
        help="Output PNG path",
    )
    p.add_argument(
        "--title",
        default="",
        help="Figure title (default: auto from configuration_id if all runs match)",
    )
    p.add_argument("--dpi", type=int, default=150)
    args = p.parse_args()

    paths: list[Path] = list(args.results_json)
    if args.glob_pattern:
        paths.extend(Path(s) for s in sorted(glob.glob(args.glob_pattern)))
    if not paths:
        p.error("pass at least one .json path or use --glob (see --help)")

    pairs = [(p, load_doc(p)) for p in paths]
    pairs.sort(key=lambda t: (strategy_label(t[1]), t[0].name))
    paths = [t[0] for t in pairs]
    docs = [t[1] for t in pairs]

    warn_if_mismatched_meta(docs, paths)

    labels_full = [strategy_label(d) for d in docs]
    labels = [short_strategy_label(s) for s in labels_full]
    if len(labels) != len(set(labels)):
        labels = [
            f"{short_strategy_label(strategy_label(d))}\n({path.stem})"
            for d, path in zip(docs, paths)
        ]

    configs = [configuration_label(d) for d in docs]
    unique_cfg = {c for c in configs if c}
    fig_title = args.title.strip()
    if not fig_title:
        if len(unique_cfg) == 1:
            fig_title = f"Strategy comparison — config: {next(iter(unique_cfg))}"
        else:
            fig_title = "Strategy comparison"

    metrics = [extract_metrics(d) for d in docs]
    n = len(labels)
    x = list(range(n))
    colors = [plt.cm.tab10(i % 10) for i in range(n)]

    if n < 2:
        print("warning: comparing a single run; add more JSON files for a cross-strategy chart", file=sys.stderr)

    fig, axes = plt.subplots(3, 2, figsize=(12, 13))
    fig.suptitle(fig_title, fontsize=14, fontweight="bold", y=0.995)

    def bar_panel(ax, values, ylabel: str, title: str, fmt: str) -> None:
        ax.bar(x, [v if v is not None else 0 for v in values], color=colors)
        ax.set_xticks(x)
        ax.set_xticklabels(labels, rotation=18, ha="right", fontsize=9)
        ax.set_ylabel(ylabel)
        ax.set_title(title)
        for i, v in enumerate(values):
            if v is not None:
                ax.text(i, v, fmt.format(v), ha="center", va="bottom", fontsize=7)

    # Row 0: latency, block rate
    lat_vals = [m["avg_latency_ms"] for m in metrics]
    bar_colors = [colors[i] if v is not None else "#cccccc" for i, v in enumerate(lat_vals)]
    axes[0, 0].bar(x, [v if v is not None else 0 for v in lat_vals], color=bar_colors)
    axes[0, 0].set_xticks(x)
    axes[0, 0].set_xticklabels(labels, rotation=18, ha="right", fontsize=9)
    axes[0, 0].set_ylabel("ms")
    axes[0, 0].set_title("Average fast-finalization latency")
    for i, v in enumerate(lat_vals):
        if v is not None:
            axes[0, 0].text(i, v, f"{v:.1f}", ha="center", va="bottom", fontsize=7)

    br = [m["blocks_per_sec_per_node"] for m in metrics]
    bar_panel(axes[0, 1], br, "blocks/s/node", "Average block production (per node)", "{:.2f}")

    ot = [m["offered_tx_s"] for m in metrics]
    bar_panel(axes[1, 0], ot, "tx/s", "Offered load (avg tx/s to node 0)", "{:.1f}")

    rs = [m["sum_shreds_per_sec"] for m in metrics]
    bar_panel(axes[1, 1], rs, "sum of node shreds/s", "Aggregate relay rate", "{:.1f}")

    ats = [m["avg_tx_per_slice"] for m in metrics]
    bar_panel(axes[2, 0], ats, "tx/slice", "Avg transactions per slice (node_0)", "{:.2f}")

    mxr = [m["max_shreds_per_sec"] for m in metrics]
    bar_panel(axes[2, 1], mxr, "shreds/s", "Peak per-node relay rate", "{:.1f}")

    fig.subplots_adjust(top=0.94)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(args.out, dpi=args.dpi, bbox_inches="tight")
    plt.close(fig)
    print(f"Wrote {args.out}")


if __name__ == "__main__":
    main()
