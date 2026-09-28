#!/usr/bin/env python3
"""Load the twitch_small_caps_spam fixture and call optimod_check via ctypes."""

from __future__ import annotations

import argparse
import os
import sys

from _optimod_plugin import PLUGIN_MODULE_ID, load_plugin, spam_fixture_path


def main() -> int:
    parser = argparse.ArgumentParser(description="Run the fixture through optimod_check.")
    parser.add_argument(
        "--expected",
        required=True,
        help="Expected optimod_check result: 1 (pass) or 0 (block)",
    )
    args = parser.parse_args()
    expected = int(args.expected)

    text = spam_fixture_path().read_text(encoding="utf-8").strip()
    payload = text.encode("utf-8")

    lib = load_plugin()

    name = lib.optimod_name().decode("utf-8")
    if name != PLUGIN_MODULE_ID:
        print(f"unexpected plugin name: {name}", file=sys.stderr)
        return 1

    force = os.environ.get("CM_FORCE_IS_GOOD", "")
    result = lib.optimod_check(payload, len(payload))
    print(f"plugin={name} force_is_good={force!r} optimod_check={result} expected={expected}")

    if result != expected:
        print("truth option mismatch", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
