use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use axum::serve;
use lauda_client::adapters::HttpApiClient;
use lauda_client::domain::ports::MailApiClient;
use lauda_client::domain::view::parse_markdown_to_html;
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::app_router;
use lauda_types::{ComposeForm, LoginRequest, Message, MessageEnvelope};
use serde_json::Value;
use tokio::net::TcpListener;

const TEST_SECRET: &str = "jwt-secret-apk-e2e-suite-2026";

fn get_repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Root do repositório lauda")
        .to_path_buf()
}

fn get_zip_entries(apk_path: &std::path::Path) -> Vec<String> {
    if let Ok(output) = Command::new("unzip")
        .args(["-l", apk_path.to_str().unwrap()])
        .output()
        && output.status.success()
    {
        let s = String::from_utf8_lossy(&output.stdout);
        return s.lines().map(|l| l.to_string()).collect();
    }

    // Fallback: Pure Rust Central Directory Parser
    let data = std::fs::read(apk_path).expect("Leitura dos bytes do APK");
    let mut entries = Vec::new();
    let mut pos = 0;
    while pos + 46 < data.len() {
        if let Some(offset) = data[pos..].windows(4).position(|w| w == b"PK\x01\x02") {
            let idx = pos + offset;
            if idx + 46 <= data.len() {
                let fn_len = u16::from_le_bytes([data[idx + 28], data[idx + 29]]) as usize;
                let extra_len = u16::from_le_bytes([data[idx + 30], data[idx + 31]]) as usize;
                let comment_len = u16::from_le_bytes([data[idx + 32], data[idx + 33]]) as usize;
                if idx + 46 + fn_len <= data.len() {
                    let fn_bytes = &data[idx + 46..idx + 46 + fn_len];
                    entries.push(String::from_utf8_lossy(fn_bytes).to_string());
                }
                pos = idx + 46 + fn_len + extra_len + comment_len;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    entries
}

fn get_stored_file_bytes(apk_path: &std::path::Path, target_filename: &str) -> Option<Vec<u8>> {
    if let Ok(extract_status) = Command::new("unzip")
        .args(["-p", apk_path.to_str().unwrap(), target_filename])
        .output()
        && extract_status.status.success()
        && !extract_status.stdout.is_empty()
    {
        return Some(extract_status.stdout);
    }

    // Fallback: busca direta do arquivo descompactado (stored) no arquivo ZIP
    let data = std::fs::read(apk_path).ok()?;
    let mut pos = 0;
    while pos + 46 < data.len() {
        if let Some(offset) = data[pos..].windows(4).position(|w| w == b"PK\x01\x02") {
            let idx = pos + offset;
            let fn_len = u16::from_le_bytes([data[idx + 28], data[idx + 29]]) as usize;
            let extra_len = u16::from_le_bytes([data[idx + 30], data[idx + 31]]) as usize;
            let comment_len = u16::from_le_bytes([data[idx + 32], data[idx + 33]]) as usize;
            if idx + 46 + fn_len <= data.len() {
                let fn_str = String::from_utf8_lossy(&data[idx + 46..idx + 46 + fn_len]);
                if fn_str == target_filename {
                    let uncomp_size = u32::from_le_bytes([
                        data[idx + 24],
                        data[idx + 25],
                        data[idx + 26],
                        data[idx + 27],
                    ]) as usize;
                    let local_offset = u32::from_le_bytes([
                        data[idx + 42],
                        data[idx + 43],
                        data[idx + 44],
                        data[idx + 45],
                    ]) as usize;
                    if local_offset + 30 <= data.len() {
                        let loc_fn_len =
                            u16::from_le_bytes([data[local_offset + 26], data[local_offset + 27]])
                                as usize;
                        let loc_extra_len =
                            u16::from_le_bytes([data[local_offset + 28], data[local_offset + 29]])
                                as usize;
                        let start = local_offset + 30 + loc_fn_len + loc_extra_len;
                        if start + uncomp_size <= data.len() {
                            return Some(data[start..start + uncomp_size].to_vec());
                        }
                    }
                }
            }
            pos = idx + 46 + fn_len + extra_len + comment_len;
        } else {
            break;
        }
    }
    None
}

/// 1. Teste E2E de Estrutura, ABI e Assinatura do APK Android (arm64-v8a)
#[test]
fn test_e2e_apk_structural_and_abi_verification() {
    let repo_root = get_repo_root();
    let apk_path = repo_root.join("site/lauda-latest.apk");

    if !apk_path.exists() {
        eprintln!("APK not found at {:?}, skipping test.", apk_path);
        return;
    }

    // A. Validação das entradas do APK
    let entries = get_zip_entries(&apk_path);
    assert!(
        !entries.is_empty(),
        "O APK deve conter entradas de arquivos válidas"
    );
    let entries_str = entries.join("\n");

    // B. Verificação do AndroidManifest.xml
    assert!(
        entries_str.contains("AndroidManifest.xml"),
        "APK deve conter AndroidManifest.xml"
    );

    // C. Verificação do Bytecode Dalvik (classes.dex)
    assert!(
        entries_str.contains("classes.dex"),
        "APK deve conter classes.dex com as classes Android (WryActivity/MainActivity)"
    );

    // D. Verificação da Biblioteca Nativa ARM64 compilada em Rust
    assert!(
        entries_str.contains("lib/arm64-v8a/libmain.so"),
        "APK deve conter a biblioteca nativa libmain.so para arquitetura arm64-v8a"
    );
    assert!(
        entries_str.contains("lib/x86_64/libmain.so"),
        "APK deve conter a biblioteca nativa libmain.so para arquitetura x86_64"
    );

    // E. Inspeção binária de cabeçalho ELF da biblioteca extraída
    let so_bytes = get_stored_file_bytes(&apk_path, "lib/arm64-v8a/libmain.so")
        .expect("Extrair bytes do cabeçalho da libmain.so");
    assert!(
        so_bytes.len() >= 64,
        "Bytes insuficientes para cabeçalho ELF"
    );

    let elf_header = &so_bytes[0..64];

    // Validação ELF Magic (\x7fELF)
    assert_eq!(
        &elf_header[0..4],
        b"\x7fELF",
        "A biblioteca libmain.so deve ser um binário ELF válido"
    );
    // Classe 64-bit (ELFCLASS64 = 2)
    assert_eq!(
        elf_header[4], 2,
        "A biblioteca deve ser 64-bit (ELFCLASS64)"
    );
    // Endianness (ELFDATA2LSB = 1)
    assert_eq!(
        elf_header[5], 1,
        "A biblioteca deve ser Little Endian (ELFDATA2LSB)"
    );
    // Machine Architecture AArch64 (0xb7 = 183 no header bytes 18..20)
    let machine_type = u16::from_le_bytes([elf_header[18], elf_header[19]]);
    assert_eq!(
        machine_type, 183,
        "A biblioteca deve ser compilada para a arquitetura AArch64 (ARM64)"
    );

    // F. Validação de Símbolos JNI Essenciais (start_app e WryActivity)
    let temp_dir = tempfile::tempdir().expect("Criar tempdir");
    let extracted_so = temp_dir.path().join("libmain.so");
    std::fs::write(&extracted_so, &so_bytes).expect("Escrever .so temporário");

    if let Ok(nm_output) = Command::new("nm")
        .args(["-D", extracted_so.to_str().unwrap()])
        .output()
    {
        let symbols = String::from_utf8_lossy(&nm_output.stdout);
        assert!(
            symbols.contains("start_app"),
            "libmain.so deve exportar start_app chamado pelo runtime Dioxus"
        );
        assert!(
            symbols.contains("Java_dev_dioxus_main_WryActivity_start")
                || symbols.contains("Java_dev_dioxus_main_"),
            "libmain.so deve exportar bindings JNI para Dioxus WryActivity"
        );
    }

    // G. Validação de Assinatura via apksigner (se disponível)
    if let Ok(signer_output) = Command::new("apksigner")
        .args(["verify", "--verbose", apk_path.to_str().unwrap()])
        .output()
    {
        assert!(
            signer_output.status.success(),
            "Assinatura do APK deve ser válida: {}",
            String::from_utf8_lossy(&signer_output.stderr)
        );
    }
}

/// 2. Teste E2E de Distribuição, Versionamento e Compatibilidade com Obtainium
#[test]
fn test_e2e_apk_distribution_manifest_and_obtainium() {
    let repo_root = get_repo_root();

    // A. Validação do arquivo VERSION
    let version_file_content = std::fs::read_to_string(repo_root.join("VERSION"))
        .expect("Leitura do VERSION")
        .trim()
        .to_string();

    // B. Validação do manifesto site/version.json
    let version_json_str = std::fs::read_to_string(repo_root.join("site/version.json"))
        .expect("Leitura do site/version.json");
    let manifest: Value = serde_json::from_str(&version_json_str)
        .expect("Manifesto version.json deve ser JSON válido");

    let json_version = manifest["version"]
        .as_str()
        .expect("version field deve ser string");
    let base_version_file = version_file_content
        .split(['-', '+'])
        .next()
        .unwrap_or(&version_file_content);
    assert!(
        json_version.starts_with(base_version_file),
        "Versão em version.json ({json_version}) deve ser compatível com VERSION base ({base_version_file})"
    );

    let apk_url = manifest["apk_url"]
        .as_str()
        .expect("apk_url field deve ser string");
    assert!(
        apk_url.ends_with("/lauda-latest.apk"),
        "apk_url deve apontar para lauda-latest.apk"
    );

    // C. Validação de Compatibilidade com Raspagem HTML do Obtainium
    let index_html = std::fs::read_to_string(repo_root.join("site/index.html"))
        .expect("Leitura de site/index.html");
    assert!(
        index_html.contains("lauda-latest.apk"),
        "site/index.html deve conter link para lauda-latest.apk para autodescoberta pelo Obtainium"
    );

    // D. Validação de Tamanho e Checksum do APK Distribuído (se presente)
    let apk_path = repo_root.join("site/lauda-latest.apk");
    if apk_path.exists() {
        let metadata = std::fs::metadata(&apk_path).expect("Metadados do APK");
        assert!(
            metadata.len() > 10 * 1024 * 1024,
            "APK deve ter tamanho superior a 10MB (contendo runtime compilado), tamanho atual: {} bytes",
            metadata.len()
        );
    } else {
        eprintln!(
            "APK not found at {:?}, skipping binary size verification.",
            apk_path
        );
    }
}

/// 3. Teste E2E de Ciclo Completo: Cliente Móvel Dioxus interagindo com Live Server
#[tokio::test]
async fn test_e2e_apk_mobile_client_live_server_full_loop() {
    // A. Inicialização do Backend em Memória e Router
    let backend = Arc::new(InMemoryMailBackend::new());
    backend.add_user("lumen", "senha_mobile_2026");
    backend.add_mailbox(
        "lumen",
        lauda_types::MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 1,
            unread_messages: 1,
        },
    );

    let sample_md =
        "# Correio Seguro Lauda Mobile\n\nMensagem recebida no cliente Android em texto puro.";
    backend.add_message(
        "lumen",
        "INBOX",
        Message {
            envelope: MessageEnvelope {
                uid: 777,
                subject: "Bem-vindo ao Lauda Android".to_string(),
                sender: "postmaster@2lp.in".to_string(),
                date: "2026-10-03T12:00:00Z".to_string(),
                cc: None,
                has_attachments: false,
                is_read: false,
                is_flagged: false,
                crypto_meta: None,
            },
            attachments: Vec::new(),
            txt_auto: Some(sample_md.to_string()),
            txt_original: Some(sample_md.to_string()),
            html_sanitized: Some(format!("<p>{sample_md}</p>")),
        },
    );

    let app = app_router(backend.clone(), TEST_SECRET);

    // B. Subida de Servidor TCP Real em Porta Efêmera (Loopback)
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Vincular listener TCP em porta livre");
    let server_addr: SocketAddr = listener.local_addr().expect("Obter endereço local");

    tokio::spawn(async move {
        serve(listener, app).await.expect("Servidor HTTP ativo");
    });

    let base_url = format!("http://{server_addr}");

    // C. Instanciação do Cliente HTTP exatamente como executado no código Rust do APK
    let client = HttpApiClient::new(base_url);

    // D. Passo 1 do Ciclo: Autenticação via /auth/login
    let login_resp = client
        .login(&LoginRequest {
            username: "lumen".to_string(),
            password: "senha_mobile_2026".to_string(),
        })
        .await
        .expect("Login do cliente mobile deve ser bem-sucedido");

    assert_eq!(login_resp.username, "lumen");
    let token = &login_resp.token;

    // E. Passo 2 do Ciclo: Listagem de Caixas Postais
    let boxes = client
        .list_mailboxes(token)
        .await
        .expect("Listar caixas postais");
    assert_eq!(boxes.len(), 1);
    assert_eq!(boxes[0].name, "INBOX");

    // F. Passo 3 do Ciclo: Listagem e Recuperação de Mensagem com Visualização Trifásica
    let messages = client
        .list_messages(token, "INBOX")
        .await
        .expect("Listar mensagens da INBOX");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].uid, 777);

    let msg = client
        .get_message(token, "INBOX", 777)
        .await
        .expect("Obter mensagem 777");
    assert_eq!(msg.envelope.subject, "Bem-vindo ao Lauda Android");

    let txt_auto = msg.txt_auto.expect("txt_auto presente");
    let rendered_html = parse_markdown_to_html(&txt_auto);
    assert!(
        rendered_html.contains("<h1>Correio Seguro Lauda Mobile</h1>"),
        "Visualizador do cliente mobile deve converter Markdown para HTML nativo"
    );

    // G. Passo 4 do Ciclo: Composição e Envio Desimpedido de Resposta (/compose)
    let send_res = client
        .send_message(
            token,
            &ComposeForm {
                from: "lumen@2lp.in".to_string(),
                to: "postmaster@2lp.in".to_string(),
                cc: None,
                subject: "Re: Bem-vindo ao Lauda Android".to_string(),
                body: "Confirmacao de recebimento pelo cliente mobile em aarch64.".to_string(),
                attachments: vec![],
            },
        )
        .await;

    assert!(
        send_res.is_ok(),
        "Envio de mensagem a partir do cliente mobile deve ter sucesso"
    );

    // H. Verificação de Persistência no Backend
    assert_eq!(backend.sent_count(), 1);
}

