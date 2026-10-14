//! Módulo de busca heurística, normalização léxica e similaridade (fuzzy matching)
//! para contatos e participantes no lauda-client.

/// Normaliza uma string para comparação fuzzy:
/// 1. Converte diacríticos latinos comuns para seus equivalentes ASCII.
/// 2. Converte para minúsculas.
/// 3. Substitui pontuações pontuais por espaços e condensa múltiplos espaços.
pub fn normalize_for_fuzzy(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            'á' | 'à' | 'ã' | 'â' | 'ä' | 'å' | 'Á' | 'À' | 'Ã' | 'Â' | 'Ä' | 'Å' => {
                out.push('a')
            }
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => out.push('e'),
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => out.push('i'),
            'ó' | 'ò' | 'õ' | 'ô' | 'ö' | 'Ó' | 'Ò' | 'Õ' | 'Ô' | 'Ö' => out.push('o'),
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => out.push('u'),
            'ç' | 'Ç' => out.push('c'),
            'ñ' | 'Ñ' => out.push('n'),
            'ý' | 'ÿ' | 'Ý' => out.push('y'),
            c if c.is_alphanumeric() => {
                for lower in c.to_lowercase() {
                    out.push(lower);
                }
            }
            c if c.is_whitespace() || c == '-' || c == '_' || c == '.' || c == '@' => {
                out.push(' ');
            }
            _ => {
                // Outros caracteres especiais/pontuações são ignorados ou viram espaço
                out.push(' ');
            }
        }
    }

    // Condensar espaços repetidos e aparar bordas
    let parts: Vec<&str> = out.split_whitespace().collect();
    parts.join(" ")
}

/// Calcula a distância de Levenshtein entre duas strings em tempo O(M*N) e espaço O(min(M, N)).
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    let m = s1_chars.len();
    let n = s2_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Usar a menor string para alocar menos memória
    let (short, long, short_len) = if m <= n {
        (&s1_chars, &s2_chars, m)
    } else {
        (&s2_chars, &s1_chars, n)
    };

    let mut prev_row: Vec<usize> = (0..=short_len).collect();
    let mut curr_row: Vec<usize> = vec![0; short_len + 1];

    for (j, &c_long) in long.iter().enumerate() {
        curr_row[0] = j + 1;
        for (i, &c_short) in short.iter().enumerate() {
            let cost = if c_short == c_long { 0 } else { 1 };
            curr_row[i + 1] = std::cmp::min(
                std::cmp::min(
                    curr_row[i] + 1,     // inserção
                    prev_row[i + 1] + 1, // remoção
                ),
                prev_row[i] + cost, // substituição
            );
        }
        prev_row.copy_from_slice(&curr_row);
    }

    prev_row[short_len]
}

/// Calcula pontuação de relevância de busca (0 a 100) para um contato contra a consulta informada.
pub fn calculate_fuzzy_match_score(query: &str, display_name: &str, primary_email: &str) -> u32 {
    let q_norm = normalize_for_fuzzy(query);
    if q_norm.is_empty() {
        return 0;
    }

    let name_norm = normalize_for_fuzzy(display_name);
    let email_norm = normalize_for_fuzzy(primary_email);

    let mut best_score: u32 = 0;

    // 1. Match exato com nome ou e-mail
    if name_norm == q_norm || email_norm == q_norm {
        return 100;
    }

    // 2. Prefixo direto
    if name_norm.starts_with(&q_norm) || email_norm.starts_with(&q_norm) {
        best_score = std::cmp::max(best_score, 85);
    }

    // 3. Match de tokens / palavras individuais
    let query_tokens: Vec<&str> = q_norm.split_whitespace().collect();
    let target_tokens: Vec<&str> = name_norm
        .split_whitespace()
        .chain(email_norm.split_whitespace())
        .collect();

    for &q_token in &query_tokens {
        for &t_token in &target_tokens {
            if t_token == q_token {
                best_score = std::cmp::max(best_score, 90);
            } else if t_token.starts_with(q_token) {
                best_score = std::cmp::max(best_score, 80);
            } else if t_token.contains(q_token) {
                best_score = std::cmp::max(best_score, 65);
            } else {
                // Fuzzy por distância de Levenshtein para palavras de tamanho razoável
                let q_len = q_token.chars().count();
                let t_len = t_token.chars().count();
                let dist = levenshtein_distance(q_token, t_token);

                if dist == 1 && q_len >= 4 && t_len >= 4 {
                    best_score = std::cmp::max(best_score, 50);
                } else if dist == 2 && q_len >= 6 && t_len >= 6 {
                    best_score = std::cmp::max(best_score, 35);
                }
            }
        }
    }

    // 4. Substring geral no nome ou e-mail
    if best_score == 0 && (name_norm.contains(&q_norm) || email_norm.contains(&q_norm)) {
        best_score = 60;
    }

    best_score
}
