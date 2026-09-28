"""ctypes loader for the small-caps-obfuscation OptiMod plugin DLL."""

from __future__ import annotations

import ctypes
import sys
import time
from pathlib import Path

TWITCH_SMALL_CAPS_SPAM = "twitch_small_caps_spam"
PLUGIN_MODULE_ID = "small-caps-obfuscation"

_LIB_NAME = {
    "win32": "optimod_small_caps_obfuscation.dll",
    "darwin": "liboptimod_small_caps_obfuscation.dylib",
}.get(sys.platform, "liboptimod_small_caps_obfuscation.so")


def workspace_root() -> Path:
    # plugins/small-caps-obfuscation/scripts/ → OptiMod root
    return Path(__file__).resolve().parents[2]


def plugin_root() -> Path:
    return Path(__file__).resolve().parents[1]


def plugin_path() -> Path:
    return workspace_root() / "target" / "release" / _LIB_NAME


def configure_optimod_exports(lib: ctypes.CDLL) -> None:
    lib.optimod_abi_version.restype = ctypes.c_uint32
    lib.optimod_name.restype = ctypes.c_char_p
    lib.optimod_check.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    lib.optimod_check.restype = ctypes.c_int32


def load_plugin(*, timed: bool = False) -> ctypes.CDLL | tuple[ctypes.CDLL, float]:
    path = plugin_path()
    if not path.exists():
        raise FileNotFoundError(
            "Build the plugin first: "
            f"cargo build -p small-caps-obfuscation-plugin --release ({path})"
        )

    started = time.perf_counter()
    lib = ctypes.CDLL(str(path))
    configure_optimod_exports(lib)

    if timed:
        load_ms = (time.perf_counter() - started) * 1000
        return lib, load_ms
    return lib


def spam_fixture_path() -> Path:
    return plugin_root() / "fixtures" / TWITCH_SMALL_CAPS_SPAM / "input.txt"
