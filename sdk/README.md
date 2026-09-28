# OptiMod plugin SDK

Content-checker plugins score **UTF-8 message text only**. OptiMod (the host) owns
Twitch metadata, transport, persistence, and enforcement policy.

## Contract

| Direction | Rule |
|-----------|------|
| **In** | `const uint8_t *input`, `size_t len` — raw chat message UTF-8 |
| **Out** | `int32_t` verdict: `1` pass, `2` flag, `0` block, `<0` error |
| **Metadata** | Plugins must not require user/channel/badge fields |

C header: [`include/optimod_plugin.h`](include/optimod_plugin.h)

Rust mirror: [`rust/optimod-plugin-sdk`](rust/optimod-plugin-sdk)

Full guide: [`../docs/plugin-sdk.md`](../docs/plugin-sdk.md)

## Example plugin

See [`../plugins/small-caps-obfuscation`](../plugins/small-caps-obfuscation) —
small-caps obfuscation detector (English-only today).

```bash
cargo build -p small-caps-obfuscation-plugin --release
# → target/release/optimod_small_caps_obfuscation.dll (Windows)
```

## Required exports

Every plugin DLL must export:

- `optimod_abi_version`
- `optimod_name`
- `optimod_check`
- `optimod_check_cstr` (recommended)
