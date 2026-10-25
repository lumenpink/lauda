# ==============================================================================
# Lauda Server — Dockerfile Multi-Stage Minimalista
# ==============================================================================
FROM docker.io/lumenpink/gost-ci-cross:latest AS builder

WORKDIR /usr/src

# Configurar registros soberanos para crates.io e pacotes da gambi no obizero
RUN mkdir -p /root/.cargo && \
    printf '[source.crates-io]\nreplace-with = "sovereign-crates"\n\n[registries.sovereign-crates]\nindex = "sparse+https://crates.2lp.in/index/"\n\n[registries.gambi]\nindex = "sparse+https://gambi.2lp.in/api/packages/lumen/cargo/"\n' > /root/.cargo/config.toml

WORKDIR /usr/src/lauda

# Copiar arquivos de dependências e código-fonte
COPY Cargo.toml Cargo.lock ./
COPY lauda-types ./lauda-types
COPY lauda-index ./lauda-index
COPY lauda-client ./lauda-client
COPY lauda-server ./lauda-server

# Compilar o binário em modo release
RUN cargo build --release --bin lauda-server

# ==============================================================================
# Estágio de Execução (Debian Slim Minimal)
# ==============================================================================
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/lauda/target/release/lauda-server /usr/local/bin/lauda-server
COPY site/app /app/web

ENV LAUDA_LISTEN_ADDR="0.0.0.0:8432"
ENV LAUDA_MAILDIR_PATH="/var/mail/lumen.pink/hi"
ENV LAUDA_WEB_DIR="/app/web"

EXPOSE 8432

ENTRYPOINT ["lauda-server"]
