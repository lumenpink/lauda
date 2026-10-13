use serde::{Deserialize, Serialize};

/// Categoria do terminal telefônico identificado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhoneKind {
    /// Telefone móvel / celular (sujeito à regra do 9º dígito).
    Mobile,
    /// Telefone fixo residencial ou comercial.
    Landline,
    /// Serviço especial ou institucional.
    Special,
}

/// Registro telefônico higienizado e formatado nos padrões canônicos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedPhone {
    /// Representação internacional canônica ITU-T E.164 (ex: "+5548991234567").
    pub e164: String,
    /// Representação formatada legível (ex: "+55 (48) 99123-4567").
    pub formatted: String,
    /// Código de Discagem Direta a Distância (2 dígitos, ex: "48").
    pub ddd: String,
    /// Tipo de terminal detectado.
    pub kind: PhoneKind,
}

/// Erros possíveis durante a análise de normalização telefônica.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhoneError {
    MissingDdd,
    InvalidDdd(String),
    InvalidLength,
    InvalidSubscriber,
}

impl std::fmt::Display for PhoneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDdd => write!(f, "DDD ausente e nenhum valor padrão foi configurado"),
            Self::InvalidDdd(ddd) => write!(f, "DDD inválido: {}", ddd),
            Self::InvalidLength => write!(f, "Comprimento de número inválido"),
            Self::InvalidSubscriber => write!(f, "Assinante telefônico inválido"),
        }
    }
}

impl std::error::Error for PhoneError {}

/// Motor soberano de normalização telefônica para o padrão brasileiro (ANATEL / E.164).
pub struct PhoneNormalizer;

impl PhoneNormalizer {
    /// Normaliza um número de telefone brasileiro para o formato internacional E.164
    /// e formato de apresentação, aplicando remoção de CSP (operadoras) e inclusão
    /// do 9º dígito móvel onde aplicável.
    pub fn normalize_br(
        input: &str,
        default_ddd: Option<&str>,
    ) -> Result<NormalizedPhone, PhoneError> {
        let raw_digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
        if raw_digits.len() < 8 {
            return Err(PhoneError::InvalidLength);
        }

        let mut digits = raw_digits.as_str();

        // 1. Remoção de Código de Seleção de Prestadora (CSP) ou 0 de longa distância
        // Ex: 02148991234567 (14 dig) -> CSP 21, remove 3 dig
        // Ex: 0214832221234  (13 dig) -> CSP 21, remove 3 dig
        // Ex: 048991234567   (12 dig) -> 0 + DDD + cel, remove 1 dig
        // Ex: 04832221234    (11 dig) -> 0 + DDD + fixo, remove 1 dig
        if digits.starts_with('0') {
            if digits.len() >= 13 {
                // Remove '0' + 2 dígitos de CSP
                digits = &digits[3..];
            } else if digits.len() == 11 || digits.len() == 12 {
                // Remove prefixo '0' do DDD
                digits = &digits[1..];
            }
        }

        // 2. Remoção do DDI brasileiro (+55) se presente
        if digits.starts_with("55") && (digits.len() == 12 || digits.len() == 13) {
            digits = &digits[2..];
        }

        // 3. Extração de DDD e Assinante
        let (ddd, subscriber_raw) = match digits.len() {
            10 | 11 => (&digits[0..2], &digits[2..]),
            8 | 9 => {
                let d = default_ddd.ok_or(PhoneError::MissingDdd)?;
                (d, digits)
            }
            _ => return Err(PhoneError::InvalidLength),
        };

        // Validação de DDD (deve ter 2 dígitos entre 11 e 99)
        let ddd_val: u8 = ddd.parse().map_err(|_| PhoneError::InvalidDdd(ddd.to_string()))?;
        if !(11..=99).contains(&ddd_val) {
            return Err(PhoneError::InvalidDdd(ddd.to_string()));
        }

        // 4. Inclusão determinística do 9º dígito e classificação (ANATEL)
        let (subscriber, kind) = match subscriber_raw.len() {
            8 => {
                let first = subscriber_raw.chars().next().unwrap();
                match first {
                    // Celular antigo de 8 dígitos: prefixar 9
                    '6' | '7' | '8' | '9' => (format!("9{}", subscriber_raw), PhoneKind::Mobile),
                    // Telefone fixo: permanece 8 dígitos
                    '2' | '3' | '4' | '5' => (subscriber_raw.to_string(), PhoneKind::Landline),
                    _ => return Err(PhoneError::InvalidSubscriber),
                }
            }
            9 => {
                let first = subscriber_raw.chars().next().unwrap();
                if first == '9' {
                    (subscriber_raw.to_string(), PhoneKind::Mobile)
                } else {
                    return Err(PhoneError::InvalidSubscriber);
                }
            }
            _ => return Err(PhoneError::InvalidLength),
        };

        let e164 = format!("+55{}{}", ddd, subscriber);
        let formatted = match kind {
            PhoneKind::Mobile => {
                format!("+55 ({}) {}-{}", ddd, &subscriber[0..5], &subscriber[5..9])
            }
            PhoneKind::Landline => {
                format!("+55 ({}) {}-{}", ddd, &subscriber[0..4], &subscriber[4..8])
            }
            PhoneKind::Special => e164.clone(),
        };

        Ok(NormalizedPhone {
            e164,
            formatted,
            ddd: ddd.to_string(),
            kind,
        })
    }
}
