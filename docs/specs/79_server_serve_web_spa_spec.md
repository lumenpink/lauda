# Especificação Técnica: Servir SPA Web Estática Diretamente pelo Lauda Server

**ID do Ticket:** #79  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificado (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Atualmente, o proxy reverso Caddy do Obizero depende de um volume bind-mount estático (`/srv/lauda-app`) mapeado a partir de uma pasta do host (`/home/lumen/stacks/infra/static/lauda-app`) que não é atualizada pela esteira de CI. Como consequência, o backend (`lauda-server`) e o frontend web divergem em versões e funcionalidades.

Ao embutir os arquivos da SPA web compilada dentro da própria imagem Docker do `lauda-server` e configurá-lo para servir esses ativos estáticos com fallback de SPA, eliminamos a necessidade de gerenciar pastas do host e permitimos que o Watchtower atualize o frontend e o backend simultaneamente a cada push da CI.

---

## 2. Requisitos e Contratos

### 2.1. Suporte a Arquivos Estáticos no `lauda-server`
- Adicionar a feature `fs` na dependência `tower-http` do `lauda-server/Cargo.toml`.
- Em `lauda-server/src/adapters/http.rs`:
  - Permitir configuração do caminho de ativos web via variável de ambiente `LAUDA_WEB_DIR` (com valor padrão `/app/web`).
  - Se o diretório apontado por `LAUDA_WEB_DIR` existir no sistema de arquivos:
    - Registrar um serviço de fallback via `tower_http::services::ServeDir` apontando para o diretório de ativos.
    - Configurar fallback de rotas SPA para `{LAUDA_WEB_DIR}/index.html` via `tower_http::services::ServeFile`.
  - Rotas explícitas da API (`/auth/*`, `/messages*`, `/mailboxes*`, `/caldav*`, `/carddav*`, `/events`) mantêm estrita prioridade sobre os arquivos estáticos.

### 2.2. Empacotamento na Imagem Docker (`Dockerfile`)
- Copiar a pasta de distribuição web `site/app` para `/app/web` no estágio de execução final.
- Definir `ENV LAUDA_WEB_DIR="/app/web"`.

---

## 3. Critérios de Aceite e Verificação
1. **Teste Automatizado de Roteamento SPA**:
   - Teste de integração em `lauda-server` validando que, com `LAUDA_WEB_DIR` apontado para pasta temporária contendo `index.html` e `style.css`:
     - `GET /` retorna o conteúdo de `index.html` com status 200.
     - `GET /style.css` retorna o conteúdo de `style.css` com status 200.
     - `GET /inbox/view` (rota SPA não-estática) faz fallback e retorna `index.html` com status 200.
     - `POST /auth/login` continua sendo roteado para o handler de autenticação da API.
2. **Quality Gate**:
   - Zero alertas em `cargo clippy --workspace -- -D warnings`.
   - `cargo fmt -- --check` 100% aderente.
   - Suíte de testes aprovada.
