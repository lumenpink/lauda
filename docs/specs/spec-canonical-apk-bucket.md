# Especificação Técnica: Canonização de APK para `lauda.apk` & Publicação no `twolp-bucket`

## 1. Contexto e Motivação
Para padronizar todos os clientes móveis do ecossistema sob a norma GOST e a convenção soberana 2lp, eliminamos sufixos como `-latest` ou `-app`. O pacote Android do Lauda passa a se chamar estritamente `lauda.apk`.
A distribuição física é unificada no bucket Cloudflare R2 `twolp-bucket` servido sob o domínio `https://bucket.2lp.in/lauda.apk`, reduzindo custos computacionais de worker e eliminando os limites de tamanho do Cloudflare Workers Assets.

## 2. Nomenclatura Canônica e Redirecionamento
- O arquivo gerado no build e empacotado em `site/` é `lauda.apk`.
- No Cloudflare Worker (`worker/index.js`), requisições para `/lauda.apk` e legadas para `/lauda-latest.apk` redirecionam transparentemente (302) para `https://bucket.2lp.in/lauda.apk`.
- Em `site/version.json`, o campo `apk_url` passa a apontar para `https://bucket.2lp.in/lauda.apk`.
- Em `site/index.html`, o link de download direto aponta para `https://bucket.2lp.in/lauda.apk`.

## 3. Upload R2
- O script `scripts/publish-r2.sh` e o workflow `.forgejo/workflows/ci.yml` utilizam o bucket `twolp-bucket`:
  - `npx wrangler r2 object put "twolp-bucket/lauda.apk" --file="site/lauda.apk" --remote`
  - Mantém cópia `twolp-bucket/lauda-latest.apk` para retrocompatibilidade.
