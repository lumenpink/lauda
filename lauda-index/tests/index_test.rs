use lauda_index::MessageIndex;
use lauda_types::{Message, MessageEnvelope};
use tempfile::tempdir;

fn mock_message(
    uid: u32,
    sender: &str,
    subject: &str,
    body: &str,
    read: bool,
    flagged: bool,
    attachments: bool,
) -> Message {
    Message {
        envelope: MessageEnvelope {
            uid,
            subject: subject.to_string(),
            sender: sender.to_string(),
            date: "2026-10-10T12:00:00Z".to_string(),
            is_read: read,
            is_flagged: flagged,
            cc: None,
            has_attachments: attachments,
        },
        txt_auto: Some(body.to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    }
}

#[test]
fn test_index_and_bm25_ranking() {
    let mut index = MessageIndex::new();

    let m1 = mock_message(
        1,
        "financeiro@empresa.com",
        "Fatura Vivo de Outubro",
        "Segue em anexo a sua fatura vivo deste mês para pagamento imediato.",
        false,
        true,
        true,
    );

    let m2 = mock_message(
        2,
        "amigo@email.com",
        "E aí, tudo bem?",
        "Estou vivo e bem por aqui, vamos combinar um café.",
        true,
        false,
        false,
    );

    index.index_message("INBOX", &m1);
    index.index_message("INBOX", &m2);

    // Busca por termo comum "vivo"
    let results = index.search("vivo");
    assert_eq!(results.len(), 2);
    // Mensagem 1 deve ter score maior pelo boost no assunto e frequência do termo
    assert_eq!(
        results[0], 1,
        "Mensagem 1 deve ser a primeira por relevância BM25"
    );
    assert_eq!(results[1], 2);
}

#[test]
fn test_gmail_operators_and_boolean() {
    let mut index = MessageIndex::new();

    let m1 = mock_message(
        1,
        "alice@work.com",
        "Relatório Mensal",
        "Resumo das métricas",
        true,
        false,
        false,
    );
    let m2 = mock_message(
        2,
        "bob@bank.com",
        "Comprovante Pix",
        "Transferência efetuada",
        false,
        true,
        true,
    );

    index.index_message("INBOX", &m1);
    index.index_message("INBOX", &m2);

    // Filtro por remetente
    assert_eq!(index.search("from:alice"), vec![1]);
    assert_eq!(index.search("from:bob"), vec![2]);

    // Filtro por anexo
    assert_eq!(index.search("has:attachment"), vec![2]);

    // Filtro por flags
    assert_eq!(index.search("is:unread"), vec![2]);
    assert_eq!(index.search("is:flagged"), vec![2]);

    // Negação
    assert_eq!(index.search("-from:alice"), vec![2]);

    // OR
    let or_res = index.search("alice OR bob");
    assert_eq!(or_res.len(), 2);
}

#[test]
fn test_fuzzy_search_for_typos() {
    let mut index = MessageIndex::new();

    let m1 = mock_message(
        10,
        "diretoria@empresa.com",
        "Ata da Reunião de Planejamento",
        "Definições para o próximo trimestre.",
        true,
        false,
        false,
    );
    index.index_message("INBOX", &m1);

    // Usuário digitou "reniao" (typo de reunião)
    let res = index.search("reniao");
    assert_eq!(
        res,
        vec![10],
        "Fuzzy search deve encontrar mesmo com erro de digitação"
    );
}

#[test]
fn test_index_persistence_roundtrip() {
    let temp = tempdir().expect("tempdir");
    let file = temp.path().join("index.json");

    let mut index = MessageIndex::new();
    let m = mock_message(
        42,
        "suporte@2lp.in",
        "Ticket Aberto",
        "Detalhes do chamado",
        true,
        false,
        false,
    );
    index.index_message("INBOX", &m);

    index.save_to_file(&file).expect("save");

    let loaded = MessageIndex::load_from_file(&file)
        .expect("load")
        .expect("some index");
    let res = loaded.search("ticket");
    assert_eq!(res, vec![42]);
}
