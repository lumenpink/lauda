use crate::tokenizer::normalize;

/// Representa uma cláusula parsed de busca estilo Gmail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryClause {
    /// Termo livre para busca textual no assunto, remetente e corpo
    Term(String),
    /// Filtro de remetente: from:nome_ou_email
    From(String),
    /// Filtro de destinatário: to:nome_ou_email
    To(String),
    /// Filtro de assunto: subject:termo
    Subject(String),
    /// Filtro de anexo: has:attachment
    HasAttachment,
    /// Filtro de leitura: is:read | is:unread
    IsRead(bool),
    /// Filtro de favorito: is:flagged
    IsFlagged(bool),
    /// Negação: -termo ou NOT termo
    Not(Box<QueryClause>),
    /// União: cláusula1 OR cláusula2
    Or(Box<QueryClause>, Box<QueryClause>),
}

/// Consulta composta parsed. Todas as cláusulas no vetor de topo têm relação AND implícita.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Query {
    pub clauses: Vec<QueryClause>,
}

/// Parser de consultas em formato texto inspirado na barra de busca do Gmail.
pub fn parse_query(input: &str) -> Query {
    let mut clauses = Vec::new();
    let parts = split_query_tokens(input);

    let mut i = 0;
    while i < parts.len() {
        let token = &parts[i];

        if token.eq_ignore_ascii_case("OR") {
            // Se houver termo anterior e próximo, agrupa em Or
            if let Some(prev) = clauses.pop() {
                if i + 1 < parts.len() {
                    let next_token = &parts[i + 1];
                    let next_clause = parse_single_clause(next_token);
                    clauses.push(QueryClause::Or(Box::new(prev), Box::new(next_clause)));
                    i += 2;
                    continue;
                } else {
                    clauses.push(prev);
                }
            }
        } else {
            clauses.push(parse_single_clause(token));
        }
        i += 1;
    }

    Query { clauses }
}

fn parse_single_clause(token: &str) -> QueryClause {
    if let Some(stripped) = token.strip_prefix('-') {
        return QueryClause::Not(Box::new(parse_single_clause(stripped)));
    }

    if let Some((prefix, value)) = token.split_once(':') {
        let p_clean = prefix.to_ascii_lowercase();
        let val_clean = normalize(value.trim_matches('"'));
        match p_clean.as_str() {
            "from" => QueryClause::From(val_clean),
            "to" => QueryClause::To(val_clean),
            "subject" => QueryClause::Subject(val_clean),
            "has" => {
                if val_clean == "attachment" {
                    QueryClause::HasAttachment
                } else {
                    QueryClause::Term(val_clean)
                }
            }
            "is" => match val_clean.as_str() {
                "read" => QueryClause::IsRead(true),
                "unread" => QueryClause::IsRead(false),
                "flagged" | "starred" => QueryClause::IsFlagged(true),
                _ => QueryClause::Term(val_clean),
            },
            _ => QueryClause::Term(normalize(token)),
        }
    } else {
        QueryClause::Term(normalize(token.trim_matches('"')))
    }
}

/// Divide a query respeitando aspas para termos compostos (ex: subject:"reuniao importante")
fn split_query_tokens(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in input.chars() {
        if c == '"' {
            in_quotes = !in_quotes;
            current.push(c);
        } else if c.is_whitespace() && !in_quotes {
            if !current.trim().is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gmail_style_parser() {
        let q = parse_query(r#"from:alice subject:"relatorio mensal" is:unread -spam"#);
        assert_eq!(
            q.clauses,
            vec![
                QueryClause::From("alice".to_string()),
                QueryClause::Subject("relatorio mensal".to_string()),
                QueryClause::IsRead(false),
                QueryClause::Not(Box::new(QueryClause::Term("spam".to_string()))),
            ]
        );
    }

    #[test]
    fn test_or_operator() {
        let q = parse_query("fatura OR boleto");
        assert_eq!(
            q.clauses,
            vec![QueryClause::Or(
                Box::new(QueryClause::Term("fatura".to_string())),
                Box::new(QueryClause::Term("boleto".to_string()))
            )]
        );
    }
}
