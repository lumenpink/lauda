# Especificação Técnica: Despacho de Notificações Push via inotify para UnifiedPush

**ID do Ticket:** #107  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Especificação Preliminar (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Para completar o ciclo de entrega de correio eletrônico sob demanda sem Google Play Services nem FCM, o `lauda-server` deve atuar como a ponte entre o sistema de arquivos local (Maildir) e os distribuidores UnifiedPush dos aparelhos clientes.

Sempre que uma nova mensagem for entregue via SMTP ou importada na pasta `new/` de uma caixa de correio, o subsistema de monitoramento de diretórios (`inotify`, implementado no Ticket #99) deve disparar um sinalizador leve (HTTP POST) para todos os endpoints registrados por clientes móveis.

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Decisão 1: Zero Conteúdo Sensível no Payload (*Blind Push*):**
  - *Contexto:* O aplicativo distribuidor (ex.: servidores públicos do ntfy.sh ou instâncias de terceiros) é um intermediário de entrega.
  - *Escolha adotada:* O HTTP POST disparado para o endpoint do distribuidor contém um corpo vazio ou estritamente um sinalizador neutro (`{"event": "sync"}`). **Nenhum cabeçalho, remetente, assunto ou dado privado de e-mail trafega pelo distribuidor**. O distribuidor atua unicamente como a campainha que cutuca o aparelho da usuária para que o Lauda baixe os dados diretamente do seu próprio servidor de forma criptografada.
- **Decisão 2: Despacho Desacoplado em Tarefas Assíncronas (`tokio::spawn`):**
  - *Contexto:* O monitoramento do Maildir precisa processar eventos de arquivos em tempo real sem sofrer bloqueios ou contenção de I/O de rede externa.
  - *Escolha adotada:* Ao identificar um novo arquivo em `new/`, o despachante invoca `tokio::spawn` com um cliente HTTP assíncrono configurado com **timeout estrito de 3 segundos**. Se o distribuidor estiver fora do ar ou com lentidão, o erro é registrado em log e descartado, sem afetar o loop do `inotify` nem a performance do servidor.
- **Decisão 3: Gerenciamento de Endpoints no Estado da Aplicação:**
  - *Escolha adotada:* Registro thread-safe de endpoints (`PushRegistry`) associado ao `AppState` do servidor Axum, com rotas REST:
    - `POST /api/push/register`: adiciona ou renova o endpoint de push.
    - `DELETE /api/push/unregister`: remove o endpoint de push.

---

## 3. Contratos de API e Estruturas de Dados

### 3.1. DTO de Registro de Push

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushRegisterRequest {
    pub endpoint: String,
    pub instance: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushUnregisterRequest {
    pub endpoint: String,
}
```

### 3.2. Rotas Axum

- `POST /api/push/register` -> `StatusCode::OK`
- `DELETE /api/push/unregister` -> `StatusCode::OK`

---

## 4. Testes e Validação (DD-TDD)

- Teste em `lauda-server/tests/push_dispatcher_test.rs`:
  - Testar inserção e remoção de endpoints no `PushRegistry`.
  - Testar despacho de notificação HTTP para um mock local com validação de payload sem vazamento de dados privados.
  - Testar tolerância a falhas e timeouts em endpoints inacessíveis sem pânico.
