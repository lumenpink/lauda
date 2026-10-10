/// Implementação padrão da pontuação BM25 (Best Matching 25).
///
/// Parâmetros clássicos:
/// - k1 = 1.2 (saturação de frequência do termo)
/// - b = 0.75 (penalização por comprimento do documento relativo à média)
pub fn score_bm25(
    term_freq: f64,
    doc_len: usize,
    avg_doc_len: f64,
    total_docs: usize,
    docs_with_term: usize,
) -> f64 {
    if total_docs == 0 || docs_with_term == 0 || term_freq <= 0.0 {
        return 0.0;
    }

    let k1 = 1.2;
    let b = 0.75;

    // Fórmula IDF de Robertson-Spärck Jones
    let idf = ((total_docs as f64 - docs_with_term as f64 + 0.5) / (docs_with_term as f64 + 0.5)
        + 1.0)
        .ln()
        .max(0.1);

    let doc_len_norm = 1.0 - b + b * (doc_len as f64 / avg_doc_len.max(1.0));
    let tf = (term_freq * (k1 + 1.0)) / (term_freq + k1 * doc_len_norm);

    idf * tf
}

/// Distância Levenshtein para fuzzy matching com tolerância a erros de digitação.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    let mut dp = vec![vec![0; n + 1]; m + 1];

    for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(n + 1) {
        *cell = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bm25_scoring() {
        let score = score_bm25(2.0, 100, 80.0, 50, 5);
        assert!(score > 0.0);
    }

    #[test]
    fn test_fuzzy_distance() {
        assert_eq!(levenshtein_distance("fatura", "fatura"), 0);
        assert_eq!(levenshtein_distance("fatura", "fatrua"), 2);
        assert_eq!(levenshtein_distance("reuniao", "reniao"), 1);
    }
}
