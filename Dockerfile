FROM rust:1.82-bullseye

SHELL ["/bin/bash", "-c"]

RUN rustup default nightly

RUN mkdir /build

WORKDIR /build

# Copy source and install frontend deps
COPY . .

ENV CI=true

# Install system dependencies for Tauri or other crates
RUN apt-get -q update && \
    apt-get install -y -q \
    build-essential curl wget pkg-config libssl-dev && \
    libgtk-3-dev libwebkit2gtk-4.0-dev libayatana-appindicator3-dev librsvg2-dev \
    curl -o- https://fnm.vercel.app/install | bash && \
    source /root/.bashrc && fnm install 22 && \
    npm i -g pnpm@latest && \
    pnpm i -C ./GUI && \
    cargo xtask build-all