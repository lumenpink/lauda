# Especificação Técnica: Badges Cripto/Assinatura e Higienização de Preâmbulo do Ishcondidim

- **Issue:** #29 (`feat(crypto): badges cripto/assinatura e higienização de preâmbulo do Ishcondidim`)
- **Status:** Especificado (Fase 3 DD-TDD)
- **Data:** 2026-10-12

---

## 1. Contexto & Problema
Quando e-mails cifrados ou assinados via PGP/MIME ou Autocrypt passam pelo daemon de segurança Ishcondidim (milter/gateway), ele decifra o conteúdo e injeta:
1. Cabeçalhos informativos:
   - `X-Ixcondidim-Decrypted: true; verified=pass` (ou `fail` / `none`)
   - `X-Ixcondidim-Status: decrypted-and-verified` (ou similar)
2. Banners visuais no topo do corpo da mensagem:
   - Plain text: moldura ASCII iniciando em `┌── 🔒 [Ixcondidim: Mensagem PGP Decifrada pelo Servidor]` e terminando em `└───...`.
   - HTML: bloco `<div ...>` verde contendo `🔒 [Ixcondidim: Mensagem PGP Decifrada pelo Servidor]`, tabela com Remetente, Assinatura, ID da Chave e opcionalmente Palavra Secreta.

Esses banners poluem a leitura do leitor smolclient Lauda, que possui estética brutalista minimalista (Trash Polka). A meta é ocultar/higienizar automaticamente esses preâmbulos dos corpos (`txt_auto`, `txt_original`, `html_sanitized`) e extrair os dados para apresentação compacta na listagem da caixa de entrada e um painel/gaveta colapsável técnica no leitor de mensagens.

---

## 2. Modelagem de Dados (`lauda-types`)

### 2.1 Enums e Estruturas
Em `lauda-types/src/message.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SignatureStatus {
    #[default]
    None,
    Valid,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CryptoMeta {
    pub is_encrypted: bool,
    pub signature_status: SignatureStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_sender: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_word: Option<String>,
}
```

### 2.2 Alterações em `MessageEnvelope` e `Message`
Em `MessageEnvelope`:
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub crypto_meta: Option<CryptoMeta>,
```

Em `Message`:
Herda automaticamente de `MessageEnvelope` via `#[serde(flatten)] pub envelope: MessageEnvelope`.

---

## 3. Lógica de Higienização e Extração (`clean_security_preambles`)

### 3.1 Higienização do Texto Puro
Função pura `sanitize_text_body(text: &str) -> (String, Option<CryptoMeta>)`:
- Detecta bloco que inicia em `┌── 🔒 [I[sx]condidim:` até `└──────...`.
- Extrai campos:
  - Remetente: linha `│ Remetente: <val>`
  - Assinatura: `VÁLIDA` -> `SignatureStatus::Valid`, `INVÁLIDA` -> `SignatureStatus::Invalid`, `Não assinada` -> `SignatureStatus::None`.
  - ID da Chave: linha `│ ID da Chave: <val>`
  - Palavra Secreta: linha `│ Palavra Secreta de Validação: [<val>]`
- Remove o bloco do texto e apara quebras de linha iniciais remanescentes.

### 3.2 Higienização de HTML
Função pura `sanitize_html_body(html: &str) -> (String, Option<CryptoMeta>)`:
- Detecta bloco `<div style="border-left: 4px solid ...>...🔒 [I[sx]condidim:...</div>`.
- Extrai campos correspondentes caso ainda não extraídos.
- Remove a `<div>` do banner, preservando o corpo HTML original intacto.

### 3.3 Detecção via Cabeçalhos RFC 5322
No `lauda-server` (`maildir.rs`):
- Analisa cabeçalhos `X-Ixcondidim-Decrypted`, `X-Ixcondidim-Status`, `X-GPG-Status`, `Autocrypt`:
  - Se `X-Ixcondidim-Decrypted` contiver `true`: `is_encrypted = true`.
  - Se contiver `verified=pass`: `signature_status = SignatureStatus::Valid`.
  - Se contiver `verified=fail`: `signature_status = SignatureStatus::Invalid`.
- Combina metadados dos cabeçalhos com metadados extraídos do corpo da mensagem.
- Higieniza `txt_auto`, `txt_original` e `html_sanitized` antes de retornar `Message`.

---

## 4. Apresentação Visual no Cliente (`lauda-client`)

### 4.1 Lista da Caixa de Entrada (`inbox_view.rs`)
Ao lado do remetente e data:
- Se `crypto_meta.is_encrypted`: exibe badge `[🔒]` com fundo escuro e borda fina.
- Se `crypto_meta.signature_status == SignatureStatus::Valid`: exibe badge `[✓ PGP]` em verde brutalista (`#047857`).
- Se `crypto_meta.signature_status == SignatureStatus::Invalid`: exibe badge `[⚠ INVÁLIDA]` em vermelho (`#dc143c`).

### 4.2 Visualizador da Mensagem (`message_view.rs`)
- No cabeçalho da mensagem, se houver `crypto_meta`, renderiza badge clicável com estado expandido/recolhido (gaveta colapsável).
- Ao expandir a gaveta:
  - Exibe caixa com borda grossa e sombra sólida:
    - Remetente verificado
    - ID da chave OpenPGP / fingerprint
    - Status técnico da assinatura
    - Palavra Secreta de autenticidade (se presente)
- O corpo da mensagem (`txt_auto`, `txt_original`, `html`) é exibido limpo, sem o banner do Ishcondidim.

---

## 5. Critérios de Aceite (Quality Gate)
1. `cargo test -j 2 --workspace` passa com 100% de sucesso.
2. Mensagens com preâmbulo Ishcondidim em texto e HTML têm seus preâmbulos removidos sem truncar o corpo legítimo do e-mail.
3. Badges compactos aparecem na lista da inbox conforme o status criptográfico.
4. Gaveta de detalhes criptográficos abre e fecha reativamente no leitor.
5. `cargo clippy --workspace --all-targets -- -D warnings` sem qualquer erro.
