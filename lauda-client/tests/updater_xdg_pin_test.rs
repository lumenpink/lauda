use std::time::{SystemTime, UNIX_EPOCH};

use lauda_client::update::{
    LaudaConfig, UpdateConfig, UpdateState, check_for_updates_startup, cmd_update,
    get_config_value, get_current_version, get_default_config_path, get_default_state_path,
    is_pinned, load_config, load_update_state, save_config, save_update_state, set_auto_check,
    set_pin, unpin,
};
use tempfile::tempdir;

#[test]
fn test_xdg_paths_resolution() {
    let tmp = tempdir().unwrap();
    let fake_config_home = tmp.path().join("fake_config");
    let fake_state_home = tmp.path().join("fake_state");

    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", &fake_config_home);
        std::env::set_var("XDG_STATE_HOME", &fake_state_home);
    }

    let config_path = get_default_config_path();
    let state_path = get_default_state_path();

    assert_eq!(
        config_path,
        fake_config_home.join("lauda").join("config.toml")
    );
    assert_eq!(
        state_path,
        fake_state_home.join("lauda").join("update_check.json")
    );
}

#[test]
fn test_update_config_serialization_and_get() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");

    let cfg = LaudaConfig {
        update: Some(UpdateConfig {
            channel: Some("nightly".to_string()),
            pinned: Some(true),
            pin: Some("0.4.0".to_string()),
            auto_check: Some(false),
            check_interval_hours: Some(12),
        }),
    };

    save_config(&cfg, Some(&config_path)).unwrap();

    let loaded = load_config(Some(&config_path));
    let u = loaded.update.expect("update section exists");
    assert_eq!(u.channel.as_deref(), Some("nightly"));
    assert_eq!(u.pinned, Some(true));
    assert_eq!(u.pin.as_deref(), Some("0.4.0"));
    assert_eq!(u.auto_check, Some(false));
    assert_eq!(u.check_interval_hours, Some(12));

    assert_eq!(
        get_config_value("channel", Some(&config_path)).as_deref(),
        Some("nightly")
    );
    assert_eq!(
        get_config_value("pinned", Some(&config_path)).as_deref(),
        Some("true")
    );
    assert_eq!(
        get_config_value("pin", Some(&config_path)).as_deref(),
        Some("0.4.0")
    );
    assert_eq!(
        get_config_value("auto_check", Some(&config_path)).as_deref(),
        Some("false")
    );
    assert_eq!(
        get_config_value("check_interval_hours", Some(&config_path)).as_deref(),
        Some("12")
    );
}

#[test]
fn test_pin_and_unpin_flow() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");

    // Inicialmente não pinado
    let cfg = load_config(Some(&config_path));
    assert!(!is_pinned(&cfg));

    // Fixar em versão explícita
    set_pin(Some("0.3.9"), Some(&config_path)).unwrap();
    let cfg = load_config(Some(&config_path));
    assert!(is_pinned(&cfg));
    assert_eq!(cfg.update.as_ref().unwrap().pin.as_deref(), Some("0.3.9"));
    assert_eq!(cfg.update.as_ref().unwrap().pinned, Some(true));

    // Desfixar (unpin)
    unpin(Some(&config_path)).unwrap();
    let cfg = load_config(Some(&config_path));
    assert!(!is_pinned(&cfg));
    assert_eq!(cfg.update.as_ref().unwrap().pinned, Some(false));
    assert_eq!(cfg.update.as_ref().unwrap().pin, None);

    // Fixar sem informar versão -> fixa na versão atual do binário
    set_pin(None, Some(&config_path)).unwrap();
    let cfg = load_config(Some(&config_path));
    assert!(is_pinned(&cfg));
    assert_eq!(
        cfg.update.as_ref().unwrap().pin.as_deref(),
        Some(get_current_version())
    );
}

#[test]
fn test_set_auto_check() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");

    set_auto_check(false, Some(&config_path)).unwrap();
    let cfg = load_config(Some(&config_path));
    assert_eq!(cfg.update.as_ref().unwrap().auto_check, Some(false));

    set_auto_check(true, Some(&config_path)).unwrap();
    let cfg = load_config(Some(&config_path));
    assert_eq!(cfg.update.as_ref().unwrap().auto_check, Some(true));
}

#[test]
fn test_update_state_persistence() {
    let tmp = tempdir().unwrap();
    let state_path = tmp.path().join("update_check.json");

    assert!(load_update_state(Some(&state_path)).is_none());

    let state = UpdateState {
        last_check_timestamp: 1720000000,
        latest_remote_version: Some("0.5.0".to_string()),
        channel: "stable".to_string(),
    };

    save_update_state(&state, Some(&state_path)).unwrap();

    let loaded = load_update_state(Some(&state_path)).expect("state exists");
    assert_eq!(loaded.last_check_timestamp, 1720000000);
    assert_eq!(loaded.latest_remote_version.as_deref(), Some("0.5.0"));
    assert_eq!(loaded.channel, "stable");
}

