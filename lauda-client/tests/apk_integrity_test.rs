use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_packaged_apk_integrity_and_abi() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let apk_path = manifest_dir.join("../site/lauda-latest.apk");

    if !apk_path.exists() {
        eprintln!("APK not found at {:?}, skipping test.", apk_path);
        return;
    }

    // 1. Verify APK is a valid zip and inspect its entries
    let output = Command::new("unzip")
        .args(["-l", apk_path.to_str().unwrap()])
        .output()
        .expect("unzip command failed to execute");

    assert!(
        output.status.success(),
        "Failed to list APK contents with unzip"
    );

    let file_list = String::from_utf8_lossy(&output.stdout);

    // 2. Ensure AndroidManifest.xml exists
    assert!(
        file_list.contains("AndroidManifest.xml"),
        "APK must contain AndroidManifest.xml"
    );

    // 3. Ensure classes.dex exists (Android runtime bytecode)
    assert!(
        file_list.contains("classes.dex"),
        "APK must contain classes.dex"
    );

    // 4. Ensure arm64-v8a shared library exists (prevents UnsatisfiedLinkError)
    assert!(
        file_list.contains("lib/arm64-v8a/libdioxusmain.so"),
        "APK must contain compiled arm64-v8a native library"
    );

    // 5. Verify signature with apksigner if available
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
