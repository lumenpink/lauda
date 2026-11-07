# Especificação Técnica: Suporte ao Prefixo /app para Ativos Estáticos e SPA Web no Lauda Server

**ID do Ticket:** #84  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Concluído (Fase 9)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

O `lauda-client` é compilado para Web com o caminho base prefixado em `/app/` (gerando tags como `<link rel="preload" href="/app/assets/..." />` e `<script src="/app/assets/..."></script>`).

No entanto, o `lauda-server` configurava o serviço estático `tower_http::services::ServeDir` somente como `fallback_service` na raiz do router do Axum. Como consequência:
1. Uma requisição para `/app/assets/lauda-client-*.js` não encontrava `/app/web/app/assets/...` no sistema de arquivos.
2. O Axum acionava o fallback `ServeFile::new("index.html")`.
3. O navegador recebia o conteúdo HTML de `index.html` com cabeçalho `content-type: text/html` em vez do código JavaScript.
4. O navegador abortava a execução do script com o erro `Failed to load module script: Expected a JavaScript module script but the server responded with a MIME type of "text/html"`, impossibilitando a inicialização da versão Web da aplicação.

---

## 2. Requisitos e Contratos

### 2.1. Roteamento de Ativos sob `/app` no `lauda-server`
- Em `lauda-server/src/adapters/http.rs`:
  - Quando `LAUDA_WEB_DIR` for um diretório válido contendo `index.html`:
    - Criar o serviço de diretório estático com fallback em `index.html`.
    - Montar o serviço explicitamente sob a rota `/app` via `.nest_service("/app", serve_dir.clone())`.
    - Manter também o serviço como `fallback_service(serve_dir)` para atender acessos diretos à raiz `/` e rotas SPA sem prefixo.
- Assegurar que:
  - `GET /app/assets/app.js` retorne o conteúdo do arquivo `assets/app.js` com status `200 OK` e tipo MIME apropriado.
  - `GET /app` e `GET /app/` retornem `index.html` com status `200 OK`.
  - `GET /app/qualquer-rota-spa` retorne `index.html` com status `200 OK`.
  - Rotas sob a raiz (`/assets/app.js`, `/`, etc.) continuem funcionando normalmente.

---

## 3. Critérios de Aceite e Verificação

1. **Teste Automatizado Red (Fase 4)**:
   - Adicionar casos de teste em `lauda-server/tests/spa_serve_test.rs`:
     - `GET /app/assets/app.js` retorna `200 OK` e o conteúdo do script.
     - `GET /app/` retorna `200 OK` e o conteúdo do `index.html`.
     - `GET /app/inbox` retorna `200 OK` e o conteúdo do `index.html`.
2. **Implementação Green (Fase 5)**:
   - Modificar `lauda-server/src/adapters/http.rs` para aninhar `/app` com `serve_dir`.
3. **Refactor & Quality Gate (Fases 6 e 8)**:
   - Executar `cargo clippy --all-targets -- -D warnings`.
   - Executar `cargo fmt --check`.
   - Rodar a suíte de testes `cargo test --package lauda-server --test spa_serve_test`.
