use serde::{Deserialize, Serialize};
use crate::MessageEnvelope;

/// Campo de verificação para a condição da regra Sieve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionField {
    From,
    To,
    Subject,
    ListId,
    Custom(String),
}

impl ConditionField {
    pub fn header_name(&self) -> &str {
        match self {
            Self::From => "from",
            Self::To => "to",
            Self::Subject => "subject",
            Self::ListId => "list-id",
            Self::Custom(name) => name.as_str(),
        }
    }

    pub fn from_header(header: &str) -> Self {
        match header.to_lowercase().as_str() {
            "from" => Self::From,
            "to" => Self::To,
            "subject" => Self::Subject,
            "list-id" => Self::ListId,
            other => Self::Custom(other.to_string()),
        }
    }
}

/// Tipo de casamento textual da condição.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchType {
    Contains,
    Is,
    Matches,
}

impl MatchType {
    pub fn modifier(&self) -> &'static str {
        match self {
            Self::Contains => ":contains",
            Self::Is => ":is",
            Self::Matches => ":matches",
        }
    }

    pub fn from_modifier(modifier: &str) -> Self {
        match modifier {
            ":is" => Self::Is,
            ":matches" => Self::Matches,
            _ => Self::Contains,
        }
    }
}

/// Condição de filtragem Sieve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SieveCondition {
    pub field: ConditionField,
    pub match_type: MatchType,
    pub value: String,
}

/// Ação executada quando a regra casa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SieveAction {
    FileInto(String),
    MarkRead,
    Discard,
    Keep,
    Stop,
}

/// Representação estruturada de uma regra de correio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SieveRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub match_all: bool,
    pub conditions: Vec<SieveCondition>,
    pub actions: Vec<SieveAction>,
}

/// Pacote completo contendo o script textual RFC 5228 e a lista parseada de regras.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SieveScriptBundle {
    pub raw_script: String,
    pub rules: Vec<SieveRule>,
}

/// Gera um script Sieve válido (RFC 5228 / Dovecot Pigeonhole) a partir de uma lista de regras.
pub fn generate_sieve_script(rules: &[SieveRule]) -> String {
    let mut script = String::new();
    script.push_str("require [\"fileinto\", \"imap4flags\"];\n\n");

    for rule in rules {
        if !rule.enabled {
            script.push_str(&format!("# disabled-rule:[{}]\n", rule.name));
            continue;
        }

        script.push_str(&format!("# rule:[{}]\n", rule.name));

        if rule.conditions.is_empty() {
            // Sem condições específicas (catch-all)
            script.push_str("if true {\n");
        } else if rule.conditions.len() == 1 {
            let cond = &rule.conditions[0];
            script.push_str(&format!(
                "if header {} \"{}\" \"{}\" {{\n",
                cond.match_type.modifier(),
                cond.field.header_name(),
                escape_sieve_string(&cond.value)
            ));
        } else {
            let combiner = if rule.match_all { "allof" } else { "anyof" };
            script.push_str(&format!("if {} (\n", combiner));
            for (idx, cond) in rule.conditions.iter().enumerate() {
                let comma = if idx + 1 < rule.conditions.len() { "," } else { "" };
                script.push_str(&format!(
                    "    header {} \"{}\" \"{}\"{}\n",
                    cond.match_type.modifier(),
                    cond.field.header_name(),
                    escape_sieve_string(&cond.value),
                    comma
                ));
            }
            script.push_str(") {\n");
        }

        for action in &rule.actions {
            match action {
                SieveAction::FileInto(folder) => {
                    script.push_str(&format!("    fileinto \"{}\";\n", escape_sieve_string(folder)));
                }
                SieveAction::MarkRead => {
                    script.push_str("    setflag \"\\\\Seen\";\n");
                }
                SieveAction::Discard => {
                    script.push_str("    discard;\n");
                }
                SieveAction::Keep => {
                    script.push_str("    keep;\n");
                }
                SieveAction::Stop => {
                    script.push_str("    stop;\n");
                }
            }
        }

        script.push_str("}\n\n");
    }

    script
}

