use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct LaudaConfig {
    pub update: Option<UpdateConfig>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct UpdateConfig {
    pub channel: Option<String>,
    pub pinned: Option<bool>,
    pub pin: Option<String>,
    pub auto_check: Option<bool>,
    pub check_interval_hours: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct UpdateState {
    pub last_check_timestamp: u64,
    pub latest_remote_version: Option<String>,
    pub channel: String,
}

/// Retorna o caminho padrão do arquivo de configuração segundo o padrão XDG
pub fn get_default_config_path() -> PathBuf {
    if let Some(xdg) = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        return PathBuf::from(xdg.trim()).join("lauda").join("config.toml");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("lauda")
            .join("config.toml");
    }
    PathBuf::from(".lauda_config.toml")
}

/// Retorna o caminho padrão do arquivo de estado em runtime segundo o padrão XDG
pub fn get_default_state_path() -> PathBuf {
    if let Some(xdg) = std::env::var("XDG_STATE_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        return PathBuf::from(xdg.trim())
            .join("lauda")
            .join("update_check.json");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("lauda")
            .join("update_check.json");
    }
    PathBuf::from(".lauda_update_check.json")
}

/// Carrega o arquivo de configuração persistido
pub fn load_config(path: Option<&Path>) -> LaudaConfig {
    let p = path
        .map(PathBuf::from)
        .unwrap_or_else(get_default_config_path);
    if !p.exists() {
        return LaudaConfig::default();
    }
    match fs::read_to_string(&p) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => LaudaConfig::default(),
    }
}

