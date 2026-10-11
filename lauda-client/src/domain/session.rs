use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Configuração persistida de sessão local do Lauda.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SessionConfig {
    pub server_url: String,
    pub username: String,
    pub token: Option<String>,
}

/// Obtém a URL padrão do servidor Lauda baseando-se no ambiente de execução.
pub fn detect_default_server_url() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window()
            && let Ok(loc) = win.location().origin()
            && !loc.is_empty()
            && loc != "null"
        {
            // Se estiver rodando em lauda.lohn.in ou lauda.2lp.in
            if loc.contains("lohn.in") {
                return "https://lauda.lohn.in".to_string();
            }
            if loc.contains("2lp.in") {
                return "https://lauda.lohn.in".to_string();
            }
            return loc;
        }
        "https://lauda.lohn.in".to_string()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(env_url) = std::env::var("LAUDA_SERVER_URL")
            && !env_url.is_empty()
        {
            return env_url;
        }
        #[cfg(target_os = "android")]
        {
            "https://lauda.lohn.in".to_string()
        }
        #[cfg(not(target_os = "android"))]
        {
            "http://127.0.0.1:8432".to_string()
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            server_url: detect_default_server_url(),
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
        if Path::new("/data/data/in.twolp.lauda/files").exists() {
            return PathBuf::from("/data/data/in.twolp.lauda/files/session.json");
        }
        PathBuf::from("/data/data/in.twolp.lauda/session.json")
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

/// Carrega a sessão do disco ou localStorage (em WASM). Retorna None se não existir ou for inválido.
pub fn load_session(path: Option<&Path>) -> Option<SessionConfig> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        let window = web_sys::window()?;
        let storage = window.local_storage().ok()??;
        let content = storage.get_item("lauda_session").ok()??;
        serde_json::from_str(&content).ok()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
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
}

/// Grava a sessão no disco de forma determinística ou no localStorage (em WASM).
pub fn save_session(config: &SessionConfig, path: Option<&Path>) -> Result<(), std::io::Error> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        if let Some(window) = web_sys::window()
            && let Ok(Some(storage)) = window.local_storage()
            && let Ok(json) = serde_json::to_string(config)
        {
            let _ = storage.set_item("lauda_session", &json);
        }
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
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
}

/// Remove a sessão persistida (ex: ao fazer logout).
pub fn clear_session(path: Option<&Path>) -> Result<(), std::io::Error> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        if let Some(window) = web_sys::window()
            && let Ok(Some(storage)) = window.local_storage()
        {
            let _ = storage.remove_item("lauda_session");
        }
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let resolved_path = match path {
            Some(p) => p.to_path_buf(),
            None => get_default_session_path(),
        };

        if resolved_path.exists() {
            std::fs::remove_file(resolved_path)?;
        }
        Ok(())
    }
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
