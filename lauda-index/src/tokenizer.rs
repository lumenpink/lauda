/// Normaliza uma string para indexação ou busca:
/// converte para minúsculas e remove diacríticos (acentos) comuns em português.
pub fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c.to_ascii_lowercase() {
            'á' | 'à' | 'ã' | 'â' | 'ä' => out.push('a'),
            'é' | 'è' | 'ê' | 'ë' => out.push('e'),
            'í' | 'ì' | 'î' | 'ï' => out.push('i'),
            'ó' | 'ò' | 'õ' | 'ô' | 'ö' => out.push('o'),
            'ú' | 'ù' | 'û' | 'ü' => out.push('u'),
            'ç' => out.push('c'),
            other => out.push(other),
        }
    }
    out
}

/// Tokeniza o texto em palavras limpas (alfanuméricas).
pub fn tokenize(input: &str) -> Vec<String> {
    let norm = normalize(input);
    let mut tokens = Vec::new();
    let mut current = String::new();

    for c in norm.chars() {
        if c.is_alphanumeric() {
            current.push(c);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalization_and_tokenization() {
        let text = "Atenção: Reunião de Orçamento & Planejamento!";
        let tokens = tokenize(text);
        assert_eq!(
            tokens,
            vec!["atencao", "reuniao", "de", "orcamento", "planejamento"]
        );
    }
}
