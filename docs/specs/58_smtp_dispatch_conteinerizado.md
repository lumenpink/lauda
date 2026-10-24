# Especificação Técnica — Despacho SMTP Resiliente em Ambiente Conteinerizado (#58)

- **ID do Ticket:** #58
- **Status:** Aprovado
- **Autora:** Lumen
- **Alinhamento:** GOST 1.5.0

## 1. Contexto & Motivação
Em ambiente conteinerizado (Docker Compose), o container `lauda-server` opera isolado sem o binário `/usr/sbin/sendmail` e sem acesso ao runtime do Docker do host (`/var/run/docker.sock`).
Atualmente, o método `dispatch_sendmail` em `lauda-server/src/adapters/maildir.rs` tenta invocar `docker exec` ou `/usr/sbin/sendmail`, e ao falhar, retorna `Ok(())` silenciosamente após persistir o e-mail em `.Sent`. Isso gera falso positivo no cliente, enquanto a mensagem nunca atinge o Postfix.

## 2. Requisitos Técnicos
1. **Despacho Direto via SMTP Interno:**
   - Implementar cliente SMTP assíncrono nativo em `lauda-server` utilizando `tokio::net::TcpStream`.
   - Conectar por padrão a `${LAUDA_SMTP_HOST:-mailserver}:${LAUDA_SMTP_PORT:-25}` na rede interna Docker (`mail_net`).
   - Ciclo SMTP RFC 5321: `EHLO` -> `MAIL FROM:<...>` -> `RCPT TO:<...>` -> `DATA` -> `QUIT`.
2. **Prioridade de Configuração:**
   - Se `LAUDA_SENDMAIL_CMD` estiver definido no ambiente, executá-lo via processo/pipe (útil para desenvolvimento local ou testes com mocks CLI).
   - Caso contrário, despachar via SMTP assíncrono para o host/porta configurados.
3. **Propagação Estrita de Erros e Limpeza de `.Sent`:**
   - Em caso de falha de conexão ou recusa SMTP (códigos 4xx/5xx), a função `send_message` deve remover o arquivo gravado provisoriamente em `.Sent` e retornar `Err(BackendError::Internal(...))`.
   - O silenciamento com `Ok(())` sem despacho efetivo é formalmente proibido.
4. **Isolamento de Testes:**
   - Preservar supressão em testes normais (`cfg!(test)` / `LAUDA_TEST_MODE`), permitindo ativação explícita para testes de integração de SMTP via porta mockada.

## 3. Critérios de Aceitação
- Teste de integração automatizado com servidor SMTP mock (`tests/smtp_dispatch_test.rs`) validando o diálogo SMTP completo e tratamento de erros.
- Validação estrita via `cargo fmt -- --check`, `cargo clippy` e `cccp check --strict`.
