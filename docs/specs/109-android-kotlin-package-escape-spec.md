# Especificação Técnica: Escapamento do Identificador de Pacote 'in' no Compilador Kotlin

**ID do Ticket:** #109  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Especificação Preliminar (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Durante a compilação do APK Android pelo Gradle (`compileDebugKotlin`), o compilador Kotlin (`kotlinc`) rejeitou os arquivos do pacote `in.twolp.lauda` com o seguinte erro de sintaxe:
```text
Syntax error: Package name must be a '.'-separated identifier list.
Expecting qualified name.
```

A raiz do problema é que a palavra `in` é uma palavra reservada da linguagem Kotlin (utilizada para operadores de pertencimento e modificadores de variância). Na gramática formal do Kotlin, identificadores que colidem com palavras-chave reservadas devem ser delimitados por crases (backticks): `` `in` ``.

Adicionalmente, o construtor da função utilitária `PeriodicWorkRequestBuilder` exige argumento do tipo `Long` para o intervalo de repetição (`15L`), e o import da classe `LaudaSyncWorker` na `MainActivity.kt` também deve usar o pacote escapado.

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Decisão 1: Escapamento com Crases no Kotlin vs Renomeação do Domínio:**
  - *Alternativa descartada:* Renomear o pacote para `intwolp.lauda` ou similar.
    - *Motivo do descarte:* O identificador oficial e canônico de contas, endpoints e domínio da organização é `in.twolp.lauda`. Renomear causaria quebra com o `AccountManager`, autoridades do `SyncAdapter` e metadados no `AndroidManifest.xml`.
  - *Escolha adotada:* **Adotar a sintaxe canônica da linguagem Kotlin: ``package `in`.twolp.lauda``**.
    - No código Kotlin: ``package `in`.twolp.lauda`` e ``import `in`.twolp.lauda.LaudaSyncWorker``.
    - No `AndroidManifest.xml` e no bytecode Java/DEX: o nome permanece `in.twolp.lauda`, preservando 100% da interoperabilidade sem qualquer mudança nos contratos externos.
- **Decisão 2: Tipagem Explícita `Long` no `PeriodicWorkRequestBuilder`:**
  - *Escolha adotada:* Passar `15L` explicitamente em vez de literal inteiro não qualificado `15`, eliminando ambiguidade na resolução de sobrecargas de métodos Kotlin/Java da biblioteca WorkManager.

---

## 3. Arquivos Afetados

1. `lauda-client/android/LaudaAuthenticatorService.kt`
2. `lauda-client/android/LaudaCalendarSyncService.kt`
3. `lauda-client/android/LaudaPushReceiver.kt`
4. `lauda-client/android/LaudaSyncService.kt`
5. `lauda-client/android/LaudaSyncWorker.kt`
6. `lauda-client/android/MainActivity.kt`
7. Testes em `lauda-client/tests/`