/// Salva a estrutura de configuração no disco
pub fn save_config(
    cfg: &LaudaConfig,
    path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let p = path
        .map(PathBuf::from)
        .unwrap_or_else(get_default_config_path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    let toml_str = toml::to_string_pretty(cfg)?;
    fs::write(&p, toml_str)?;
    Ok(())
}

/// Carrega o estado de verificação em runtime
pub fn load_update_state(path: Option<&Path>) -> Option<UpdateState> {
    let p = path
        .map(PathBuf::from)
        .unwrap_or_else(get_default_state_path);
    if !p.exists() {
        return None;
    }
    let content = fs::read_to_string(&p).ok()?;
    serde_json::from_str(&content).ok()
}

/// Salva o estado de verificação em runtime
pub fn save_update_state(
    state: &UpdateState,
    path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let p = path
        .map(PathBuf::from)
        .unwrap_or_else(get_default_state_path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    let json_str = serde_json::to_string_pretty(state)?;
    fs::write(&p, json_str)?;
    Ok(())
}

/// Salva o canal padrão no arquivo de configuração do usuário
pub fn save_default_channel(
    channel: &str,
    path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let p = path
        .map(PathBuf::from)
        .unwrap_or_else(get_default_config_path);
    let mut cfg = load_config(Some(&p));
    let mut update_cfg = cfg.update.unwrap_or_default();
    update_cfg.channel = Some(channel.to_string());
    cfg.update = Some(update_cfg);
    save_config(&cfg, Some(&p))
}

/// Verifica se a versão configurada está travada/pinada
pub fn is_pinned(config: &LaudaConfig) -> bool {
    if let Some(u) = &config.update {
        if u.pinned == Some(true) {
            return true;
        }
        if let Some(p) = &u.pin
            && !p.trim().is_empty()
        {
            return true;
        }
    }
    false
}

/// Fixa a versão desejada ou a versão corrente se omitida
pub fn set_pin(
    pin_version: Option<&str>,
    path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let p = path
        .map(PathBuf::from)
        .unwrap_or_else(get_default_config_path);
    let mut cfg = load_config(Some(&p));
    let mut u = cfg.update.unwrap_or_default();
    u.pinned = Some(true);
    let ver = match pin_version {
        Some(v) if !v.trim().is_empty() => v.trim().trim_start_matches('v').to_string(),
        _ => get_current_version().to_string(),
    };
    u.pin = Some(ver);
    cfg.update = Some(u);
    save_config(&cfg, Some(&p))
}

/// Remove a trava de versão (unpin)
pub fn unpin(path: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let p = path
        .map(PathBuf::from)
        .unwrap_or_else(get_default_config_path);
    let mut cfg = load_config(Some(&p));
    let mut u = cfg.update.unwrap_or_default();
    u.pinned = Some(false);
    u.pin = None;
    cfg.update = Some(u);
    save_config(&cfg, Some(&p))
}

/// Define o status de verificação automática periódica
pub fn set_auto_check(
    enabled: bool,
    path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let p = path
        .map(PathBuf::from)
        .unwrap_or_else(get_default_config_path);
    let mut cfg = load_config(Some(&p));
    let mut u = cfg.update.unwrap_or_default();
    u.auto_check = Some(enabled);
    cfg.update = Some(u);
    save_config(&cfg, Some(&p))
}

/// Retorna o valor de uma chave específica de configuração
pub fn get_config_value(key: &str, path: Option<&Path>) -> Option<String> {
    let cfg = load_config(path);
    let u = cfg.update?;
    match key {
        "channel" => u.channel,
        "pinned" => u.pinned.map(|b| b.to_string()),
        "pin" => u.pin,
        "auto_check" => u.auto_check.map(|b| b.to_string()),
        "check_interval_hours" => u.check_interval_hours.map(|h| h.to_string()),
        _ => None,
    }
}

/// Resolve o canal ativo seguindo a precedência: Flag > Env > Config > "stable"
pub fn resolve_channel(
    flag: Option<&str>,
    env_val: Option<&str>,
    config_path: Option<&Path>,
) -> String {
    if let Some(f) = flag.map(str::trim).filter(|s| !s.is_empty()) {
        return f.to_lowercase();
    }

    if let Some(e) = env_val.map(str::trim).filter(|s| !s.is_empty()) {
        return e.to_lowercase();
    }

    if let Ok(e) = std::env::var("LAUDA_CHANNEL") {
        let trimmed = e.trim();
        if !trimmed.is_empty() {
            return trimmed.to_lowercase();
        }
    }

    let cfg = load_config(config_path);
    if let Some(ch) = cfg
        .update
        .and_then(|u| u.channel)
        .filter(|s| !s.trim().is_empty())
    {
        return ch.trim().to_lowercase();
    }

    "stable".to_string()
}

/// Retorna os endpoints oficiais de distribuição na Banca: (stable_url, nightly_url, version_url)
pub fn get_distribution_endpoints(channel: &str) -> (String, String, String) {
    let stable = "https://banca.2lp.in/downloads/lauda/stable-latest/lauda".to_string();
    let nightly = "https://banca.2lp.in/downloads/lauda/nightly-latest/lauda".to_string();
    let ver = if channel == "nightly" {
        "https://banca.2lp.in/downloads/lauda/nightly-version".to_string()
    } else {
        "https://banca.2lp.in/downloads/lauda/version".to_string()
    };
    (stable, nightly, ver)
}

/// Retorna a URL de download correspondente à seleção
pub fn get_download_url(channel: &str, pin: Option<&str>) -> String {
    if let Some(p) = pin {
        let clean = p.trim().trim_start_matches('v');
        format!(
            "https://banca.2lp.in/downloads/lauda/releases/v{}/lauda",
            clean
        )
    } else if channel == "nightly" {
        "https://banca.2lp.in/downloads/lauda/nightly-latest/lauda".to_string()
    } else {
        "https://banca.2lp.in/downloads/lauda/stable-latest/lauda".to_string()
    }
}

/// Compara se a versão remota é mais recente que a atual
pub fn semver_is_newer(remote: &str, current: &str) -> bool {
    let r_clean = remote.trim().trim_start_matches('v');
    let c_clean = current.trim().trim_start_matches('v');

    if r_clean == c_clean {
        return false;
    }

    if let (Ok(r_ver), Ok(c_ver)) = (
        semver::Version::parse(r_clean),
        semver::Version::parse(c_clean),
    ) {
        return r_ver > c_ver;
    }

    // Fallback para strings dev com timestamp
    r_clean > c_clean
}

/// Obtém a versão compilada atual do Lauda
pub fn get_current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Verificação periódica não-bloqueante no startup
pub fn check_for_updates_startup(
    config_path: Option<&Path>,
    state_path: Option<&Path>,
) -> Option<String> {
    let cfg = load_config(config_path);

    // 1. Se estiver desativado ou pinado, silencia
    let auto_check = cfg
        .update
        .as_ref()
        .and_then(|u| u.auto_check)
        .unwrap_or(true);
    if !auto_check || is_pinned(&cfg) {
        return None;
    }

    let channel = resolve_channel(None, None, config_path);
    let current_ver = get_current_version();
    let interval_hours = cfg
        .update
        .as_ref()
        .and_then(|u| u.check_interval_hours)
        .unwrap_or(24);
    let interval_secs = interval_hours.saturating_mul(3600);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let cached_state = load_update_state(state_path);
    let mut latest_remote = None;

    if let Some(state) = &cached_state
        && state.channel == channel
        && now.saturating_sub(state.last_check_timestamp) < interval_secs
    {
        latest_remote = state.latest_remote_version.clone();
    }

    // Se não há cache válido, efetua checagem HTTP leve com timeout curto (<= 2s)
    if latest_remote.is_none() {
        let (_, _, ver_url) = get_distribution_endpoints(&channel);
        let ureq_cfg = ureq::config::Config::builder()
            .timeout_global(Some(Duration::from_secs(2)))
            .build();
        let agent = ureq::Agent::new_with_config(ureq_cfg);
        if let Ok(mut body) = agent.get(&ver_url).call().map(|r| r.into_body())
            && let Ok(text) = body.read_to_string()
        {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                latest_remote = Some(trimmed.to_string());
                let new_state = UpdateState {
                    last_check_timestamp: now,
                    latest_remote_version: Some(trimmed.to_string()),
                    channel: channel.clone(),
                };
                let _ = save_update_state(&new_state, state_path);
            }
        }
    }

    if let Some(remote) = latest_remote
        && semver_is_newer(&remote, current_ver)
    {
        let msg = format!(
            "aviso: nova versão do Lauda disponível: v{} -> v{}. Execute 'lauda update' para atualizar.",
            current_ver, remote
        );
        eprintln!("{msg}");
        return Some(msg);
    }

    None
}

/// Consulta o endpoint da Banca para checagem de atualizações
pub fn cmd_check_update(channel_override: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let channel = resolve_channel(channel_override, None, None);
    let current_ver = get_current_version();
    let (_, _, ver_url) = get_distribution_endpoints(&channel);

    println!("⚡ Lauda Updater // Banca 2lp.in");
    println!("Canal de Distribuição : {}", channel);
    println!("Versão Local Atual    : v{}", current_ver);
    println!("Endpoint de Consulta  : {}", ver_url);

    let ureq_cfg = ureq::config::Config::builder()
        .timeout_global(Some(Duration::from_secs(3)))
        .build();
    let agent = ureq::Agent::new_with_config(ureq_cfg);

    let mut remote_ver = String::new();
    if let Ok(mut body) = agent.get(&ver_url).call().map(|r| r.into_body())
        && let Ok(text) = body.read_to_string()
    {
        remote_ver = text.trim().to_string();
    }

    if remote_ver.is_empty() {
        println!("Versão Remota         : indisponível (servidor offline ou rota não publicada)");
        println!("\nNão foi possível validar atualizações no momento.");
        return Ok(());
    }

    println!("Versão Remota         : v{}", remote_ver);

    if semver_is_newer(&remote_ver, current_ver) {
        println!(
            "\n★ Nova versão disponível: v{} -> v{}",
            current_ver, remote_ver
        );
        println!("Para atualizar agora, execute:");
        println!("  lauda update --channel {}", channel);
    } else {
        println!(
            "\n✔ O Lauda já está atualizado na versão mais recente do canal {}.",
            channel
        );
    }

    Ok(())
}

/// Executa a atualização atômica do binário
pub fn cmd_update(
    channel_override: Option<&str>,
    pin: Option<&str>,
    force: bool,
    save_channel: bool,
    target_override: Option<&Path>,
    config_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = load_config(config_path);

    // Validação de Pinning: bloqueia sem --force se já estiver pinado
    if is_pinned(&cfg) && !force && pin.is_none() {
        let pinned_ver = cfg
            .update
            .as_ref()
            .and_then(|u| u.pin.as_deref())
            .unwrap_or("fixada");
        return Err(format!(
            "Atualização bloqueada: versão do Lauda está fixada (pin: v{}). Use 'lauda update --force' ou 'lauda config unpin' para prosseguir.",
            pinned_ver
        )
        .into());
    }

    let channel = resolve_channel(channel_override, None, config_path);

    if save_channel {
        save_default_channel(&channel, config_path)?;
        println!("✔ Canal padrão salvo com sucesso: '{}'", channel);
    }

    if let Some(p) = pin {
        set_pin(Some(p), config_path)?;
        println!("✔ Versão fixada (pinned) salva com sucesso: 'v{}'", p);
    }

    let download_url = get_download_url(&channel, pin);
    let current_ver = get_current_version();

    println!("⚡ Lauda Updater // Atualização Manual");
    println!("Canal Selecionado : {}", channel);
    if let Some(p) = pin {
        println!("Versão Fixada     : v{}", p);
    }
    println!("Versão Atual      : v{}", current_ver);
    println!("Baixando de       : {}", download_url);

    let target_path = if let Some(t) = target_override {
        t.to_path_buf()
    } else {
        std::env::current_exe()?
    };

    let ureq_cfg = ureq::config::Config::builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .build();
    let agent = ureq::Agent::new_with_config(ureq_cfg);

    let resp = agent
        .get(&download_url)
        .call()
        .map_err(|e| format!("Falha ao conectar com {}: {}", download_url, e))?;

    let bytes = resp
        .into_body()
        .read_to_vec()
        .map_err(|e| format!("Falha ao baixar payload do executável: {}", e))?;

    if bytes.is_empty() {
        return Err("Arquivo baixado está vazio.".into());
    }

    let target_dir = target_path
        .parent()
        .ok_or("Não foi possível identificar o diretório do binário alvo.")?;

    let mut temp_file = tempfile::Builder::new()
        .prefix(".lauda-update-")
        .tempfile_in(target_dir)?;

    temp_file.write_all(&bytes)?;
    temp_file.flush()?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(temp_file.path(), perms)?;
    }

    temp_file.persist(&target_path)?;

    println!(
        "✔ Lauda atualizado com sucesso em '{}'!",
        target_path.display()
    );
    Ok(())
}
