# syntax=docker/dockerfile:1.4

FROM node:current-bookworm

# --- Install Rust Nightly ---
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${PATH}"

# --- Install system dependencies for Tauri or other crates ---
RUN apt-get -q update && \
    apt-get install -y -q \
    libwebkit2gtk-4.1-dev \
    libappindicator3-dev \
    librsvg2-dev \
    libgtk-3-dev \
    libjavascriptcoregtk-4.1-dev \
    xdg-utils \
    curl unzip ca-certificates \
    python3 python3-venv python3-pip && \
    rm -rf /var/lib/apt/lists/*

# Download the latest installer
ADD https://astral.sh/uv/install.sh /uv-installer.sh

# Run the installer then remove it
RUN sh /uv-installer.sh && rm /uv-installer.sh

# Ensure the installed binary is on the `PATH`
ENV PATH="/root/.local/bin/:$PATH"

# --- Install pnpm and JS deps ---
RUN npm i -g pnpm@latest

# --- Create working directory ---
RUN mkdir /build
WORKDIR /build

# --- Copy project files ---
COPY . .

ENV CI=true

# --- Install Python deps (if you use a uv-managed project) ---
# This will auto-detect pyproject.toml if it exists
RUN uv sync --frozen || true

# --- Install frontend dependencies ---
RUN pnpm i -C ./GUI

# --- Run Python xtask build instead of cargo xtask ---
RUN --mount=type=secret,id=tauri_key \
    export TAURI_SIGNING_PRIVATE_KEY="$(cat /run/secrets/tauri_key)" && \
    uv run build.py
