# Secure Twitch Chat Monitor

Native Twitch OAuth + EventSub chat monitor with a Rust Axum backend and Solid.js UI.

## Features

- Login with Twitch via browser OAuth
- EventSub WebSocket subscription for `channel.chat.message`
- In-memory token storage
- SSE stream for UI/plugins
- Unicode normalization and confusable-character security checks
- Plugin trait with logger and metrics examples

## Run

### Local backend
1. Copy `config.toml.example` to `config.toml`.
2. Set environment variables:
   - `TWITCH_CLIENT_ID`
   - `TWITCH_CLIENT_SECRET`
   - `TWITCH_CHANNEL`
3. Start the Rust server.
4. Open `http://localhost:3000/` and click `Login with Twitch`.

### Local UI
1. `cd ui`
2. `npm install`
3. `npm run dev`

The UI reads `http://localhost:3000/api/status` and `http://localhost:3000/events`.

### Docker
1. Populate `.env` and `config.toml`.
2. Build with `docker compose up --build`.
3. Visit `http://localhost:3000/`.

## UI

The Solid app is a development dashboard that consumes `http://localhost:3000/events` via `EventSource` and shows live chat events.

## Security

The current pipeline normalizes chat text with NFC and flags confusable/control characters. It does not yet block messages; it only reports metadata.
