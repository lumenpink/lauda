# Especificação Técnica — Remoção de Supressão Artificial e Adoção de SMTP Efêmero em Testes (#81)

- **ID do Ticket:** #81
- **Status:** Em Progresso (Fase 3: Spec)
- **Autora:** Lumen
- **Alinhamento:** GOST 1.5.0

## 1. Contexto & Motivação
A trava de supressão `is_sendmail_suppressed_for_tests` e `is_smtp_live_dispatch_enabled` no crate `lauda-server` adicionava complexidade desnecessária e lógica artificial dentro do código de produção para adivinhar se a execução era um teste.
A abordagem correta e idiomática é manter o código de produção limpo: ele simplesmente envia SMTP via conexão TCP padrão RFC 5321 para o host e porta configurados no backend (`LAUDA_SMTP_HOST` e `LAUDA_SMTP_PORT`).
A responsabilidade de isolar o tráfego pertence exclusivamente à esteira e suíte de testes, que deve disponibilizar um servidor SMTP local efêmero (mock TCP) onde as mensagens são entregues e consumidas sem nenhum vazamento para o host ou internet.

## 2. Requisitos Técnicos
1. **Purificação do Código de Produção (`MaildirBackend`):**
   - Remover totalmente as funções `is_sendmail_suppressed_for_tests()`, `is_smtp_live_dispatch_enabled()` e variáveis como `LAUDA_FORCE_DISPATCH`.
   - Adicionar ao `MaildirBackend` a capacidade de configurar o endpoint SMTP (`smtp_host: String`, `smtp_port: u16`), com construtor `with_smtp_endpoint(base_dir, host, port)` e padrão lendo de `LAUDA_SMTP_HOST` e `LAUDA_SMTP_PORT` (ou `mailserver:25`).
   - O método `dispatch_email` conecta e envia via `dispatch_smtp_raw` de forma direta e sem bloqueios artificiais.
2. **Servidor SMTP Local Efêmero nos Testes:**
   - Criar helper/harness de teste reutilizável (`run_mock_smtp_server` ou struct `TestSmtpServer`) escutando em `127.0.0.1:0`.
   - Em `maildir_test.rs`, inicializar o mock SMTP efêmero e configurar o `MaildirBackend` com a porta gerada dinamicamente, permitindo validar o fluxo completo de envio de mensagens com sucesso.
   - Deletar ou readequar `sendmail_isolation_test.rs` para atestar a comunicação com o mock SMTP em vez de checar flags artificiais de supressão.

## 3. Critérios de Aceitação
- Zero referências a `is_sendmail_suppressed_for_tests` ou heurísticas de executável em `lauda-server`.
- 100% dos testes passando contra mocks SMTP locais efêmeros.
- Conformidade GOST via `cargo fmt -- --check`, `cargo clippy --workspace -- -D warnings` e `cccp check --strict`.

## 4. Checklist GOST DD-TDD
- [x] **Fase 1: Ticket** — Ticket #81 aberto no Forgejo e mapeado.
- [x] **Fase 2: Branch** — Branch `refactor/81-remover-supressao-artificial-e-adotar-smtp-efemero-nos-testes` criada.
- [x] **Fase 3: Spec** — Especificação técnica redigida.
- [ ] **Fase 4: Red Test** — Criado teste comprovando a nova interface `with_smtp_endpoint` e envio real ao mock.
- [ ] **Fase 5: Green Implement** — Removida a supressão artificial e implementado endpoint explícito.
- [ ] **Fase 6: Refactor** — Limpeza e padronização dos testes.
- [ ] **Fase 7: Mutation Test** — Cobertura de mutantes no endpoint SMTP.
- [ ] **Fase 8: Quality Gate** — Verificação rigorosa com clippy, fmt e Gosplan check.
- [ ] **Fase 9: Finish & Close** — Atualização do changelog e fechamento canônico.
