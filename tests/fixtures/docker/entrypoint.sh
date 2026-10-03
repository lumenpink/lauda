#!/bin/bash
set -e

echo "=== Iniciando ambiente de homologacao Soberano (Postfix + Dovecot + Lauda) ==="

# Inicia servico Dovecot
if command -v dovecot >/dev/null 2>&1; then
    dovecot
fi

# Inicia servico Postfix
if command -v postfix >/dev/null 2>&1; then
    postfix start
fi

echo "Ambiente pronto. Aguardando conexoes de teste..."
exec "$@"
