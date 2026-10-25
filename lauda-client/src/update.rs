use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct LaudaConfig {
    pub update: Option<UpdateConfig>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct UpdateConfig {
    pub channel: Option<String>,
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

/// Salva o canal padrão no arquivo de configuração do usuário
pub fn save_default_channel(
    channel: &str,
    path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let p = path
        .map(PathBuf::from)
        .unwrap_or_else(get_default_config_path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut cfg = load_config(Some(&p));
    let mut update_cfg = cfg.update.unwrap_or_default();
    update_cfg.channel = Some(channel.to_string());
    cfg.update = Some(update_cfg);

    let toml_str = toml::to_string_pretty(&cfg)?;
    fs::write(&p, toml_str)?;
    Ok(())
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

/// Retorna os endpoints soberanos oficiais de distribuição na Banca: (stable_url, nightly_url, version_url)
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

/// Consulta o endpoint da Banca para checagem de atualizações
pub fn cmd_check_update(channel_override: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let channel = resolve_channel(channel_override, None, None);
    let current_ver = get_current_version();
    let (_, _, ver_url) = get_distribution_endpoints(&channel);

    println!("⚡ Lauda Updater // Banca Soberana 2lp.in");
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
    save_channel: bool,
    target_override: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let channel = resolve_channel(channel_override, None, None);

    if save_channel {
        save_default_channel(&channel, None)?;
        println!("✔ Canal padrão salvo com sucesso: '{}'", channel);
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
