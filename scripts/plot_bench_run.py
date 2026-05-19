#!/usr/bin/env python3
# Copyright (c) Anza Technology, Inc.
# SPDX-License-Identifier: Apache-2.0
"""Level 1: plot metrics from a single bench_rotor run.

Accepts results.txt (parses at startup) or results.json. For .txt, pass the same
--duration / --target-tps / --tx-size you used for bench_rotor.sh (defaults match the script).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib import gridspec

_SCRIPT_DIR = Path(__file__).resolve().parent
if str(_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPT_DIR))

from bench_results_to_json import build_bench_document  # noqa: E402


def load_doc_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "results",
        type=Path,
        help="Path to results.txt or results.json",
    )
    p.add_argument(
        "--duration",
        type=float,
        default=60.0,
        help="Benchmark duration in seconds (for results.txt only; must match bench_rotor)",
    )
    p.add_argument(
        "--target-tps",
        type=float,
        default=200.0,
        help="Target TPS (for results.txt only)",
    )
    p.add_argument(
        "--tx-size",
        type=int,
        default=512,
        help="Transaction size in bytes (for results.txt only)",
    )
    p.add_argument(
        "--strategy",
        default="",
        help="Override strategy in meta (for results.txt); default from results title",
    )
    p.add_argument(
        "--configuration-id",
        default="",
        help="Optional configuration label stored in meta (for results.txt)",
    )
    p.add_argument(
        "--json-out",
        type=Path,
        default=None,
        help="If set, write parsed document as JSON to this path (useful for level-2 plots)",
    )
    p.add_argument(
        "--out-dir",
        type=Path,
        default=None,
        help="Directory for PNG (default: same directory as results file)",
    )
    p.add_argument(
        "--out-name",
        default="bench_run_plots.png",
        help="Output filename (single figure with subplots)",
    )
    p.add_argument("--dpi", type=int, default=150)
    args = p.parse_args()

    results_path = args.results
    suffix = results_path.suffix.lower()
    if suffix == ".json":
        doc = load_doc_json(results_path)
    elif suffix == ".txt":
        doc = build_bench_document(
            results_path,
            args.duration,
            args.target_tps,
            args.tx_size,
            strategy=args.strategy,
            configuration_id=args.configuration_id,
        )
        if args.json_out is not None:
            args.json_out.parent.mkdir(parents=True, exist_ok=True)
            args.json_out.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
            print(f"Wrote {args.json_out}")
    else:
        p.error(f"expected .txt or .json, got {results_path.suffix!r}")

    meta = doc.get("meta") or {}
    strategy = meta.get("strategy", "unknown")
    cfg = meta.get("configuration_id")
    title = f"Rotor benchmark — {strategy}"
    if cfg:
        title += f" ({cfg})"

    out_dir = args.out_dir or results_path.parent
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / args.out_name

    fig = plt.figure(figsize=(11, 12))
    fig.suptitle(title, fontsize=14, fontweight="bold", y=0.995)
    gs = gridspec.GridSpec(3, 2, figure=fig, height_ratios=[1.0, 1.0, 0.85], hspace=0.35, wspace=0.28)

    # --- (0,0) KPIs ---
    ax = fig.add_subplot(gs[0, 0])
    ax.axis("off")
    lines = [f"Strategy: {strategy}"]
    if cfg:
        lines.append(f"Configuration: {cfg}")
    if meta:
        lines.append(
            f"Duration: {meta.get('duration_secs')} s | Target TPS: {meta.get('target_tps')} | "
            f"Tx size: {meta.get('tx_size_bytes')} B"
        )
    offered = doc.get("offered")
    if offered:
        lines.append(
            f"Avg offered load: {offered.get('avg_tx_per_s')} tx/s, {offered.get('avg_mb_per_s')} Mb/s"
        )
    else:
        lines.append("Offered load: n/a")
    lat = doc.get("latency") or {}
    if lat.get("average_fast_final_ms") is not None:
        lines.append(f"Avg fast-final latency: {lat['average_fast_final_ms']:.2f} ms")
    else:
        lines.append("Avg fast-final latency: n/a")
    bp = doc.get("block_production") or {}
    if bp.get("average_blocks_per_sec_per_node") is not None:
        lines.append(
            f"Avg block production: {bp.get('average_blocks_per_node')} blocks/node, "
            f"{bp['average_blocks_per_sec_per_node']:.2f} blocks/s/node"
        )
    sl = doc.get("slice_payload")
    if sl:
        lines.append(
            f"Slices: {sl.get('slices_built')} | txs/slice min {sl.get('min_transactions')} "
            f"max {sl.get('max_transactions')} avg {sl.get('avg_transactions_per_slice')}"
        )
    ax.text(0.02, 0.98, "\n".join(lines), transform=ax.transAxes, va="top", fontsize=9, family="monospace")

    # --- (0,1) block production ---
    ax = fig.add_subplot(gs[0, 1])
    per_b = (doc.get("block_production") or {}).get("per_node") or []
    if per_b:
        nodes = [f"n{d['node']}" for d in per_b]
        rates = [d["blocks_per_sec"] for d in per_b]
        ax.bar(nodes, rates, color="steelblue")
        ax.set_ylabel("blocks / sec")
        ax.set_title("Block production (leader slots)")
    else:
        ax.text(0.5, 0.5, "No block data", ha="center", va="center", transform=ax.transAxes)

    # --- (1,0) latency ---
    ax = fig.add_subplot(gs[1, 0])
    per_l = (doc.get("latency") or {}).get("per_node") or []
    if per_l:
        nodes = [f"n{d['node']}" for d in per_l]
        ms = [d["avg_fast_final_ms"] for d in per_l if d.get("avg_fast_final_ms") is not None]
        if len(ms) == len(per_l):
            ax.bar(nodes, ms, color="darkorange")
            avg = lat.get("average_fast_final_ms")
            if avg is not None:
                ax.axhline(avg, color="red", linestyle="--", label=f"Avg {avg:.1f} ms")
                ax.legend(fontsize=8)
            ax.set_ylabel("ms")
            ax.set_title("Fast-final latency (avg per node)")
        else:
            ax.text(0.5, 0.5, "Incomplete latency", ha="center", va="center", transform=ax.transAxes)
    else:
        ax.text(0.5, 0.5, "No latency data", ha="center", va="center", transform=ax.transAxes)

    # --- (1,1) relay stacked counts ---
    ax = fig.add_subplot(gs[1, 1])
    relay = (doc.get("relay") or {}).get("per_node") or []
    if relay:
        nodes = [f"n{r['node']}" for r in relay]
        leader = [float(r["leader_tx"] or 0) for r in relay]
        relay_tx = [float(r["relay_tx"] or 0) for r in relay]
        x = range(len(nodes))
        ax.bar(x, leader, label="leader_tx (count)", color="seagreen")
        ax.bar(x, relay_tx, bottom=leader, label="relay_tx (count)", color="mediumpurple")
        ax.set_xticks(list(x))
        ax.set_xticklabels(nodes)
        ax.set_ylabel("shred tx count (run total)")
        ax.set_title("Rotor relay volume (total over run)")
        ax.legend(fontsize=8)
    else:
        ax.text(0.5, 0.5, "No relay data", ha="center", va="center", transform=ax.transAxes)

    # --- (2,0) slice payload min / avg / max ---
    ax = fig.add_subplot(gs[2, 0])
    sl = doc.get("slice_payload")
    if sl and sl.get("slices_built"):
        mn = sl.get("min_transactions")
        mx = sl.get("max_transactions")
        av = sl.get("avg_transactions_per_slice")
        if mn is not None and mx is not None and av is not None:
            ax.bar(
                ["min", "avg", "max"],
                [float(mn), float(av), float(mx)],
                color=["#6baed6", "#3182bd", "#08519c"],
            )
            ax.set_ylabel("transactions per slice")
            ax.set_title("Slice payload (node_0)")
        else:
            ax.text(0.5, 0.5, "Incomplete slice stats", ha="center", va="center", transform=ax.transAxes)
    else:
        ax.text(0.5, 0.5, "No slice payload data", ha="center", va="center", transform=ax.transAxes)

    # --- (2,1) shreds/sec per node ---
    ax = fig.add_subplot(gs[2, 1])
    if relay:
        nodes = [f"n{r['node']}" for r in relay]
        rates = [float(r["shreds_per_sec"] or 0) for r in relay]
        ax.bar(nodes, rates, color="coral")
        ax.set_ylabel("shreds / sec")
        ax.set_title("Relay rate by node")
    else:
        ax.text(0.5, 0.5, "No relay data", ha="center", va="center", transform=ax.transAxes)

    fig.subplots_adjust(top=0.93)
    fig.savefig(out_path, dpi=args.dpi, bbox_inches="tight")
    plt.close(fig)
    print(f"Wrote {out_path}")


if __name__ == "__main__":
    main()
