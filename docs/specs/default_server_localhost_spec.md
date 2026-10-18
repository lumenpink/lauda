# Especificação Técnica: Servidor Padrão Localhost (8432) em Instalação Limpa

**ID do Ticket:** #75  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificado (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Atualmente, ao instalar o Lauda smolclient em um dispositivo novo (em especial no Android via `detect_default_server_url()`), o campo de servidor padrão vem pré-preenchido com `https://lauda.lohn.in`.
Isso quebra o princípio de soberania e neutralidade de cliente open source:
1. Usuários que baixam o app não conseguem usar com seus próprios servidores sem alterar a URL pré-preenchida.
2. Em ambientes de desenvolvimento e teste locais, o app tenta conectar à rede do Baquara em vez de se comunicar com o backend local (`lauda-server`) na porta padrão `8432`.

Esta especificação define que instalações limpas devem apontar deterministicamente para `http://localhost:8432` como padrão local canônico.

---

## 2. Requisitos e Contratos

### 2.1. Função `detect_default_server_url()`
- **Android (`#[cfg(target_os = "android")]`):**
  Deve retornar `"http://localhost:8432".to_string()`.
- **Nativo Desktop/Linux (`#[cfg(not(target_os = "android"))]`):**
  Se a variável de ambiente `LAUDA_SERVER_URL` estiver presente e não-vazia, ela tem precedência.
  Caso contrário, deve retornar `"http://localhost:8432".to_string()`.
- **WebAssembly (`#[cfg(target_arch = "wasm32")]`):**
  Se executando em um navegador e o `window.location.origin` for válido, utiliza a origem do host (ex: `https://lauda.2lp.in`), preservando o comportamento para SPAs web. Se a origem for nula ou vazia, o fallback deve ser `"http://localhost:8432".to_string()`.

### 2.2. Interface do Usuário (`login_view.rs`)
- O placeholder do campo de texto `URL DO SERVIDOR:` deve ser `"http://localhost:8432"`.

### 2.3. Configuração de Sessão Padrão (`SessionConfig::default()`)
- `server_url` deve ser gerado através de `detect_default_server_url()`.
- Em ambiente sem sessão prévia salva, o cliente deve inicializar apontando para `http://localhost:8432`.

---

## 3. Critérios de Aceite e Verificação
1. Teste unitário em `lauda-client` validando que `detect_default_server_url()` retorna `"http://localhost:8432"` em ambiente nativo sem `LAUDA_SERVER_URL`.
2. Teste unitário validando que `SessionConfig::default().server_url` é `"http://localhost:8432"`.
3. Verificação de que a UI de login exibe placeholder `"http://localhost:8432"`.
4. Zero regressões em `cargo fmt --check`, `cargo clippy --workspace -- -D warnings` e `cargo test --workspace`.