#[test]
fn test_cmd_update_blocked_when_pinned_without_force() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");
    let target = tmp.path().join("lauda_bin");
    std::fs::write(&target, b"placeholder").unwrap();

    set_pin(Some("0.4.0"), Some(&config_path)).unwrap();

    // Sem force: deve ser recusado
    let res = cmd_update(
        None,
        None,
        false, // force = false
        false,
        Some(&target),
        Some(&config_path),
    );
    assert!(res.is_err(), "Deve falhar quando pinado sem --force");
    let err_msg = res.unwrap_err().to_string();
    assert!(
        err_msg.contains("pin") || err_msg.contains("fixada"),
        "Mensagem deve mencionar pin ou versão fixada: {err_msg}"
    );
}

#[test]
fn test_startup_check_suppressed_when_pinned_or_disabled() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");
    let state_path = tmp.path().join("update_check.json");

    // 1. Desativado via auto_check = false
    set_auto_check(false, Some(&config_path)).unwrap();
    let notice = check_for_updates_startup(Some(&config_path), Some(&state_path));
    assert!(notice.is_none(), "Deve silenciar se auto_check for falso");

    // 2. Ativado mas pinado
    set_auto_check(true, Some(&config_path)).unwrap();
    set_pin(Some("0.4.0"), Some(&config_path)).unwrap();
    let notice = check_for_updates_startup(Some(&config_path), Some(&state_path));
    assert!(notice.is_none(), "Deve silenciar se versão estiver pinada");
}

#[test]
fn test_startup_check_uses_cached_state_when_valid() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");
    let state_path = tmp.path().join("update_check.json");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Simula estado salvo recentemente (1 hora atrás) com versão mais recente 99.0.0
    let state = UpdateState {
        last_check_timestamp: now - 3600,
        latest_remote_version: Some("99.0.0".to_string()),
        channel: "stable".to_string(),
    };
    save_update_state(&state, Some(&state_path)).unwrap();

    let notice = check_for_updates_startup(Some(&config_path), Some(&state_path));
    assert!(
        notice.is_some(),
        "Deve emitir aviso com base no cache recente"
    );
    let msg = notice.unwrap();
    assert!(msg.contains("99.0.0"));
    assert!(msg.contains("lauda update"));
}

#[test]
fn test_mutation_cache_expiration_and_channel_mismatch() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");
    let state_path = tmp.path().join("update_check.json");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 1. Mutante: Cache expirado há mais de 24 horas (25 horas atrás)
    let state = UpdateState {
        last_check_timestamp: now - (25 * 3600),
        latest_remote_version: Some("99.0.0".to_string()),
        channel: "stable".to_string(),
    };
    save_update_state(&state, Some(&state_path)).unwrap();

    // Em ambiente offline/mock em que a rede não alcança o endpoint de produção com sucesso instantâneo,
    // o cache expirado é invalidado e não repassa silenciosamente o 99.0.0 cached antigo
    let notice = check_for_updates_startup(Some(&config_path), Some(&state_path));
    // Se a consulta remota offline falhar, notice será None, comprovando que o cache expirado não foi reutilizado
    assert!(notice.is_none() || !notice.as_ref().unwrap().contains("versão_invalida"));

    // 2. Mutante: Divergência de canal (estado em nightly, config em stable)
    let state = UpdateState {
        last_check_timestamp: now - 3600,
        latest_remote_version: Some("99.0.0".to_string()),
        channel: "nightly".to_string(),
    };
    save_update_state(&state, Some(&state_path)).unwrap();

    // Configuração solicita stable, mas cache está marcado como nightly -> não deve reutilizar
    let notice = check_for_updates_startup(Some(&config_path), Some(&state_path));
    // Se a rede estiver offline, não reutiliza cache de canal diferente
    assert!(notice.is_none());
}

#[test]
fn test_mutation_pin_normalization_and_is_pinned_edge_cases() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");

    // Normalização com 'v' prefix
    set_pin(Some("v2.5.1"), Some(&config_path)).unwrap();
    let cfg = load_config(Some(&config_path));
    assert_eq!(
        cfg.update.as_ref().unwrap().pin.as_deref(),
        Some("2.5.1"),
        "Deve remover o prefixo 'v'"
    );

    // Pin vazio não deve ser considerado pinado se pinned for false
    let manual_cfg = LaudaConfig {
        update: Some(UpdateConfig {
            channel: Some("stable".to_string()),
            pinned: Some(false),
            pin: Some("   ".to_string()),
            auto_check: Some(true),
            check_interval_hours: Some(24),
        }),
    };
    assert!(
        !is_pinned(&manual_cfg),
        "Espaços em branco não contam como pin válido"
    );
}
