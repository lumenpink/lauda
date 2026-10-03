# 🐳 Ambiente Docker para Testes e Homologação E2E

Este diretório contém a infraestrutura de contêiner para homologação da cadeia completa do ecossistema de correio soberano:

```text
[ lauda-client ] ──(HTTP/REST)──► [ lauda-server ] ──► [ Postfix ] ──(Milters)──► [ Dovecot Maildir ]
                                                              │
                                            ├─► [ Papéli Milter ]
                                            ├─► [ Ishcondidim Milter ]
                                            └─► [ Urupema Pipe Filter ]
```

## Como Usar

1. **Construção da imagem de teste:**
   ```bash
   docker build -t lauda-mail-stack -f tests/fixtures/docker/Dockerfile tests/fixtures/docker/
   ```

2. **Execução:**
   ```bash
   docker run --rm -it -p 2525:25 -p 1433:143 -p 8432:8432 lauda-mail-stack bash
   ```

3. **Execução Hermética no CI:**
   A suíte de testes Rust `e2e_full_chain_test.rs` roda nativamente no CI sem necessidade do daemon Docker, executando em microssegundos no `cargo test`.
