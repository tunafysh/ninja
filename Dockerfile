FROM node:current-bookworm

# Install Rust Nightly
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"

ARG SIGNING_KEY
ENV TAURI_SIGNING_PRIVATE_KEY=${SIGNING_KEY}

RUN mkdir /build

WORKDIR /build

# Copy source and install frontend deps
COPY . .

ENV CI=true

# Install system dependencies for Tauri or other crates
RUN apt-get -q update && \
    apt-get install -y -q \
    libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf curl libgtk-3-dev libjavascriptcoregtk-4.1-dev xdg-utils


RUN npm i -g pnpm@latest && \
    pnpm i -C ./GUI && \
    cargo xtask build-all
