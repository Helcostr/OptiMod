#!/usr/bin/env python3
"""Run the twitch_small_caps_spam example through the plugin and print details."""

from __future__ import annotations

import ctypes
import sys
from pathlib import Path


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    text = (root / "fixtures/twitch_small_caps_spam/input.txt").read_text(encoding="utf-8").strip()

    lib_name = "cm_plugin.dll" if sys.platform == "win32" else "libcm_plugin.so"
    lib_path = root / "target/release" / lib_name
    if not lib_path.exists():
        print(f"Build first: cargo build -p cm-plugin --release", file=sys.stderr)
        return 1

    lib = ctypes.CDLL(str(lib_path))
    lib.cm_abi_version.restype = ctypes.c_uint32
    lib.cm_name.restype = ctypes.c_char_p
    lib.cm_check.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    lib.cm_check.restype = ctypes.c_int32

    payload = text.encode("utf-8")
    result = lib.cm_check(payload, len(payload))
    name = lib.cm_name().decode()

    verdict = {1: "ALLOW (good message)", 0: "BLOCK (bad message)"}.get(result, f"ERROR ({result})")

    print("=== DLL OUTPUT ===")
    print(f"cm_name():     {name}")
    print(f"cm_check():    {result}  ->  {verdict}")
    print(f"input:         {len(text)} chars, {len(payload)} UTF-8 bytes")
    print()
    print("=== WHAT THE DLL SEES (human vs machine) ===")
    print("Human eye:     normal-looking small-caps English")
    print("Machine:       hundreds of non-ASCII Unicode letters (U+1D00, U+028F, etc.)")
    print()
    print("=== INPUT (first 80 chars) ===")
    print(text[:80])
    print("...")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
