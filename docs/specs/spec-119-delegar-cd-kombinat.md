# Especificação do Ticket #119: Delegar Entregas e Reset de Container do Lauda ao Kombinat

## 1. Contexto e Motivação
O workflow `.forgejo/workflows/ci.yml` executava `wrangler deploy`, upload direto para R2, rotação manual na Banca, trigger direto na Pratilera e SSH direto no `obizero`. Com o Kombinat, a CI publica todos os artefatos (site, binários e APKs) nos Generic Packages via `cccp package upload` e mantém apenas build/push da imagem Docker para a Gambi.
