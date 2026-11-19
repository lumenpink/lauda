/// Normaliza uma string para indexação ou busca:
/// converte para minúsculas e remove diacríticos (acentos) comuns em português.
pub fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        for lower_c in c.to_lowercase() {
            match lower_c {
                'á' | 'à' | 'ã' | 'â' | 'ä' => out.push('a'),
                'é' | 'è' | 'ê' | 'ë' => out.push('e'),
                'í' | 'ì' | 'î' | 'ï' => out.push('i'),
                'ó' | 'ò' | 'õ' | 'ô' | 'ö' => out.push('o'),
                'ú' | 'ù' | 'û' | 'ü' => out.push('u'),
                'ç' => out.push('c'),
                other => out.push(other),
            }
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

    #[test]
    fn test_normalization_uppercase_diacritics() {
        assert_eq!(normalize("ATENÇÃO"), "atencao");
        assert_eq!(normalize("ORÇAMENTO"), "orcamento");
        assert_eq!(normalize("REUNIÃO"), "reuniao");
        assert_eq!(
            normalize("ÁÉÍÓÚ ÀÈÌÒÙ ÂÊÎÔÛ ÄËÏÖÜ ÃÕ Ç"),
            "aeiou aeiou aeiou aeiou ao c"
        );

        let tokens = tokenize("ATENÇÃO: REUNIÃO DE ORÇAMENTO & PLANEJAMENTO!");
        assert_eq!(
            tokens,
            vec!["atencao", "reuniao", "de", "orcamento", "planejamento"]
        );
    }
}