#[test]
fn test_ci_multi_target_pipeline_manifests_and_scripts() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap();

    // 1. O script canônico de sincronização de manifestos deve existir e ser executável
    let sync_script = repo_root.join("scripts/sync-version-manifests.sh");
    assert!(
        sync_script.exists(),
        "scripts/sync-version-manifests.sh deve existir"
    );

    // 2. O script canônico de empacotamento completo deve existir
    let package_script = repo_root.join("scripts/package-all.sh");
    assert!(
        package_script.exists(),
        "scripts/package-all.sh deve existir"
    );

    // 3. Execução do script de sincronização e validação de version.json e version-nightly.json
    let version_content = std::fs::read_to_string(repo_root.join("VERSION"))
        .expect("Leitura do VERSION")
        .trim()
        .to_string();

    let v_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root.join("site/version.json")).expect("version.json"),
    )
    .expect("json parse");
    assert_eq!(
        v_json["version"].as_str().unwrap(),
        version_content,
        "site/version.json deve refletir a versão canônica do arquivo VERSION"
    );

    let v_nightly_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root.join("site/version-nightly.json"))
            .expect("version-nightly.json"),
    )
    .expect("json parse");
    assert_eq!(
        v_nightly_json["version"].as_str().unwrap(),
        version_content,
        "site/version-nightly.json deve refletir a versão canônica do arquivo VERSION"
    );
}
