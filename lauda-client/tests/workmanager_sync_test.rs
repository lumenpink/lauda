use std::path::PathBuf;

#[test]
fn test_workmanager_sync_worker_contract() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let worker_kt = manifest_dir.join("android/LaudaSyncWorker.kt");

    assert!(worker_kt.exists(), "LaudaSyncWorker.kt deve existir");
    let content = std::fs::read_to_string(&worker_kt).unwrap();

    assert!(
        content.contains("package in.twolp.lauda"),
        "Deve pertencer ao pacote in.twolp.lauda"
    );
    assert!(
        content.contains("class LaudaSyncWorker"),
        "Deve declarar a classe LaudaSyncWorker"
    );
    assert!(
        content.contains("CoroutineWorker"),
        "Deve herdar de CoroutineWorker"
    );
    assert!(
        content.contains("external fun nativeSyncInbox"),
        "Deve declarar o método JNI nativo nativeSyncInbox"
    );
}

#[test]
fn test_publish_nightly_script_injects_workmanager() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script_path = manifest_dir.join("../scripts/publish-nightly.sh");
    assert!(script_path.exists(), "publish-nightly.sh deve existir");

    let script_content = std::fs::read_to_string(&script_path).unwrap();

    // 1. Deve sincronizar LaudaSyncWorker.kt
    assert!(
        script_content.contains("LaudaSyncWorker.kt"),
        "publish-nightly.sh deve sincronizar LaudaSyncWorker.kt"
    );

    // 2. Deve injetar a dependência androidx.work:work-runtime-ktx
    assert!(
        script_content.contains("androidx.work:work-runtime-ktx"),
        "publish-nightly.sh deve injetar androidx.work:work-runtime-ktx no build.gradle"
    );
}

#[test]
fn test_main_activity_schedules_periodic_workmanager() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let main_kt = manifest_dir.join("android/MainActivity.kt");
    assert!(main_kt.exists(), "MainActivity.kt deve existir");

    let content = std::fs::read_to_string(&main_kt).unwrap();
    assert!(
        content.contains("WorkManager") && content.contains("LaudaSyncWorker"),
        "MainActivity.kt deve inicializar o WorkManager com LaudaSyncWorker"
    );
}
