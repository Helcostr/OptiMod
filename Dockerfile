# ---- Build Stage (Rust) ----
FROM rust:latest AS rust-builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev libpq-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY crates ./crates
COPY sdk ./sdk
COPY plugins ./plugins
RUN cargo build --release

# ---- Build Stage (UI) ----
FROM node:20-bookworm-slim AS ui-builder
WORKDIR /ui
COPY ui/package.json ui/package-lock.json ./
RUN npm ci
COPY ui ./
RUN npm run build

# ---- Final Runtime ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates libpq5 && rm -rf /var/lib/apt/lists/*
WORKDIR /app

RUN useradd -m -U bot
USER bot

COPY --from=rust-builder /app/target/release/secure-twitch-monitor ./bot
COPY --from=ui-builder /ui/dist ./dist

EXPOSE 3000
CMD ["./bot"]

# ---- Nginx (SPA + reverse proxy) ----
FROM nginx:alpine AS nginx
COPY nginx/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=ui-builder /ui/dist /usr/share/nginx/html
EXPOSE 80
