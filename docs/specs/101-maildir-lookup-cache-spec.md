# Especificação Técnica: Cache de Lookup de Arquivos no MaildirBackend

**ID do Ticket:** #101  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Concluído (Fase 9)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Durante o ciclo de prefetch de mensagens pelo cliente (`prefetch_missing_bodies`), o cliente solicita o corpo de cada mensagem individualmente através do endpoint `/api/mail/:mailbox/:uid` (`get_message`).
No servidor, cada requisição invocava `find_message_file_static(dir, uid)`, que por sua vez:
1. Executava `load_uid_maps_static(dir)` (lendo `dovecot-uidlist` e realizando `read_dir` completo em `cur/` e `new/`).
2. Realizava uma segunda varredura completa `read_dir` em `cur/` e `new/` para casar o prefixo encontrado.

Em uma caixa de entrada com milhares de mensagens, a sincronização de 37 corpos gerava mais de **74 varreduras completas no sistema de arquivos**, levando quase 1 minuto de I/O em disco desnecessário e travando a esteira de sincronização.

O objetivo deste ticket é introduzir um cache de diretório de alta performance (`MaildirDirCache`) indexado por UID em memória, validado por carimbos de modificação (`mtime`) dos subdiretórios `cur/`, `new/` e do arquivo `dovecot-uidlist`.

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Decisão 1: Validação por mtime dos Subdiretórios (`cur/`, `new/`, `dovecot-uidlist`):**
  - *Alternativa descartada:* Fazer `read_dir` completo e linear a cada busca de mensagem.
    - *Motivo do descarte:* Complexidade $O(M \times N)$ no disco, onde $M$ é a quantidade de mensagens no prefetch e $N$ é o total de arquivos na pasta. Degradação severa de performance no prefetch em lote.
  - *Alternativa descartada:* Cache em memória perene sem invalidação.
    - *Motivo do descarte:* Programas externos (Dovecot, mbsync, scripts locais) modificam mensagens, adicionam arquivos e atualizam flags externamente. Um cache estático ficaria dessincronizado.
  - *Escolha adotada:* Cache mapeado por caminho do diretório (`PathBuf`), contendo `HashMap<u32, CachedFileInfo>` e os carimbos `mtime` de `cur/`, `new/` e `dovecot-uidlist`. Como em sistemas de arquivos POSIX adicionar, remover ou renomear arquivos em uma pasta altera o `mtime` do diretório pai, 3 chamadas leves a `stat()` (nanossegundos) comprovam se o cache continua quente ou se precisa ser reconstruído.

- **Decisão 2: Invalidação Imediata nas Operações Locais Mutantes:**
  - *Contexto:* Ao alterar flags (`mark_message_read`, `mark_message_flagged`), mover (`move_message`) ou excluir (`delete_message`), o próprio `MaildirBackend` renomeia arquivos.
  - *Escolha adotada:* O método executor invalida ou atualiza imediatamente a entrada correspondente no cache do diretório afetado, sem depender de latência de granularidade de filesystem.

- **Decisão 3: Cache Thread-Safe com Suporte a Funções Estáticas:**
  - *Contexto:* O `MaildirBackend` possui métodos de instância e métodos utilitários associados (`find_message_file_static`, `load_uid_maps_static`, `read_message_static`), que são utilizados tanto por rotas HTTP quanto por tarefas de segundo plano (inotify e indexador vetorial).
  - *Escolha adotada:* Centralizar o cache sob um `RwLock` thread-safe global ou compartilhado, permitindo que todas as vias de acesso compartilhem o índice quente em memória.

---

## 3. Contratos e Requisitos de Teste

1. **Eficiência de Lookup:**
   - Para múltiplos lookups em sequência na mesma caixa postal sem alterações no disco, `find_message_file_static` deve resolver em memória $O(1)$ sem re-ler o diretório.
2. **Invalidação Reativa a Renomeação / Novas Mensagens:**
   - Quando um arquivo é renomeado ou criado (ex.: adição de `:2,S` ou chegada de nova mensagem), o cache detecta a alteração de `mtime` e recarrega os dados com exatidão.
3. **Invalidação por Atualização no `dovecot-uidlist`:**
   - Modificação no `dovecot-uidlist` invalida o mapeamento e reflete os novos UIDs.
4. **Isolamento de Diretórios:**
   - Cada pasta de correio (`INBOX`, `.Sent`, `.Trash`, etc.) mantém seu próprio isolamento de cache independente.

---

## 4. Laudo de Resolução (Fase 9)

- **[Resumo]:** Implementação de cache de lookup em memória indexado por UID (`MaildirDirCache`) no `MaildirBackend`, eliminando a varredura dupla `read_dir` por mensagem.
- **[Solução]:** Introdução de estrutura de carimbos `MaildirDirMtimes` (`cur/`, `new/`, `dovecot-uidlist`) e armazenamento global thread-safe `MAILDIR_CACHE`. Quando os carimbos não são alterados, a resolução de caminhos e flags é resolvida em $O(1)$ diretamente da memória. As mutações locais (`mark_message_read`, `mark_message_flagged`, `move_message`, `delete_message`, `send_message`) invalidam reativa e imediatamente o cache do diretório afetado.
- **[Validação]:** Testes dedicados em `lauda-server/tests/maildir_cache_test.rs` aprovados em 100%, suítes `maildir_test` e `websocket_sync_test` íntegras, `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings` 100% limpos sem alertas.
