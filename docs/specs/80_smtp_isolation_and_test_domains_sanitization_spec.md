# Especificação Técnica — Isolamento Hermético SMTP e Saneamento de Domínios de Teste (#80)

- **ID do Ticket:** #80
- **Status:** Em Progresso (Fase 3: Spec)
- **Autora:** Lumen
- **Alinhamento:** GOST 1.5.0

## 1. Contexto & Motivação
Em ambiente de desenvolvimento e esteira de testes, mensagens de correio eletrônico sintéticas (ex: `testuser@2lp.in` para `dest@2lp.in`, ou `a@b.com`) disparadas durante testes de integração ou execução de binários locais conseguiram vazar para o servidor de correio local (`mailserver`). Como o Postfix possui relay configurado para o SMTP2GO e os destinatários/remetentes sintéticos não existem, o SMTP2GO rejeitou as mensagens e gerou reportes de bounce que prejudicam a reputação do domínio.

A trava anterior `is_sendmail_suppressed_for_tests` dependia de heurísticas frágeis (`cfg!(test)` não tem efeito em testes de integração de bibliotecas, e `current_exe()` não detecta execuções fora do padrão `test` ou `/deps/`).

## 2. Requisitos Técnicos

1. **Despacho Seguro por Padrão (Opt-In Estrito):**
   - O despacho real via SMTP só deve ocorrer se a variável `LAUDA_SMTP_ENABLED` estiver explicitamente definida como `"1"` ou `"true"` (ou `LAUDA_FORCE_DISPATCH="1"`).
   - Na ausência dessa variável (padrão em desenvolvimento, testes, compilação e esteiras CI), o despacho real é suprimido imediatamente sem tentativa de conexão de rede, registrando log informativo e retornando `Ok(())`.
   - No container de produção em [`compose.yml`](file:///home/lumen/stacks/mail/compose.yml), a variável `LAUDA_SMTP_ENABLED=true` deve ser declarada.

2. **Saneamento Canônico de Domínios e E-mails Sintéticos (RFC 2606 / RFC 6761):**
   - Todos os e-mails de teste em suítes de teste (como `maildir_test.rs`) devem ser migrados de `@2lp.in` para domínios reservados pela IANA para testes: `@example.test`, `@example.com` ou `@example.org`.
   - É terminantemente proibido o uso de domínios reais ou funcionais (`@2lp.in`, `@lumen.pink`, `@lohn.in`) como destinatários fictícios que possam causar vazamento para relays públicos.

3. **Isolamento de Descarte de Cabeçalhos Problemáticos no Mailserver:**
   - No container `mailserver` (Amavis), configurar `$final_bad_header_destiny = D_DISCARD;` via arquivo de configuração `/tmp/docker-mailserver/amavis.cf` (`/etc/amavis/conf.d/50-user`) para descartar spams/mensagens com cabeçalhos malformados sem gerar bounces com remetente vazio (`<>`) para o SMTP2GO.

## 3. Critérios de Aceitação
- Testes automatizados garantindo que:
  - Na ausência de `LAUDA_SMTP_ENABLED=true`, nenhuma conexão de rede é tentada e a supressão é ativa.
  - Quando `LAUDA_SMTP_ENABLED=true` (ou `LAUDA_FORCE_DISPATCH=1`), a rotina prossegue para o host/porta configurados.
- 100% dos testes unitários e de integração de `lauda-server` aprovados sem disparar sockets na porta 25 externa.
- `cargo fmt -- --check`, `cargo clippy --workspace -- -D warnings`.

## 4. Checklist GOST DD-TDD
- [x] **Fase 1: Ticket** — Ticket #80 criado no Forgejo e sincronizado.
- [x] **Fase 2: Branch** — Branch `fix/80-isolar-envio-smtp-e-sanear-dominios-de-teste-para-prevenir-bounces` criada.
- [x] **Fase 3: Spec** — Documento técnico de especificação redigido.
- [x] **Fase 4: Red Test** — Criado teste comprovando a falha/ausência da trava segura por padrão.
- [x] **Fase 5: Green Implement** — Implementada a trava opt-in e saneados os e-mails de teste.
- [x] **Fase 6: Refactor** — Limpeza de código e garantia de robustez.
- [x] **Fase 7: Mutation Test** — Validação com mutantes eliminados.
- [x] **Fase 8: Quality Gate** — Verificação rigorosa com clippy, fmt e testes.
- [ ] **Fase 9: Finish & Close** — Atualização do changelog e fechamento canônico.