/// Analisa um script Sieve e extrai as regras anotadas com `# rule:[Nome]`.
pub fn parse_sieve_script(script: &str) -> Vec<SieveRule> {
    let mut rules = Vec::new();
    let lines: Vec<&str> = script.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("# rule:[") && line.ends_with(']') {
            let name = &line[8..line.len() - 1];
            let id = format!("rule_{}", rules.len() + 1);
            let mut conditions = Vec::new();
            let mut actions = Vec::new();
            let mut match_all = true;

            i += 1;
            // Parse do bloco de condicao
            while i < lines.len() {
                let l = lines[i].trim();
                if l.contains("anyof") {
                    match_all = false;
                } else if l.contains("allof") {
                    match_all = true;
                }

                if l.contains("header") {
                    if let Some(cond) = parse_header_condition(l) {
                        conditions.push(cond);
                    }
                }

                if l.ends_with('{') {
                    i += 1;
                    break;
                }
                i += 1;
            }

            // Parse das acoes no corpo do bloco
            while i < lines.len() {
                let l = lines[i].trim();
                if l.starts_with('}') {
                    break;
                }

                if l.starts_with("fileinto") {
                    if let Some(folder) = extract_quoted_value(l) {
                        actions.push(SieveAction::FileInto(folder));
                    }
                } else if l.contains("\\\\Seen") || l.contains("\\Seen") {
                    actions.push(SieveAction::MarkRead);
                } else if l.starts_with("discard;") {
                    actions.push(SieveAction::Discard);
                } else if l.starts_with("keep;") {
                    actions.push(SieveAction::Keep);
                } else if l.starts_with("stop;") {
                    actions.push(SieveAction::Stop);
                }
                i += 1;
            }

            rules.push(SieveRule {
                id,
                name: name.to_string(),
                enabled: true,
                match_all,
                conditions,
                actions,
            });
        }
        i += 1;
    }

    rules
}

/// Infere regra preditiva inteligente a partir de um envelope de mensagem.
pub fn predict_rule_from_envelope(env: &MessageEnvelope) -> SieveRule {
    // 1. Verificar prefixo entre colchetes no assunto (ex: "[lauda-dev]")
    let trimmed_subject = env.subject.trim();
    if trimmed_subject.starts_with('[')
        && let Some(closing_idx) = trimmed_subject.find(']')
    {
        let tag = &trimmed_subject[1..closing_idx];
        let full_tag = &trimmed_subject[0..=closing_idx];
        return SieveRule {
            id: format!("pred_{}", env.uid),
            name: format!("Regra para [{}]", tag),
            enabled: true,
            match_all: true,
            conditions: vec![SieveCondition {
                field: ConditionField::Subject,
                match_type: MatchType::Contains,
                value: full_tag.to_string(),
            }],
            actions: vec![
                SieveAction::FileInto(format!("Listas/{}", tag)),
                SieveAction::Stop,
            ],
        };
    }

    // 2. Fallback: inferir a partir do remetente
    let sender_clean = if let Some(start) = env.sender.find('<')
        && let Some(end) = env.sender.find('>')
    {
        &env.sender[start + 1..end]
    } else {
        env.sender.trim()
    };

    let user_part = sender_clean.split('@').next().unwrap_or("Contato");

    SieveRule {
        id: format!("pred_{}", env.uid),
        name: format!("Regra para {}", sender_clean),
        enabled: true,
        match_all: true,
        conditions: vec![SieveCondition {
            field: ConditionField::From,
            match_type: MatchType::Contains,
            value: sender_clean.to_string(),
        }],
        actions: vec![
            SieveAction::FileInto(format!("Contatos/{}", user_part)),
            SieveAction::Stop,
        ],
    }
}

fn escape_sieve_string(val: &str) -> String {
    val.replace('\\', "\\\\").replace('\"', "\\\"")
}

fn extract_quoted_value(line: &str) -> Option<String> {
    let start = line.find('\"')?;
    let rest = &line[start + 1..];
    let end = rest.find('\"')?;
    Some(rest[..end].to_string())
}

fn parse_header_condition(line: &str) -> Option<SieveCondition> {
    // Exemplo: header :contains "from" "news@example.com"
    let modifier = if line.contains(":is") {
        ":is"
    } else if line.contains(":matches") {
        ":matches"
    } else {
        ":contains"
    };

    let quotes: Vec<usize> = line.match_indices('\"').map(|(i, _)| i).collect();
    if quotes.len() >= 4 {
        let field_name = &line[quotes[0] + 1..quotes[1]];
        let val = &line[quotes[2] + 1..quotes[3]];
        return Some(SieveCondition {
            field: ConditionField::from_header(field_name),
            match_type: MatchType::from_modifier(modifier),
            value: val.to_string(),
        });
    }

    None
}
