# ---- Build Stage (Rust) ----
FROM rust:latest AS rust-builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml .
COPY src ./src
RUN cargo build --release

# ---- Build Stage (UI) ----
FROM node:20-bookworm-slim AS ui-builder
WORKDIR /ui
COPY ui/package.json ./
RUN npm install
COPY ui ./
RUN npm run build

# ---- Final Runtime ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Create non-root user
RUN useradd -m -U bot
USER bot

# Copy binary
COPY --from=rust-builder /app/target/release/secure-twitch-monitor ./bot

EXPOSE 3000
CMD ["./bot"]
