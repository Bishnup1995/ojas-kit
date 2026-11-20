# One shared base, one thin layer per arm. Everything pinned.
ARG CLAUDE_CODE_VERSION=2.1.329
ARG OJAS_COMMIT=a46b0452c056847db68bedc44ae8a638b5378f27
ARG CAVEMAN_TAG=v4.1.0
ARG HEADROOM_VERSION=0.44.0

# ---- build ojas from the pinned commit (never from the working tree) ----
FROM rust:1.87-slim-bookworm AS ojas-build
ARG OJAS_COMMIT
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates && rm -rf /var/lib/apt/lists/*
RUN git clone https://github.com/Bishnup1995/ojas-kit.git /src && cd /src && git checkout "$OJAS_COMMIT" \
 && cargo build --release && cp target/release/ojas /ojas

# ---- shared base ----
FROM node:22-bookworm AS base
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
ARG CLAUDE_CODE_VERSION
RUN apt-get update && apt-get install -y --no-install-recommends \
      git curl ca-certificates python3 python3-pip python3-venv jq ripgrep tree build-essential \
 && rm -rf /var/lib/apt/lists/*
RUN npm install -g "@anthropic-ai/claude-code@${CLAUDE_CODE_VERSION}"
# non-root user: Claude Code refuses --dangerously-skip-permissions as root
RUN useradd -m -s /bin/bash bench
ENV DISABLE_AUTOUPDATER=1 DISABLE_TELEMETRY=1 CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1
USER bench
WORKDIR /home/bench
ENV PATH="/home/bench/venv/bin:/home/bench/.cargo/bin:${PATH}"
# Toolchains and pinned task repos, installed once so runs never depend on the network.
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.87.0 >/dev/null 2>&1  && python3 -m venv /home/bench/venv
RUN mkdir -p /home/bench/work && cd /home/bench/work  && git clone -q https://github.com/pallets/click.git click && git -C click checkout -q 08b803cf32ceb2ae6a772c62d5da42757a897d44  && git clone -q https://github.com/expressjs/express.git express && git -C express checkout -q 43fca52d28a422c8be48fd489cbd4f2d6152bab6  && git clone -q https://github.com/BurntSushi/walkdir.git walkdir && git -C walkdir checkout -q 6fd031c82ba5a4204b4ce6eae73dacb00dc072ec
RUN cd /home/bench/work/click && pip install -q -e . pytest 2>&1 | tail -2  && cd ../express && npm install --no-audit --no-fund 2>&1 | tail -2  && cd ../walkdir && cargo build --tests -q 2>&1 | tail -2

# ---- arms ----
FROM base AS baseline
ENV ARM=baseline

FROM base AS ojas
ENV ARM=ojas
COPY --from=ojas-build /ojas /usr/local/bin/ojas
# `ojas init` writes hooks into ~/.claude/settings.json (default config, as documented)
RUN ojas init && ojas --version

FROM base AS caveman
ARG CAVEMAN_TAG
ENV ARM=caveman
RUN curl -fsSL "https://raw.githubusercontent.com/JuliusBrussee/caveman/${CAVEMAN_TAG}/install.sh" -o /tmp/caveman-install.sh \
 && bash /tmp/caveman-install.sh --only claude 2>&1 | tail -15
# The plugin alone is only the "say less" voice. The documented full install is the CLI setup
# (proxy, read-less compressors, blocks). Auto model routing stays inactive without `cave login`.
USER root
RUN npm install -g "@caveman-ai/cli@${CAVEMAN_TAG#v}"
USER bench
RUN cave setup --yes --agents claude 2>&1 | tee /tmp/cave-setup.log; ! grep -q "✗" /tmp/cave-setup.log

FROM base AS headroom
ARG HEADROOM_VERSION
ENV ARM=headroom
USER root
RUN pip install --break-system-packages uv >/dev/null 2>&1 || true
USER bench
ENV PATH="/home/bench/.local/bin:${PATH}"
RUN pip install --user --break-system-packages uv && uv tool install --python 3.13 "headroom-ai[all]==${HEADROOM_VERSION}" && headroom --version
