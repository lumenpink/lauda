# Especificação Técnica - Ticket #88

## 1. Objetivo
Garantir compilação limpa em alvos WASM (`wasm32-unknown-unknown`) e empacotamento web confiável no CI (`deploy-web` e `deploy-apk`).

## 2. Diagnóstico & Problema
1. Em `lauda-client/src/ui/mod.rs`, a declaração `let cache_dir = crate::domain::cache::get_default_cache_dir();` estava condicional a `#[cfg(not(target_arch = "wasm32"))]`.
2. No tratamento de `ClientError::AuthFailed`, `crate::domain::session::logout_and_wipe` é invocado passando `&cache_dir`. Como essa chamada ocorre em código comum a todas as plataformas, causou erro de compilação `cannot find value cache_dir in this scope` em alvos WASM.
3. O `dx build` na etapa `deploy-web` sofre timeout durante a resolução do `cargo metadata` em virtude de concorrência ou bloqueio de rede no container, resolvido com a flag `--offline`.

## 3. Diretrizes de Implementação
1. Em `lauda-client/src/ui/mod.rs`, remover a anotação `#[cfg(not(target_arch = "wasm32"))]` da definição de `cache_dir`, tornando-a incondicional (já compatível via `get_default_cache_dir()`).
2. No fluxo de `deploy-web` em `.forgejo/workflows/ci.yml`, passar a flag `--offline` ao `dx build -p lauda-client --platform web --release`.

## 4. Validação
- `cargo check -p lauda-client --target wasm32-unknown-unknown --features web` deve passar sem nenhum erro.
- Execução das suítes de teste de regressão e compliance audit com `cccp check --strict`.
