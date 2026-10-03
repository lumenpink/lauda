use std::path::{Path, PathBuf};
use std::process::Command;

fn get_zip_entries(apk_path: &Path) -> Vec<String> {
    if let Ok(output) = Command::new("unzip")
        .args(["-l", apk_path.to_str().unwrap()])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            return s.lines().map(|l| l.to_string()).collect();
        }
    }

    // Fallback: Pure Rust Central Directory Parser (funciona sem unzip)
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

#[test]
fn test_packaged_apk_integrity_and_abi() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let apk_path = manifest_dir.join("../site/lauda-latest.apk");

    if !apk_path.exists() {
        eprintln!("APK not found at {:?}, skipping test.", apk_path);
        return;
    }

    // 1. Inspecionar entradas do APK (com suporte nativo em Rust ou unzip)
    let entries = get_zip_entries(&apk_path);
    assert!(
        !entries.is_empty(),
        "O APK deve conter entradas de arquivos válidas"
    );

    let entries_str = entries.join("\n");

    // 2. Garantir que AndroidManifest.xml existe
    assert!(
        entries_str.contains("AndroidManifest.xml"),
        "APK deve conter AndroidManifest.xml"
    );

    // 3. Garantir que classes.dex existe (Dalvik bytecode)
    assert!(
        entries_str.contains("classes.dex"),
        "APK deve conter classes.dex"
    );

    // 4. Garantir que libdioxusmain.so para arm64-v8a existe
    assert!(
        entries_str.contains("lib/arm64-v8a/libdioxusmain.so"),
        "APK deve conter a biblioteca nativa compilada para arm64-v8a"
    );

    // 5. Verificar assinatura com apksigner se disponível no ambiente
    if let Ok(signer_output) = Command::new("apksigner")
        .args(["verify", apk_path.to_str().unwrap()])
        .output()
    {
        assert!(
            signer_output.status.success(),
            "APK signature verification failed: {}",
            String::from_utf8_lossy(&signer_output.stderr)
        );
    }
}
