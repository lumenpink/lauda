use lauda_index::{FlatVectorIndex, reciprocal_rank_fusion};

#[test]
fn test_flat_vector_index_basic_knn() {
    let mut index = FlatVectorIndex::new(4);
    assert_eq!(index.len(), 0);
    assert!(index.is_empty());

    // Vetores ortogonais e diagonais normalizados
    // uid 10: [1, 0, 0, 0]
    index.insert(10, &[1.0, 0.0, 0.0, 0.0]);
    // uid 20: [0, 1, 0, 0]
    index.insert(20, &[0.0, 1.0, 0.0, 0.0]);
    // uid 30: [0.7071, 0.7071, 0, 0]
    index.insert(30, &[0.70710677, 0.70710677, 0.0, 0.0]);

    assert_eq!(index.len(), 3);

    // Consulta próxima ao uid 10: [0.9, 0.1, 0.0, 0.0] normalizado
    let query = [0.9938837, 0.11043153, 0.0, 0.0];
    let results = index.search_knn(&query, 2);

    assert_eq!(results.len(), 2);
    // Primeiro deve ser uid 10
    assert_eq!(results[0].0, 10);
    // Segundo deve ser uid 30
    assert_eq!(results[1].0, 30);
    assert!(results[0].1 > results[1].1);
}

#[test]
fn test_flat_vector_index_removal_and_update() {
    let mut index = FlatVectorIndex::new(3);
    index.insert(1, &[1.0, 0.0, 0.0]);
    index.insert(2, &[0.0, 1.0, 0.0]);
    index.insert(3, &[0.0, 0.0, 1.0]);

    assert_eq!(index.len(), 3);

    // Remover uid 2
    let removed = index.remove(2);
    assert!(removed);
    assert_eq!(index.len(), 2);

    // Buscar perto de [0, 1, 0] não deve achar o uid 2
    let results = index.search_knn(&[0.0, 1.0, 0.0], 5);
    assert_eq!(results.len(), 2);
    assert!(!results.iter().any(|(uid, _)| *uid == 2));

    // Atualizar uid 1 com novo vetor
    index.insert(1, &[0.0, 1.0, 0.0]);
    assert_eq!(index.len(), 2);
    let results_after = index.search_knn(&[0.0, 1.0, 0.0], 1);
    assert_eq!(results_after[0].0, 1);
    assert!((results_after[0].1 - 1.0).abs() < 1e-4);
}

#[test]
fn test_reciprocal_rank_fusion_logic() {
    // Cenário:
    // Léxico encontrou: UID 100 (rank 0), UID 200 (rank 1), UID 300 (rank 2)
    // Semântico encontrou: UID 200 (rank 0), UID 400 (rank 1 - sinônimo puro), UID 100 (rank 2)
    let lexical = vec![100, 200, 300];
    let semantic = vec![200, 400, 100];

    let fused = reciprocal_rank_fusion(&lexical, &semantic, 60.0);

    // UID 200 esteve no rank 1 do léxico e rank 0 do semântico -> deve ser o vencedor
    assert_eq!(fused[0].0, 200);
    // UID 100 esteve no rank 0 do léxico e rank 2 do semântico -> segundo lugar
    assert_eq!(fused[1].0, 100);
    // UID 400 que só esteve no semântico (sinônimo) deve estar presente na lista fundida
    assert!(fused.iter().any(|(uid, _)| *uid == 400));
}
