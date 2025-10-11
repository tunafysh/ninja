FROM rust:1.82-bullseye

SHELL ["/bin/bash", "-c"]

RUN rustup default nightly

# Install system dependencies for Tauri or other crates
RUN apt-get update && apt-get install -y \
    libgtk-3-dev libwebkit2gtk-4.0-dev libayatana-appindicator3-dev librsvg2-dev \
    build-essential curl wget pkg-config libssl-dev

RUN mkdir /build

WORKDIR /build

# Copy source and install frontend deps
COPY . .

RUN curl -o- https://fnm.vercel.app/install | bash
RUN source /root/.bashrc && fnm install 22 && \
    node -v # Should print "v22.20.0". && \
    corepack enable pnpm && \
    pnpm -v && \
    pnpm i -w ./GUI


RUN cargo xtask build-all
