# Spec: Desmistificação e Humanização de Copy - Lauda (#52)

## Contexto
O Lauda foi concebido com uma experiência limpa de correio pessoal. No entanto, a landing page (`site/index.html`) e a apresentação principal (`README.md`) continham excesso de jargões técnicos de infraestrutura ("hospedagem soberana", "HTTP Range", "correio soberano minimalista", "Rust puro", "WASM", "outbox assíncrona").

## Objetivo
Reorientar a narrativa para a experiência acolhedora e aconchegante de ler e escrever com tranquilidade, sem complicações ou ansiedade digital.

## Critérios de Aceite
1. O título e subtítulo do site e documentação devem destacar a leitura e escrita aconchegante, sem ruídos e sem jargões de infraestrutura.
2. Eliminar referências a "hospedagem soberana" e "HTTP Range" no manifesto e nos cartões de download.
3. Teste automatizado validando a ausência desses termos proibidos e a presença dos termos acolhedores em `site/index.html` e `README.md`.
