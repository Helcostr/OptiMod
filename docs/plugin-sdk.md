# OptiMod plugin SDK — programmer guide

OptiMod is the **host**. Your plugin is a **child** shared library that scores chat
message text. The host passes UTF-8 bytes; your DLL returns a verdict code.

## What you implement

Copy [`sdk/include/optimod_plugin.h`](../sdk/include/optimod_plugin.h) into your project
and export these four symbols from a `cdylib`:

| Export | Signature | Purpose |
|--------|-----------|---------|
| `optimod_abi_version` | `uint32_t()` | Return `OPTIMOD_PLUGIN_ABI_VERSION` (currently `1`) |
| `optimod_name` | `const char *()` | Stable module id, e.g. `"small-caps-obfuscation"` |
| `optimod_check` | `int32_t(const uint8_t *, size_t)` | Score one message |
| `optimod_check_cstr` | `int32_t(const char *)` | Optional NUL-terminated helper |

## Input contract

- **Only** the raw chat message string (UTF-8).
- No user id, channel login, badges, emotes, or timestamps.
- `input` may be `NULL` only when `len == 0` (empty message).

## Output contract (`optimod_check` return value)

| Value | Constant | Host behavior |
|-------|----------|---------------|
| `1` | `OPTIMOD_CHECK_PASS` | Deliver message |
| `2` | `OPTIMOD_CHECK_FLAG` | Deliver; `security_flags` get `<name>:flag` |
| `0` | `OPTIMOD_CHECK_BLOCK` | Block (or flag only if `OPTIMOD_DROP_BLOCKED` unset) |
| `-1` | `OPTIMOD_ERR_NULL` | Host logs error, skips plugin for this message |
| `-2` | `OPTIMOD_ERR_UTF8` | Host logs error, skips plugin for this message |

The host calls `optimod_abi_version()` at load time. Mismatched versions are rejected.

## Rust plugins

Use the workspace crate `optimod-plugin-sdk` for shared types:

```rust
use optimod_plugin_sdk::{Action, ABI_VERSION, check_code};

#[no_mangle]
pub extern "C" fn optimod_abi_version() -> u32 {
    ABI_VERSION
}

#[no_mangle]
pub extern "C" fn optimod_check(input: *const u8, len: usize) -> i32 {
    // decode UTF-8, run your logic, then:
    Action::Block.to_check_code()
}
```

Set `crate-type = ["cdylib"]` in `Cargo.toml`.

## Loading in OptiMod

Built-in checkers (in-process) are enabled with `OPTIMOD_CHECKERS=small-caps-obfuscation`.

External DLLs use semicolon-separated paths:

```env
OPTIMOD_PLUGIN_PATH=target/release/optimod_small_caps_obfuscation.dll
```

Host env vars:

| Variable | Default | Effect |
|----------|---------|--------|
| `OPTIMOD_CHECKERS` | `small-caps-obfuscation` | Built-in module ids (comma-separated) |
| `OPTIMOD_PLUGIN_PATH` | — | DLL paths (`;`-separated on Windows) |
| `OPTIMOD_DROP_BLOCKED` | `false` | Skip SSE when any checker blocks |

## Example plugin

[`plugins/small-caps-obfuscation`](../plugins/small-caps-obfuscation) is the
reference implementation (small-caps Unicode obfuscation; English-only today).

```bash
git submodule update --init --recursive
cargo build -p small-caps-obfuscation-plugin --release
```

Produces `target/release/optimod_small_caps_obfuscation.dll` (`optimod_name()` →
`small-caps-obfuscation`).

## What not to do

- Do not require Twitch/OAuth types inside the DLL.
- Do not block on network, disk, or database I/O inside `optimod_check`.
- Do not change export names or calling convention without bumping ABI version.

## Repository layout

```
OptiMod/                          ← host repo (you clone this)
├── sdk/                          ← contract for plugin authors
├── plugins/small-caps-obfuscation/   ← example child plugin
├── crates/optimod-checker/       ← host loader + pipeline
└── src/                          ← Twitch monitor app
```
