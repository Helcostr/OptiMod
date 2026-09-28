# Host contract

OptiMod is the **parent** repository. Child content-checker plugins live under
`plugins/` and implement [`sdk/include/optimod_plugin.h`](../sdk/include/optimod_plugin.h).

See [`plugin-sdk.md`](plugin-sdk.md) for the programmer-facing DLL contract.

## Layout

```
OptiMod/
├── sdk/                              ← plugin author interface
├── crates/optimod-checker/           ← host pipeline + DLL loader
├── plugins/small-caps-obfuscation/   ← example child plugin
└── src/                              ← Twitch monitor (parent app)
```

## Parent vs child

| | Parent (OptiMod) | Child (plugin DLL) |
|---|---|---|
| Input to checker | `raw_message` from Twitch | UTF-8 bytes via `optimod_check` |
| Metadata | full `ChatMessage` | none |
| Enforcement | SSE drop, DB, flags | verdict code only |
