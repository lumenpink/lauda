# Especificação Técnica: Placeholder Genérico para a Quick-Add Bar da Agenda

**ID do Ticket:** #57  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificado (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Contexto e Motivação

O campo de inserção em linha única na Agenda Soberana (`lauda-client/src/ui/calendar_view.rs`) atualmente utiliza um exemplo de texto fixo com referências a pessoas e locais específicos: `"Ex: Almoço com Marcelo amanhã 12h30 no Fasano"`.

Para garantir neutralidade, profissionalismo e maior clareza pedagógica aos novos usuários sobre como redigir comandos em linguagem natural, o texto de sugestão (placeholder) deve ser padronizado para um formato impessoal e universal.

---

## 2. Contrato de Interface e Comportamento

1. **Constante Canônica:**
   - Em `lauda-client/src/ui/calendar_view.rs`, o texto de sugestão padrão é definido pela constante pública:
     ```rust
     pub const QUICK_ADD_DEFAULT_PLACEHOLDER: &str = "Ex: Reunião de equipe amanhã às 14h na Sala 2";
     ```
2. **Exemplificação Trifocal:**
   - O texto exemplifica simultaneamente os três elementos essenciais que o parser de linguagem natural (`CalendarIntentParser`) extrai:
     - **Título/Compromisso:** "Reunião de equipe"
     - **Data/Horário:** "amanhã às 14h"
     - **Localização:** "na Sala 2"
3. **Consumo na Interface:**
   - O componente `<input>` da Quick-Add bar em `calendar_view.rs` consome diretamente `placeholder: "{QUICK_ADD_DEFAULT_PLACEHOLDER}"`.

---

## 3. Critérios de Aceite e Verificação

1. **Neutralidade e Clareza:** O placeholder não contém menção a nomes pessoais ou estabelecimentos comerciais específicos.
2. **Constante Dedicada:** O texto é centralizado em `QUICK_ADD_DEFAULT_PLACEHOLDER`.
3. **Cobertura de Testes:** Teste automatizado assegura a presença da constante, formato genérico e não-vazio.
4. **Qualidade e Estilo:** Conformidade com `cargo fmt -- --check`, `cargo clippy -- -D warnings` e `cccp check --strict`.
