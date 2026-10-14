# Especificação: Busca Heurística e Vinculação de Contatos no Calendário

- **Ticket:** [#51](https://codeberg.org/lumenpink/lauda/issues/51)
- **Status:** Aprovado / Em Implementação
- **Data:** 2026-10-14
- **Autor:** Lumen Pink / Antigravity

---

## 1. Visão Geral & Escopo

Esta especificação define o mecanismo de busca heurística (fuzzy matching, tolerância a typos, insensibilidade a acentos e maiúsculas) sobre a base local de contatos (`LocalContactsCache`), permitindo a vinculação rápida e sem atrito de pessoas a eventos do calendário do Lauda.

O fluxo cobre tanto o modo de entrada em linguagem natural na agenda quanto a edição de participantes em compromissos, garantindo fluidez tanto em interfaces desktop (teclado) quanto móveis (toque/touch).

---

## 2. Algoritmo de Busca Heurística & Similaridade Léxica

### 2.1. Normalização de Texto
Antes do cálculo de proximidade, os termos de busca e os campos do contato (`display_name`, `primary_email`, `secondary_emails`, `username`) passam por:
1. **Case-folding:** Conversão integral para minúsculas (`to_lowercase()`).
2. **Remoção de Acentos e Diacríticos:** Mapeamento de caracteres acentuados latinos para equivalentes básicos (ex.: `á, à, ã, â` $\to$ `a`, `é, ê` $\to$ `e`, `í` $\to$ `i`, `ó, õ, ô` $\to$ `o`, `ú, ü` $\to$ `u`, `ç` $\to$ `c`).
3. **Trim e limpeza de pontuações redundantes.**

### 2.2. Categorias de Match & Ranking de Relevância
Cada contato recebe uma pontuação (`score`) de 0 a 100:
1. **Match Exato (Score: 100):** O termo pesquisado é idêntico ao nome normalizado, ao apelido ou ao prefixo de e-mail.
2. **Prefixo de Token (Score: 80):** O termo é prefixo de qualquer palavra contida no nome (ex.: `mar` em `Marcelo Silva`).
3. **Substring Contida (Score: 60):** O termo ocorre como subsequência contígua no nome ou no endereço de e-mail.
4. **Fuzzy Match com Tolerância a Typos (Score: 30 a 50):**
   - Baseado na distância de Levenshtein entre o termo e as palavras do nome/e-mail.
   - Distância $= 1$ para palavras de comprimento $\ge 4$: Score 50.
   - Distância $= 2$ para palavras de comprimento $\ge 6$: Score 35.
   - Distâncias maiores ou termos muito curtos ($< 3$ caracteres com erro) são descartados (Score 0).

A ordenação final é estritamente decrescente por `score`, com desempate alfabético por `display_name`.

---

## 3. Contratos de Dados & API

### 3.1. Busca no `LocalContactsCache`
```rust
impl LocalContactsCache {
    /// Executa busca heurística ponderada sobre os contatos ativos,
    /// retornando os melhores resultados até o limite especificado.
    pub fn search_contacts_fuzzy(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ContactEntry>, ClientError>;
}
```

### 3.2. Vinculação ao `EventAttendee`
Ao selecionar um contato:
- `attendee.name = Some(contact.display_name.clone().unwrap_or(contact.primary_email.clone()))`
- `attendee.email = contact.primary_email.clone()`
- Caso o contato possua chave Autocrypt, preserva-se a capacidade de envio seguro e verificação de chave.

---

## 4. Comportamento e Ergonomia da Interface (UX)

### 4.1. Posicionamento do Dropdown
- O container de sugestões (`div.contact-suggestions-dropdown`) abre imediatamente abaixo do campo de entrada ou do token de nome em edição.
- Visual neobrutalista: fundo branco, borda sólida preta `2px solid #0a0a0a`, sombra deslocada `box-shadow: 4px 4px 0 #0a0a0a`, z-index elevado.

### 4.2. Navegação & Interações
1. **Setas Teclado (`ArrowDown` / `ArrowUp`):**
   - Percorrem a lista de sugestões incrementando/decrementando o índice ativo (`active_index`).
   - O item focado recebe destaque com fundo invertido ou borda destacada (`background: #0a0a0a; color: #fff;`).
2. **Enter:**
   - Seleciona o contato ativo em foco, preenchendo o nome e associando o e-mail oficial à entrada do calendário. Fecha o dropdown.
3. **Toque / Rolagem com o Dedo (Touch & Mousewheel):**
   - O container possui `overflow-y: auto` e `max-height: 200px`, permitindo rolagem fluida e inercial com o dedo sem selecionar acidentalmente.
4. **Clique / Tap:**
   - O toque direto em qualquer item da lista seleciona o contato imediatamente.
5. **Espaço (`Space`) / Escape / Clique fora:**
   - Se o usuário pressionar a barra de espaço para continuar a digitação normal da frase (ex.: *"com Marcelo amanhã"*), a lista suspensa é ocultada imediatamente e o nome digitado é aceito como texto puro sem vinculação forçada.

---

## 5. Critérios de Aceite & Validação (DD-TDD)

1. **Unitário Heurístico:**
   - Buscar `"marcelo"` localiza `"Marcelo Silva"`.
   - Buscar `"débora"` localiza `"Debora Santos"` e vice-versa.
   - Buscar `"beatiz"` (typo) localiza `"Beatriz"`.
   - Buscar termo inexistente `"xyz123"` retorna lista vazia sem erros.
2. **Vinculação:**
   - A seleção do contato transforma um participante genérico sem e-mail em um `EventAttendee` com nome e `primary_email` persistidos no evento.
3. **Conformidade Gosplan:**
   - Zero warnings de compilação.
   - `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings` limpos.
