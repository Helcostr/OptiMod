#!/usr/bin/env python3
"""Walk through early-exit decisions with CM_TRACE_EARLY_EXIT=1.

Build first (from OptiMod repo root):
  cargo build -p small-caps-obfuscation-plugin --release
"""

from __future__ import annotations

import argparse
import os
import sys
import time

from _optimod_plugin import load_plugin, plugin_path, spam_fixture_path

CASES = {
    "ascii": "hello world",
    "mixed": "hellᴏ stream",
    "dilution": "ʏᴏ ʙʀᴏ " + "hello " * 20,
    "spam": None,
}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", default="all", choices=[*CASES.keys(), "all"])
    parser.add_argument("--pause", type=float, default=0.0)
    args = parser.parse_args()

    os.environ["CM_TRACE_EARLY_EXIT"] = "1"
    load_plugin()
    print(f"loaded {plugin_path()}", file=sys.stderr)

    names = CASES.keys() if args.case == "all" else [args.case]
    for name in names:
        text = CASES[name] if name != "spam" else spam_fixture_path().read_text(encoding="utf-8")
        payload = text.encode("utf-8")
        print(f"\n=== case: {name} ===", file=sys.stderr)
        lib = load_plugin()
        code = lib.optimod_check(payload, len(payload))
        print(f"optimod_check={code}", file=sys.stderr)
        if args.pause:
            time.sleep(args.pause)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
