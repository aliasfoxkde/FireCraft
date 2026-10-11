# Pre-warmed Rust + WebAssembly CI image for the local GitForge pipeline
# (FireCraft).
#
# Same rationale as backend-fixed's ci-rust image (the reference
# implementation on this fleet): bare rust:1-slim-bookworm jobs
# re-download the whole crates.io graph per run and crates.io transfers
# stall on this network, so every input a job needs is baked at build
# time and jobs touch the network for nothing.
#
# FireCraft additions beyond the backend image:
# - the wasm32-unknown-unknown target — the primary release target is a
#   WASM build for the user's WebOS, so CI judges the wasm compile on
#   every push, not just locally;
# - trunk (the web bundle build, `xtask wasm`);
# - wasm-bindgen-cli + nodejs — wasm-bindgen-test-runner shells out to
#   node, so the fwpng wasm smoke suite executes rather than being
#   compile-only;
# - a fetch of the standalone fuzz/ workspace (not a root-workspace
#   member, so the main fetch does not warm it) so the stable fuzz
#   drivers build offline.
#
# Build with a manifest-and-source-only context — building from the repo
# root would ship the NAS-backed target/ directory to the daemon:
#
#   tmp=$(mktemp -d)
#   cp Cargo.toml Cargo.lock "$tmp"/
#   cp --parents crates/*/Cargo.toml "$tmp"/
#   cp --parents -r crates/*/src "$tmp"/
#   cp --parents xtask/Cargo.toml xtask/src web/Cargo.toml web/src "$tmp"/
#   cp --parents fuzz/Cargo.toml fuzz/Cargo.lock "$tmp"/
#   cp --parents -r fuzz/src fuzz/fuzz_targets "$tmp"/
#   docker build -f infrastructure/docker/ci-rust.Dockerfile \
#          -t firecraft-ci-rust:1 "$tmp"
#
# cargo fetch needs each workspace member's manifest plus its declared
# targets (src/); nothing else ships in the context.
#
# Ship to every fleet member before pushing a commit that relies on it —
# the GitForge runner has a hard 60-second sandbox-acquisition cap and
# will never pull the image itself:
#
#   docker save firecraft-ci-rust:1 | ssh <host> docker load
#
# Size guard: keep the image within a few GB — both known daemons run
# overlay2, so the practical limits are LAN transfer time and fleet disk.
#
# Rebuild whenever Cargo.lock or fuzz/Cargo.lock changes, when a pinned
# tool version below moves, or when the Rust minor moves (keep CI on the
# toolchain local dev uses — see the FROM note; this pin matches
# backend-ci-rust:1 so both pipelines judge the same rustc). The
# wasm-bindgen-cli pin MUST equal the wasm-bindgen version in
# Cargo.lock — a mismatched smoke runner rejects the generated bindings.
FROM rust:1.99-slim-bookworm AS fetch
# WORKDIR must precede the COPYs: with the default cwd of /, "./" lands
# the manifests at / and cargo fetch resolves the workspace from
# /Cargo.toml, where no crates/ tree exists.
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY xtask/ ./xtask/
COPY web/ ./web/
COPY fuzz/ ./fuzz/
# Warm both workspaces from their committed lockfiles. The fuzz
# workspace is standalone (`[workspace]` in fuzz/Cargo.toml) and shares
# the same cargo registry cache.
RUN cargo fetch --locked \
    && cargo fetch --locked --manifest-path fuzz/Cargo.toml

FROM rust:1.99-slim-bookworm

# No nodejs here: bookworm's nodejs is 18, and the wasm-bindgen 0.2.129
# test runtime traps in its V8 before running a single smoke test
# (observed: raw "Stacktrace: ptr1=..." V8 dump inside the first test's
# js_sys call; the same suite passes under node 24). Node is installed
# from the official distribution below, pinned to the developer's
# version. No git: no CI step talks to git, and no dependency in either
# lockfile comes from a git source. No libssl: nothing in the graph
# builds openssl-sys.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && rustup component add rustfmt clippy \
    && rustup target add wasm32-unknown-unknown

# Version-pinned to match local dev exactly (trunk 0.21.14 and
# wasm-bindgen-cli 0.2.129 are the versions on the developer's PATH).
# Each in its own layer so bumping one never rebuilds the other.
RUN cargo install --locked trunk --version 0.21.14
RUN cargo install --locked wasm-bindgen-cli --version 0.2.129

# node v24.16.0 — the developer's version (local dev runs v24.16.0 via
# nvm; bookworm's 18 is too old for the wasm-bindgen test runtime, see
# the apt note above). Checksum-pinned so the build fails loudly if the
# distribution ever changes under the pin. Kept after the cargo install
# layers so bumping node never rebuilds the toolchain layers.
ADD --checksum=sha256:2faf6a387e9b62b888e21c54f01249fb27537ffecf1842f29f4c919d0a59a0ff \
    https://nodejs.org/dist/v24.16.0/node-v24.16.0-linux-x64.tar.gz /tmp/node.tar.gz
RUN tar -xzf /tmp/node.tar.gz --strip-components=1 -C /usr/local \
    && rm /tmp/node.tar.gz

COPY --from=fetch /usr/local/cargo/registry /usr/local/cargo/registry

# A missing crate fails in seconds instead of hanging on a stalled
# crates.io transfer (the dsc pipeline lost runs to "transfer too slow"
# retries before compiling anything).
ENV CARGO_NET_OFFLINE=true \
    CARGO_TERM_COLOR=always
