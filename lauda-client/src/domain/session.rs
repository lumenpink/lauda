use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Configuração persistida de sessão local do Lauda.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SessionConfig {
    pub server_url: String,
    pub username: String,
    pub token: Option<String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        #[cfg(target_os = "android")]
        let default_server = "http://10.0.2.2:8432".to_string();
        #[cfg(not(target_os = "android"))]
        let default_server = "http://127.0.0.1:8432".to_string();

        Self {
            server_url: default_server,
            username: "lumen".to_string(),
            token: None,
        }
    }
}

/// Obtém o caminho padrão para gravação do arquivo de sessão conforme a plataforma.
pub fn get_default_session_path() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        if let Ok(home) = std::env::var("HOME")
            && !home.is_empty()
        {
            return PathBuf::from(home).join("session.json");
        }
        if Path::new("/data/data/com.example.LaudaClient/files").exists() {
            return PathBuf::from("/data/data/com.example.LaudaClient/files/session.json");
        }
        PathBuf::from("/data/data/com.example.LaudaClient/session.json")
    }
    #[cfg(not(target_os = "android"))]
    {
        if let Ok(path) = std::env::var("LAUDA_SESSION_PATH")
            && !path.is_empty()
        {
            return PathBuf::from(path);
        }
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
            && !xdg.is_empty()
        {
            return PathBuf::from(xdg).join("lauda/session.json");
        }
        if let Ok(home) = std::env::var("HOME")
            && !home.is_empty()
        {
            return PathBuf::from(home).join(".config/lauda/session.json");
        }
        PathBuf::from("session.json")
    }
}

/// Carrega a sessão do disco. Retorna None se o arquivo não existir ou for inválido.
pub fn load_session(path: Option<&Path>) -> Option<SessionConfig> {
    let resolved_path = match path {
        Some(p) => p.to_path_buf(),
        None => get_default_session_path(),
    };

    if !resolved_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&resolved_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Grava a sessão no disco de forma determinística.
pub fn save_session(config: &SessionConfig, path: Option<&Path>) -> Result<(), std::io::Error> {
    let resolved_path = match path {
        Some(p) => p.to_path_buf(),
        None => get_default_session_path(),
    };

    if let Some(parent) = resolved_path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&resolved_path, json)?;
    Ok(())
}

/// Remove a sessão persistida (ex: ao fazer logout).
pub fn clear_session(path: Option<&Path>) -> Result<(), std::io::Error> {
    let resolved_path = match path {
        Some(p) => p.to_path_buf(),
        None => get_default_session_path(),
    };

    if resolved_path.exists() {
        std::fs::remove_file(resolved_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_lifecycle() {
        let temp = tempfile::tempdir().expect("tempdir");
        let session_file = temp.path().join("test_session.json");

        assert_eq!(load_session(Some(&session_file)), None);

        let config = SessionConfig {
            server_url: "http://127.0.0.1:3000".to_string(),
            username: "lumen".to_string(),
            token: Some("secret-token-123".to_string()),
        };

        save_session(&config, Some(&session_file)).expect("save session");

        let loaded = load_session(Some(&session_file)).expect("load session");
        assert_eq!(loaded, config);

        clear_session(Some(&session_file)).expect("clear session");
        assert_eq!(load_session(Some(&session_file)), None);
    }
}
