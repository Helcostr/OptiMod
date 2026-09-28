# OptiMod — Secure Twitch Chat Monitor

Native Twitch OAuth + EventSub chat monitor with a Rust Axum backend, Solid.js UI,
and a **plugin SDK** for content-checker DLLs.

## Repository layout

```
OptiMod/                              ← parent (this repo)
├── sdk/                              ← plugin contract for DLL authors
├── plugins/small-caps-obfuscation/   ← example child plugin
├── crates/optimod-checker/           ← host pipeline + loader
└── src/                              ← Twitch monitor app
```

## Features

- Login with Twitch via browser OAuth
- EventSub WebSocket subscription for `channel.chat.message`
- Content-checker plugins (in-process + DLL via `OPTIMOD_PLUGIN_PATH`)
- Event plugins: logger, metrics, database
- Unicode normalization and confusable-character pre-checks

## Plugin SDK

Plugin authors implement four C exports defined in
[`sdk/include/optimod_plugin.h`](sdk/include/optimod_plugin.h).

Full guide: [`docs/plugin-sdk.md`](docs/plugin-sdk.md)

Example plugin: [`plugins/small-caps-obfuscation`](plugins/small-caps-obfuscation)

```bash
git submodule update --init --recursive
cargo build -p small-caps-obfuscation-plugin --release
# → target/release/optimod_small_caps_obfuscation.dll
```

## Run

### Local backend

1. Copy `config.toml.example` to `config.toml`.
2. Set `TWITCH_CLIENT_ID`, `TWITCH_CLIENT_SECRET`, `TWITCH_CHANNEL`.
3. `cargo run`
4. Open `http://localhost:3000/` and click **Login with Twitch**.

### Local UI

```bash
cd ui && npm install && npm run dev
```

### Docker

```bash
docker compose up --build
```

## Checker configuration

| Env var | Default | Layer |
|---------|---------|-------|
| `OPTIMOD_CHECKERS` | `small-caps-obfuscation` | Host — built-in checker |
| `OPTIMOD_PLUGIN_PATH` | — | Host — DLL paths (`;`-separated) |
| `OPTIMOD_DROP_BLOCKED` | `false` | Host — skip SSE on block |
| `CM_BLOCK_OBFUSCATION_THRESHOLD` | `0.75` | Plugin — english module |
| `CM_BLOCK_ENGLISH_RATIO` | `0.75` | Plugin — english module |
