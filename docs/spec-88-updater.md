# Especificação Técnica: Auto-Atualizador & Distribuição na Banca (Ticket #88)

## 1. Contexto e Motivação
Em conformidade com a norma GOST v1.5.0, a compilação de binários nativos de release e desenvolvimento é restrita à esteira sequencial (Forgejo Runner). A distribuição oficial ocorre através da vitrine soberana da Banca (`https://banca.2lp.in/downloads/lauda/`), com retenção estrita de 5 versões estáveis e 10 compilações de desenvolvimento (nightly).
O binário do cliente `lauda` passa a incorporar funcionalidade nativa de auto-atualização atômica e verificação de novas versões, consumindo diretamente esses canais.

## 2. Endpoints Soberanos da Banca
O módulo de atualização resolve os seguintes endpoints:
- **Canal Estável (Produção)**:
  - Binário mais recente: `https://banca.2lp.in/downloads/lauda/stable-latest/lauda`
  - Identificador de versão: `https://banca.2lp.in/downloads/lauda/version`
- **Canal Nightly (Desenvolvimento)**:
  - Binário mais recente: `https://banca.2lp.in/downloads/lauda/nightly-latest/lauda`
  - Identificador de versão: `https://banca.2lp.in/downloads/lauda/nightly-version`
- **Versão Fixada (Pin)**:
  - `https://banca.2lp.in/downloads/lauda/releases/v{pin}/lauda`

Nenhum fallback para releases do Codeberg ou GitHub é executado, mantendo a soberania da infraestrutura.

## 3. Persistência de Configuração e Canal Padrão
Para permitir que o usuário defina seu canal de preferência (por exemplo, `nightly` para receber compilações frequentes ou `stable` para lançamentos validados):
- **Arquivo de Configuração**: `$XDG_CONFIG_HOME/lauda/config.toml` (fallback `~/.config/lauda/config.toml`).
  ```toml
  [update]
  channel = "nightly" # ou "stable"
  ```
- **Gravação do Canal**:
  - `lauda update --channel nightly --save` grava `channel = "nightly"` no arquivo de configuração do usuário.
  - `lauda config set-channel <stable|nightly>` permite configurar diretamente o canal padrão.
- **Ordem de Precedência**:
  1. Flag de linha de comando: `--channel <stable|nightly>`
  2. Variável de ambiente: `LAUDA_CHANNEL`
  3. Arquivo de configuração do usuário: `~/.config/lauda/config.toml`
  4. Valor padrão de fallback: `"stable"`

## 4. Comandos e Interface da CLI
O `lauda` adiciona os seguintes subcomandos:
- `lauda update`:
  - `--channel <stable|nightly>`: Seleciona temporariamente o canal de atualização.
  - `--save`: Salva o canal informado como o padrão do usuário.
  - `--pin <versão>`: Instala ou trava em uma versão estável específica.
  - `--check`: Apenas checa se há atualização disponível sem baixar.
  - `--target <caminho>`: Sobrescreve o caminho do executável de destino.
- `lauda check-update`:
  - `--channel <stable|nightly>`: Checa disponibilidade no canal especificado.
- `lauda config set-channel <canal>` / `lauda config show`:
  - Permite definir e consultar chaves de configuração.

## 5. Algoritmo de Substituição Atômica
1. O executável determina o caminho do binário atual via `std::env::current_exe()`.
2. O novo binário é baixado em um arquivo temporário no mesmo sistema de arquivos (`<current_exe>.tmp.<random>`).
3. As permissões de execução (`chmod 0o755`) são atribuídas no arquivo temporário.
4. É executada uma substituição atômica via `std::fs::rename`.

## 6. Pipeline de Entrega na Esteira
No workflow de CI (`.forgejo/workflows/ci.yml`), a etapa de build publica os artefatos compilados na rota `/srv/banca/downloads/lauda/` e no R2 da Banca, atualizando os links `stable-latest` ou `nightly-latest` e o arquivo `version` correspondente.
