# Especificação Técnica: Tratar Versão Idêntica ou Superior à Remota como UpToDate

**ID do Ticket:** #111  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Em Especificação (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

No cliente Lauda (`lauda-client/src/domain/update.rs`), a função `check_update(current_version, manifest)` avalia se o aplicativo necessita de atualização ou se há uma versão disponível.

Atualmente, a implementação avalia a condição mandatória antes de checar se a versão remota é de fato mais nova:
```rust
if current_triplet < min_triplet {
    UpdateStatus::Required(manifest.clone())
} else if current_triplet < remote_triplet {
    UpdateStatus::Available(manifest.clone())
} else {
    UpdateStatus::UpToDate
}
```

Essa lógica possui uma falha crítica de encadeamento: se o manifesto remoto contiver uma `min_compatible_version` maior que a versão atual (ou se o manifesto for publicado com valor inconsistente), qualquer cliente em versão inferior a `min_triplet` é marcado como `UpdateStatus::Required`, **mesmo quando a versão instalada for exatamente igual à versão mais recente oferecida pelo próprio manifesto** (`manifest.version`). Como consequência, a interface web e mobile exibe um banner contraditório e circular: "NOVA VERSÃO DISPONÍVEL: vX.Y.Z", onde vX.Y.Z é a própria versão instalada.

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Problema:** Um cliente na versão mais recente disponível (ou superior) recebe status `Required` ou `Available` caso `min_compatible_version` do manifesto seja divergente ou mal formatado.
- **Alternativa Descartada 1: Apenas comparar igualdade de strings exatas antes de avaliar os tripletos.**
  - *Motivo do descarte:* Insuficiente. Se a versão local for uma compilação de desenvolvimento superior (ex: build local com commit à frente), ou se as versões contiverem prefixos `v` inconsistentes ou metadados de build (`+...`), a igualdade estrita de strings falharia e ainda assim o cliente cairia na checagem errada de `Required`.
- **Alternativa Descartada 2: Remover o conceito de `UpdateStatus::Required`.**
  - *Motivo do descarte:* Viola o contrato arquitetural do Lauda, que necessita forçar atualizações em clientes obsoletos quando ocorrem quebras de protocolo ou migrações de esquema incompatíveis no servidor.
- **Escolha Adotada (Condição Precedente de Novidade Remota):**
  - **Princípio Invariante:** Um cliente **só pode ser notificado de atualização (`Available` ou `Required`) se a versão remota for comprovadamente mais recente que a versão instalada localmente**.
  - **Fluxo de Decisão Canônico:**
    1. Normalizar as versões removendo prefixos `v` e espaços em branco.
    2. Se `current_clean == remote_clean`: retorno imediato `UpdateStatus::UpToDate`.
    3. Avaliar se a versão remota é estritamente mais nova que a versão local:
       - Se a remota NÃO for mais nova (local $\ge$ remota): retorno imediato `UpdateStatus::UpToDate`.
       - Se a remota FOR mais nova (remota $>$ local):
         - Se a versão local for menor que `min_compatible_version`: retorna `UpdateStatus::Required(manifest)`.
         - Caso contrário: retorna `UpdateStatus::Available(manifest)`.

---

## 3. Arquivos Afetados

1. `lauda-client/src/domain/update.rs` — correção da precedência lógica em `check_update`, garantindo curto-circuito de igualdade e verificação prévia de versão remota mais nova.
2. `lauda-client/tests/update_domain_test.rs` — suíte abrangente de testes unitários cobrindo:
   - Versão local idêntica à remota com `min_compatible_version` superior.
   - Versão local idêntica à remota com metadados dev/build.
   - Versão local superior à remota.
   - Atualização opcional (`Available`).
   - Atualização mandatória legítima (`Required`).
3. `docs/specs/111-tratar-versao-identica-a-remota-como-uptodate-em-check-update-spec.md` — esta especificação técnica.

---

## 4. Requisitos e Critérios de Aceite

1. **Curto-Circuito de Identidade:** `check_update` retorna `UpdateStatus::UpToDate` quando `current_version == manifest.version`, independentemente de `min_compatible_version`.
2. **Precedência de Versão:** `Required` ou `Available` só podem ocorrer se a versão remota for estritamente mais recente que a atual.
3. **Cobertura Unitária:** Suíte de testes com 100% de passagem nos cenários de igualdade, prefixo `v`, builds de desenvolvimento e inconsistências de manifesto.
4. **Quality Gate:** `cargo test -p lauda-client`, `cargo clippy --all-targets -- -D warnings` e `cargo fmt --check`.
