# syntax=docker/dockerfile:1.4

FROM node:current-bookworm

# Install Rust Nightly
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Create working directory
RUN mkdir /build
WORKDIR /build

# Copy project files
COPY . .

ENV CI=true

# Install system dependencies for Tauri or other crates
RUN apt-get -q update && \
    apt-get install -y -q \
    libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf curl libgtk-3-dev libjavascriptcoregtk-4.1-dev xdg-utils

# Install pnpm and dependencies
RUN npm i -g pnpm@latest && \
    pnpm i -C ./GUI

# Use secret during build (not persisted)
RUN --mount=type=secret,id=tauri_key \
    export TAURI_SIGNING_PRIVATE_KEY="$(cat /run/secrets/tauri_key)" && \
    cargo xtask build-all
