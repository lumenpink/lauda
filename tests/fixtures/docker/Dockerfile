FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    postfix \
    dovecot-core \
    dovecot-imapd \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Cria diretórios de correio e configuração
RUN mkdir -p /var/mail/vhosts/2lp.in/lumen/Maildir \
    && mkdir -p /run/dovecot \
    && mkdir -p /etc/postfix \
    && mkdir -p /etc/dovecot

# Script de entrada para homologação com Postfix + Dovecot
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 25 143 8432

ENTRYPOINT ["/entrypoint.sh"]
