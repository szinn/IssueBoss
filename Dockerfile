FROM rust:1@sha256:5b1e3484ddcd22a3738c0ec34a5e98bf19382eb295fb6db54295e62379119040 AS chef

# ARG TARGETPLATFORM
# ARG TARGETARCH
# ARG TARGETOS

RUN apt-get update && apt-get install -y --no-install-recommends musl-tools pkg-config && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef --locked
RUN rustup target add x86_64-unknown-linux-musl

# Install protobuf-compiler
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    binutils \
    build-essential \
    curl \
    mold \
    musl-tools \
    pkg-config \
    protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*

RUN cargo install dioxus-cli --locked --version 0.7.3

RUN curl -fsSL -o /usr/local/bin/tailwindcss \
    https://github.com/tailwindlabs/tailwindcss/releases/download/v4.2.1/tailwindcss-linux-x64 && \
    chmod +x /usr/local/bin/tailwindcss

WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# FROM chef AS builder-web
# COPY --from=planner /app/recipe.json recipe.json
#
# # Build deps layer (cached)
# RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
#
# COPY . .
#
# RUN tailwindcss -i ./crates/frontend/assets/input.css -o ./crates/frontend/assets/tailwind.css
#
# # Build web client
# RUN /usr/local/cargo/bin/dx bundle --web --package issueboss --release

FROM chef AS builder-server
COPY --from=planner /app/recipe.json recipe.json

# Build deps layer (cached)
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

COPY . .

# Build actual binary
# RUN tailwindcss -i ./crates/frontend/assets/input.css -o ./crates/frontend/assets/tailwind.css
# RUN /usr/local/cargo/bin/dx bundle --server --package issueboss --release --target x86_64-unknown-linux-musl
RUN cargo build --features server --bin issueboss --release --target x86_64-unknown-linux-musl
# RUN musl-strip target/x86_64-unknown-linux-musl/release/issueboss
RUN ls -lR target/x86_64-unknown-linux-musl

# Sanity check: should say "not a dynamic executable"
RUN ldd target/x86_64-unknown-linux-musl/release/issueboss || true

FROM ubuntu:latest@sha256:c4a8d5503dfb2a3eb8ab5f807da5bc69a85730fb49b5cfca2330194ebcc41c7b AS certs
RUN groupadd --gid 1234 issueboss && useradd -g 1234 -M -u 1234 -s /usr/sbin/nologin issueboss
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates
RUN update-ca-certificates

# FROM chef AS runtime
FROM scratch
COPY --from=certs /etc/passwd /etc/passwd
COPY --from=certs /etc/group /etc/group
COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder-server /app/target/x86_64-unknown-linux-musl/release/issueboss /app/issueboss

# LABEL tech.zinn.image.target_platform=$TARGETPLATFORM
# LABEL tech.zinn.image.target_architecture=$TARGETARCH
# LABEL tech.zinn.image.target_os=$TARGETOS

LABEL org.opencontainers.image.source="https://github.com/szinn/IssueBoss"
LABEL org.opencontainers.image.description="Take Control Of Your Project Issues"

WORKDIR /app
USER issueboss
ENTRYPOINT [ "/app/issueboss", "server" ]
