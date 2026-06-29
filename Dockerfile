# ---- Build Stage (Rust) ----
FROM debian:bookworm as rust-builder
RUN apt-get update && apt-get install -y curl build-essential pkg-config libssl-dev
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /app
COPY Cargo.toml .
COPY src ./src
RUN cargo build --release

# ---- Build Stage (UI) ----
FROM node:20-bookworm-slim as ui-builder
WORKDIR /ui
COPY ui/package.json ./
# Remove the package-lock.json if it doesn't exist, we just want to install
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

# Copy binaries and ui artifacts
COPY --from=rust-builder /app/target/release/secure-twitch-monitor ./bot
COPY --from=ui-builder /ui/dist ./ui/dist

# Expose SSE API & Web server port
EXPOSE 3000

CMD ["./bot"]
