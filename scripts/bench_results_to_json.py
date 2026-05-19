#!/usr/bin/env python3
# Copyright (c) Anza Technology, Inc.
# SPDX-License-Identifier: Apache-2.0
"""Parse bench_rotor results.txt into structured JSON for plotting and comparison."""

from __future__ import annotations

import argparse
import json
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def _parse_float(s: str) -> float | None:
    try:
        return float(s)
    except ValueError:
        return None


def parse_results_txt(text: str) -> dict[str, Any]:
    lines = text.splitlines()
    out: dict[str, Any] = {
        "title_strategy": None,
        "offered": None,
        "block_production": None,
        "latency": None,
        "slice_payload": None,
        "relay": None,
    }

    if lines:
        m = re.match(r"^Alpenglow Rotor benchmark summary \(([^)]*)\)\s*$", lines[0].strip())
        if m:
            s = m.group(1).strip()
            out["title_strategy"] = s if s else None

    offered: dict[str, float] = {}
    for line in lines:
        m = re.match(r"^\s+average send rate:\s+([\d.]+)\s+tx/s\s*$", line)
        if m:
            offered["avg_tx_per_s"] = float(m.group(1))
        m = re.match(r"^\s+average data rate:\s+([\d.]+)\s+Mb/s\s*$", line)
        if m:
            offered["avg_mb_per_s"] = float(m.group(1))
    if offered:
        out["offered"] = offered

    per_node_blocks: list[dict[str, Any]] = []
    for line in lines:
        m = re.match(
            r"^\s+node_(\d+):\s+(\d+)\s+blocks\s+\(~([\d.]+)\s+blocks/sec\)\s*$",
            line,
        )
        if m:
            per_node_blocks.append(
                {
                    "node": int(m.group(1)),
                    "blocks": int(m.group(2)),
                    "blocks_per_sec": float(m.group(3)),
                }
            )
        m = re.match(
            r"^\s+AVERAGE BLOCK PRODUCTION:\s+(\d+)\s+blocks per node\s+\(~([\d.]+)\s+blocks/sec per node\)\s*$",
            line,
        )
        if m and per_node_blocks:
            out["block_production"] = {
                "per_node": per_node_blocks,
                "average_blocks_per_node": int(m.group(1)),
                "average_blocks_per_sec_per_node": float(m.group(2)),
            }

    per_node_lat: list[dict[str, Any]] = []
    avg_lat: float | None = None
    for line in lines:
        m = re.match(
            r"^\s+node_(\d+):\s+average fast-finalization latency:\s+([\d.]+|n/a)\s+ms\s*$",
            line,
        )
        if m:
            v = m.group(2)
            per_node_lat.append(
                {
                    "node": int(m.group(1)),
                    "avg_fast_final_ms": None if v == "n/a" else float(v),
                }
            )
        m = re.match(
            r"^\s+AVERAGE FAST-FINALIZATION LATENCY:\s+([\d.]+|n/a)\s+ms\s*$",
            line,
        )
        if m:
            v = m.group(1)
            avg_lat = None if v == "n/a" else float(v)
    if per_node_lat or avg_lat is not None:
        out["latency"] = {"per_node": per_node_lat, "average_fast_final_ms": avg_lat}

    slice_info: dict[str, Any] = {}
    for line in lines:
        m = re.match(r"^\s+slices built:\s+(\d+)\s*$", line)
        if m:
            slice_info["slices_built"] = int(m.group(1))
        m = re.match(r"^\s+minimum transactions in a slice:\s+(\d+)\s*$", line)
        if m:
            slice_info["min_transactions"] = int(m.group(1))
        m = re.match(r"^\s+maximum transactions in a slice:\s+(\d+)\s*$", line)
        if m:
            slice_info["max_transactions"] = int(m.group(1))
        m = re.match(r"^\s+AVERAGE TRANSACTIONS PER SLICE:\s+([\d.]+)\s*$", line)
        if m:
            slice_info["avg_transactions_per_slice"] = float(m.group(1))
    if slice_info:
        out["slice_payload"] = slice_info

    relay_rows: list[dict[str, Any]] = []
    in_relay_table = False
    for line in lines:
        if "Rotor relay throughput" in line:
            continue
        if re.match(r"^\s+NODE\s+LEADER_TX", line):
            in_relay_table = True
            continue
        if in_relay_table:
            m = re.match(
                r"^\s+node_(\d+)\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s*$",
                line,
            )
            if not m:
                continue

            def cell(x: str) -> int | float | None:
                if x == "-":
                    return None
                if "." in x:
                    return _parse_float(x)
                try:
                    return int(x)
                except ValueError:
                    return _parse_float(x)

            relay_rows.append(
                {
                    "node": int(m.group(1)),
                    "leader_tx": cell(m.group(2)),
                    "relay_tx": cell(m.group(3)),
                    "total_tx": cell(m.group(4)),
                    "shreds_per_sec": cell(m.group(5)),
                }
            )
    if relay_rows:
        out["relay"] = {"per_node": relay_rows}

    return out


def build_bench_document(
    results_txt_path: Path,
    duration_secs: float,
    target_tps: float,
    tx_size_bytes: int,
    *,
    strategy: str = "",
    configuration_id: str = "",
) -> dict[str, Any]:
    """Load results.txt and return the same dict that is written as JSON."""
    text = results_txt_path.read_text(encoding="utf-8", errors="replace")
    parsed = parse_results_txt(text)
    strategy_resolved = strategy.strip() or parsed.get("title_strategy") or "unknown"

    meta: dict[str, Any] = {
        "strategy": strategy_resolved,
        "duration_secs": duration_secs,
        "target_tps": target_tps,
        "tx_size_bytes": tx_size_bytes,
        "results_txt": str(results_txt_path.resolve()),
        "generated_at": datetime.now(timezone.utc).isoformat(),
    }
    cfg = configuration_id.strip()
    if cfg:
        meta["configuration_id"] = cfg

    return {**{"meta": meta}, **parsed}


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--results", type=Path, required=True, help="Path to results.txt")
    p.add_argument("--out", type=Path, required=True, help="Output JSON path")
    p.add_argument("--duration", type=float, required=True)
    p.add_argument("--target-tps", type=float, required=True)
    p.add_argument("--tx-size", type=int, required=True)
    p.add_argument("--strategy", default="", help="Override strategy label in meta")
    p.add_argument(
        "--configuration-id",
        default="",
        help="Optional label for cluster/stake setup (for comparing runs later)",
    )
    args = p.parse_args()

    doc = build_bench_document(
        args.results,
        args.duration,
        args.target_tps,
        args.tx_size,
        strategy=args.strategy,
        configuration_id=args.configuration_id,
    )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {args.out}")


if __name__ == "__main__":
    main()
