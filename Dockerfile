# ==============================================================================
# Lauda Server — Dockerfile Multi-Stage Minimalista
# ==============================================================================
FROM codeberg.org/lumenpink/gost-ci-rust:latest AS builder

WORKDIR /usr/src

# Clonar dependências irmãs para satisfazer dev-dependencies do workspace em build hermético
RUN git clone --depth 1 https://codeberg.org/lumenpink/ishcondidim.git /usr/src/ishcondidim && \
    git clone --depth 1 https://codeberg.org/lumenpink/papeli.git /usr/src/papeli && \
    git clone --depth 1 https://codeberg.org/lumenpink/urupema.git /usr/src/urupema

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

ENV LAUDA_LISTEN_ADDR="0.0.0.0:8432"
ENV LAUDA_MAILDIR_PATH="/var/mail/lumen.pink/hi"

EXPOSE 8432

ENTRYPOINT ["lauda-server"]
