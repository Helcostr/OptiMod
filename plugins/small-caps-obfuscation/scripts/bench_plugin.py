#!/usr/bin/env python3
"""Benchmark optimod_check with the release DLL kept loaded in-process."""

from __future__ import annotations

import argparse
import ctypes
import statistics
import time

from _optimod_plugin import load_plugin, spam_fixture_path


def bench_case(
    lib: ctypes.CDLL,
    payload: bytes,
    warmup: int,
    iters: int,
) -> dict[str, float | int]:
    for _ in range(warmup):
        lib.optimod_check(payload, len(payload))

    times_ns: list[int] = []
    for _ in range(iters):
        start = time.perf_counter_ns()
        result = lib.optimod_check(payload, len(payload))
        times_ns.append(time.perf_counter_ns() - start)

    times_ns.sort()
    return {
        "result": result,
        "iters": iters,
        "p50_us": times_ns[len(times_ns) // 2] / 1000,
        "mean_us": statistics.mean(times_ns) / 1000,
        "min_us": times_ns[0] / 1000,
        "max_us": times_ns[-1] / 1000,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--warmup", type=int, default=50)
    parser.add_argument("--iters", type=int, default=500)
    args = parser.parse_args()

    lib, load_ms = load_plugin(timed=True)
    payload = spam_fixture_path().read_text(encoding="utf-8").strip().encode("utf-8")

    stats = bench_case(lib, payload, args.warmup, args.iters)
    print(f"load_ms={load_ms:.2f}")
    print(stats)


if __name__ == "__main__":
    main()
