# Especificação de Isolamento Hermético de Sendmail em Testes (RFC-55)

- **Ticket de Referência:** #55 (`fix(server): isolar despacho sendmail em testes e suprimir vazamento para caixas reais`)
- **Status:** Aprovado / Em Implementação
- **Norma GOST:** GOST-04-TICKETS-COMMITS, Ordem Nº 227 (Hermeticidade de Testes)

---

## 1. Problema Identificado
O método `MaildirBackend::dispatch_sendmail` no crate `lauda-server` inspeciona a presença de um container Docker chamado `mailserver` em execução no host local. Se o container estiver ativo, despacha mensagens via `docker exec -i mailserver /usr/sbin/sendmail -t -i`.

Como resultado, a execução ordinária de testes de integração (`cargo test`) injetava e-mails reais no Postfix do ambiente de desenvolvimento, poluindo caixas postais do desenvolvedor com centenas de mensagens de teste. Além disso, o arquivo `lauda-server/tests/maildir_test.rs` continha caminhos absolutos para a caixa postal de produção do usuário.

---

## 2. Requisitos de Isolamento

### 2.1 Trava de Supressão Automática em Teste
O método `MaildirBackend::dispatch_sendmail` **DEVE** abortar o envio real e retornar `Ok(())` imediatamente quando:
1. `cfg!(test)` estiver ativo durante a compilação de testes;
2. A variável de ambiente `LAUDA_TEST_MODE` estiver definida;
3. A variável de ambiente `CI` estiver definida;
4. A variável de ambiente `LAUDA_SENDMAIL_CMD` estiver explicitamente configurada como `"mock"`, `"none"` ou vazia.

### 2.2 Eliminação de Caminhos Reais nos Testes
Em `lauda-server/tests/maildir_test.rs`, o primeiro teste `test_maildir_backend_read_mailboxes_and_messages` deve ser refatorado para utilizar uma árvore Maildir sintética e hermética gerada sob `tempfile::tempdir()`, eliminando qualquer dependência ou leitura da pasta `/home/lumen/stacks/mail/data/...`.

---

## 3. Critérios de Aceite
- [ ] Nenhuma chamada a `docker exec` ou `/usr/sbin/sendmail` ocorre durante qualquer suite de testes (`cargo test`).
- [ ] O teste `maildir_test.rs` executa de forma 100% hermética sem tocar em caixas postais reais do host.
- [ ] A funcionalidade de despacho real em ambiente de produção permanece inalterada quando o servidor estiver rodando fora de modo de teste.
