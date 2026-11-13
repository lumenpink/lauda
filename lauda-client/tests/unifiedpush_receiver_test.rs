use std::path::PathBuf;

#[test]
fn test_unifiedpush_receiver_contract() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let receiver_kt = manifest_dir.join("android/LaudaPushReceiver.kt");

    assert!(receiver_kt.exists(), "LaudaPushReceiver.kt deve existir");
    let content = std::fs::read_to_string(&receiver_kt).unwrap();

    assert!(
        content.contains("package in.twolp.lauda"),
        "Deve pertencer ao pacote in.twolp.lauda"
    );
    assert!(
        content.contains("class LaudaPushReceiver"),
        "Deve declarar a classe LaudaPushReceiver"
    );
    assert!(
        content.contains("MessagingReceiver"),
        "Deve estender MessagingReceiver do UnifiedPush"
    );
    assert!(
        content.contains("setExpedited") && content.contains("ExistingWorkPolicy.KEEP"),
        "Deve seguir a Regra de Ouro com setExpedited e ExistingWorkPolicy.KEEP"
    );
    assert!(
        content.contains("onNewEndpoint") && content.contains("onMessage"),
        "Deve implementar onNewEndpoint e onMessage"
    );
}

#[test]
fn test_publish_nightly_script_injects_unifiedpush() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script_path = manifest_dir.join("../scripts/publish-nightly.sh");
    assert!(script_path.exists(), "publish-nightly.sh deve existir");

    let script_content = std::fs::read_to_string(&script_path).unwrap();

    // 1. Deve sincronizar LaudaPushReceiver.kt
    assert!(
        script_content.contains("LaudaPushReceiver.kt"),
        "publish-nightly.sh deve sincronizar LaudaPushReceiver.kt"
    );

    // 2. Deve injetar a dependência org.unifiedpush.android:connector
    assert!(
        script_content.contains("org.unifiedpush.android:connector"),
        "publish-nightly.sh deve injetar o SDK do UnifiedPush no build.gradle"
    );

    // 3. Deve injetar o receiver no AndroidManifest.xml
    assert!(
        script_content.contains("in.twolp.lauda.LaudaPushReceiver"),
        "publish-nightly.sh deve registrar LaudaPushReceiver no manifesto"
    );
    assert!(
        script_content.contains("org.unifiedpush.android.connector.MESSAGE"),
        "Manifesto deve incluir action MESSAGE do UnifiedPush"
    );
}
