# Plugin: `small-caps-obfuscation`

Detects **small-caps Unicode obfuscation** in chat message text. Uses an English
dictionary today; other languages may be added later without changing the module id.

| Property | Value |
|----------|-------|
| **Folder** | `plugins/small-caps-obfuscation/` |
| **Module id** | `small-caps-obfuscation` (`optimod_name()`) |
| **Cargo package** | `small-caps-obfuscation-plugin` |
| **DLL** | `optimod_small_caps_obfuscation.dll` |
| **Language support** | English only (for now) |

Implements the OptiMod C ABI — [`sdk/include/optimod_plugin.h`](../../sdk/include/optimod_plugin.h).

## Build

```bash
# from OptiMod repo root
git submodule update --init --recursive
cargo build -p small-caps-obfuscation-plugin --release
# → target/release/optimod_small_caps_obfuscation.dll
```

```env
OPTIMOD_PLUGIN_PATH=target/release/optimod_small_caps_obfuscation.dll
OPTIMOD_CHECKERS=small-caps-obfuscation
```

## Why multiple internal crates?

OptiMod only requires the `optimod_*` DLL exports. The crates below are **internal**
to this example — a minimal plugin can be one `cdylib` crate.

| Crate | Role |
|-------|------|
| `small-caps-obfuscation-plugin` | DLL shim (`optimod_*` exports) |
| `cm-english` | Scoring (English dictionary + obfuscation ratios) |
| `cm-alphabet` | Bitmap + canonicalize (`build.rs` from `data/custom_alphabet.toml`) |
| `cm-normalize` | NFKC wrapper |
| `cm-core` | In-plugin types |
| `cm-fixture-runner` | JSONL CI (not shipped in DLL) |

```
message → NFKC → alphabet canonicalize → English word scoring → Pass/Flag/Block
```

## Fixtures

```bash
cargo run -p cm-fixture-runner -- fixtures/
```

## Configuration

| Env var | Default |
|---------|---------|
| `CM_BLOCK_OBFUSCATION_THRESHOLD` | `0.75` |
| `CM_BLOCK_ENGLISH_RATIO` | `0.75` |
