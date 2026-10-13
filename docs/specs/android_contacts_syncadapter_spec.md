# Especificação Técnica de Contratos: Android SyncAdapter e Normalizador Telefônico Soberano

> **Documento de Especificação e Contrato Preliminar (Fase 3 - DD-TDD GOST)**  
> **Ticket Vinculado:** [#35 - feat(contacts): android syncadapter e normalizador telefônico soberano](https://codeberg.org/lumenpink/lauda/issues/35)  
> **Norma Base:** GOST 1.4.0 / RFC 6350 (vCard 4.0) / RFC 6578 (Sync-Collection) / ITU-T E.164 / Resolução ANATEL nº 553/2010

---

## 1. Visão Geral da Arquitetura

O `lauda-client` atua como um cidadão de primeira classe no sistema operacional Android, registrando uma conta oficial de usuário gerenciada (`pink.lumen.lauda`) e um serviço de sincronização bidirecional (`SyncAdapter`) que conecta o `ContactsProvider` nativo do Android ao repositório incremental do `lauda-server`.

Adicionalmente, incorpora um motor soberano de normalização e higienização telefônica focado nas idiossincrasias do plano de numeração brasileiro (E.164 / ANATEL) e suporte a aniversários (`BDAY`).

```
                ┌──────────────────────────────────────────────┐
                │          lauda-server (Rust / Linux)         │
                │        SQLite Contacts Store + CardDAV       │
                └──────────────────────┬───────────────────────┘
                                       │ REST /contacts/sync
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Android Device (Obtainium)                          │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                    lauda-types / lauda-client (Rust)                  │  │
│  │  • PhoneNormalizer: E.164 BR (9º dígito, CSP strip, DDD fallback)     │  │
│  │  • ContactEntry Mapper: vCard 4.0 (BDAY, FN, TEL, EMAIL, ORG)         │  │
│  │  • DeduplicationEngine: Similaridade fuzzy e merge assistido          │  │
│  └───────────────────────────────────┬───────────────────────────────────┘  │
│                                      │ JNI C-ABI Bridge                     │
│  ┌───────────────────────────────────▼───────────────────────────────────┐  │
│  │       Android SyncAdapter & AccountManager (pink.lumen.lauda)         │  │
│  │   (LaudaAuthenticatorService, LaudaSyncAdapter, syncadapter.xml)      │  │
│  └───────────────────────────────────┬───────────────────────────────────┘  │
│                                      │ ContentProviderOperation.applyBatch  │
│  ┌───────────────────────────────────▼───────────────────────────────────┐  │
│  │                Android Native ContactsProvider                        │  │
│  │     (Discador Nativo, WhatsApp, Signal, Notificações do SO)           │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Metadados e Componentes Nativos do Android

### 2.1 Identificador de Conta (`AccountType`)
- **Tipo de Conta:** `pink.lumen.lauda`
- **Nome da Conta:** E-mail do usuário autenticado (ex: `hi@lumen.pink` ou `lumen`)
- **Autoridade Vinculada:** `com.android.contacts`

### 2.2 Estrutura Declarativa de Recursos
- **`res/xml/authenticator.xml`**:
  ```xml
  <account-authenticator xmlns:android="http://schemas.android.com/apk/res/android"
      android:accountType="pink.lumen.lauda"
      android:icon="@mipmap/ic_launcher"
      android:smallIcon="@mipmap/ic_launcher"
      android:label="@string/app_name" />
  ```
- **`res/xml/syncadapter.xml`**:
  ```xml
  <sync-adapter xmlns:android="http://schemas.android.com/apk/res/android"
      android:contentAuthority="com.android.contacts"
      android:accountType="pink.lumen.lauda"
      android:userVisible="true"
      android:supportsUploading="true"
      android:allowParallelSyncs="false"
      android:isAlwaysSyncable="true" />
  ```

---

## 3. Mapeamento de Contratos (`ContactEntry` ↔ `ContactsContract`)

Cada contato Lauda gera um registro correspondente em `RawContacts` marcado com `account_type = "pink.lumen.lauda"` e `account_name = "{username}"`.

| Campo `ContactEntry` | Tabela / MimeType Android | Coluna Android | Observações / Regra |
| :--- | :--- | :--- | :--- |
| `id` (UUID) | `RawContacts.SOURCE_ID` | `sourceid` | Chave de reconciliação soberana |
| `name` / `given_name` | `StructuredName.CONTENT_ITEM_TYPE` | `DISPLAY_NAME`, `GIVEN_NAME`, `FAMILY_NAME` | Separado heuristicamente |
| `email` | `Email.CONTENT_ITEM_TYPE` | `DATA1`, `TYPE_WORK` / `TYPE_HOME` | Preservado íntegro |
| `phone` | `Phone.CONTENT_ITEM_TYPE` | `DATA1`, `DATA4` (Normalized) | Higienizado em E.164 pelo motor |
| `birthday` (`BDAY`) | `Event.CONTENT_ITEM_TYPE` | `DATA1` (`YYYY-MM-DD` ou `--MM-DD`), `TYPE_BIRTHDAY` | RFC 6350 BDAY |
| `organization` | `Organization.CONTENT_ITEM_TYPE` | `COMPANY`, `TITLE` | Opcional |
| `notes` | `Note.CONTENT_ITEM_TYPE` | `NOTE` | Observações |

---

## 4. Motor Soberano de Higienização Telefônica Brasileira (`PhoneNormalizer`)

O plano de numeração brasileiro (Resolução ANATEL nº 553/2010 e padrão ITU-T E.164) apresenta desafios específicos que este motor resolve de forma hermética:

### 4.1 Regras de Normalização:
1. **Remoção de Código de Seleção de Prestadora (CSP):**
   - Prefixos comuns discados no Brasil como `0XX` (ex: `014`, `015`, `021`, `031`, `041`, `043`, `099`) antes do DDD são estritados:
   - Exemplo: `021 (48) 99123-4567` → detecta CSP `21`, DDD `48`, número `991234567`.
2. **Inclusão do 9º Dígito Móvel (ANATEL):**
   - Telefones móveis (celulares) iniciam com dígitos `6`, `7`, `8` ou `9`.
   - Se um celular brasileiro tiver apenas 8 dígitos (ex: `48 9123-4567` ou `11 8123-4567`), o dígito `9` é prefixado automaticamente: `48 99123-4567`.
   - Telefones fixos iniciam com dígitos `2`, `3`, `4` ou `5` e mantêm estritamente 8 dígitos.
3. **Formatação E.164 Canônica:**
   - Todo número brasileiro válido é normalizado para `+55` + `DDD (2 dígitos)` + `Número (8 ou 9 dígitos)`:
   - Exemplo canônico: `+5548991234567`.
4. **Formatação Visual Canônica:**
   - Formato de exibição: `+55 (48) 99123-4567` ou `+55 (48) 3222-1234`.

---

## 5. Algoritmo de Deduplicação e Scoring

Para evitar contatos duplicados entre a agenda do celular e a conta Lauda:
1. **Match Exato de Identificadores:**
   - Mesma chave E.164 telefônica OU mesmo endereço de e-mail limpo em minúsculas → Score 1.0 (mesmo contato).
2. **Match Fonético e Fuzzy de Nomes:**
   - Levenshtein normalizado no nome completo combinado com similaridade de tokens.
   - Score $\ge 0.85$ dispara sugestão de mescla de dados sem perda de atributos.

---

## 6. Critérios de Aceite e Fases TDD
- **Testes Unitários:** Suíte completa em `lauda-types` cobrindo o `PhoneNormalizer` (todos os casos de borda: fixo, celular sem 9º dígito, com CSP 021/015, DDI internacional pré-existente, sem DDD com fallback configurável).
- **Testes de Contratos:** Serialização e deserialização de `ContactEntry` com `birthday` (RFC 6350 `BDAY`).
- **Testes de Operações:** Gerador de lotes `ContentProviderOperation` mockado para validação do SyncAdapter.
- **Zero Warnings:** Clippy e formatação rigorosamente validados.
