use lauda_client::App;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    #[cfg(feature = "desktop")]
    {
        let window = dioxus::desktop::WindowBuilder::new().with_title("Lauda");
        let config = dioxus::desktop::Config::new().with_window(window);
        dioxus::LaunchBuilder::desktop()
            .with_cfg(config)
            .launch(App);
    }

    #[cfg(all(
        not(feature = "desktop"),
        any(feature = "mobile", feature = "web", target_arch = "wasm32")
    ))]
    {
        dioxus::launch(App);
    }

    #[cfg(not(any(
        feature = "desktop",
        feature = "mobile",
        feature = "web",
        target_arch = "wasm32"
    )))]
    {
        // Verificação periódica não-bloqueante no startup de comandos CLI ou TUI
        let _ = lauda_client::update::check_for_updates_startup(None, None);

        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            let cmd = args[1].as_str();
            match cmd {
                "update" => {
                    let mut channel = None;
                    let mut pin = None;
                    let mut force = false;
                    let mut save = false;
                    let mut check = false;
                    let mut i = 2;
                    while i < args.len() {
                        match args[i].as_str() {
                            "--channel" | "-c" => {
                                if i + 1 < args.len() {
                                    channel = Some(args[i + 1].clone());
                                    i += 1;
                                }
                            }
                            "--pin" | "-p" => {
                                if i + 1 < args.len() {
                                    pin = Some(args[i + 1].clone());
                                    i += 1;
                                }
                            }
                            "--force" | "-f" => force = true,
                            "--save" => save = true,
                            "--check" => check = true,
                            _ => {}
                        }
                        i += 1;
                    }
                    if check {
                        if let Err(e) = lauda_client::update::cmd_check_update(channel.as_deref()) {
                            eprintln!("Erro: {e}");
                            std::process::exit(1);
                        }
                    } else if let Err(e) = lauda_client::update::cmd_update(
                        channel.as_deref(),
                        pin.as_deref(),
                        force,
                        save,
                        None,
                        None,
                    ) {
                        eprintln!("Erro ao atualizar: {e}");
                        std::process::exit(1);
                    }
                    return;
                }
                "check-update" => {
                    let mut channel = None;
                    let mut i = 2;
                    while i < args.len() {
                        if (args[i] == "--channel" || args[i] == "-c") && i + 1 < args.len() {
                            channel = Some(args[i + 1].clone());
                            i += 1;
                        }
                        i += 1;
                    }
                    if let Err(e) = lauda_client::update::cmd_check_update(channel.as_deref()) {
                        eprintln!("Erro: {e}");
                        std::process::exit(1);
                    }
                    return;
                }
                "config" => {
                    if args.len() > 2 {
                        match args[2].as_str() {
                            "set-channel" if args.len() > 3 => {
                                if let Err(e) =
                                    lauda_client::update::save_default_channel(&args[3], None)
                                {
                                    eprintln!("Erro ao salvar canal: {e}");
                                    std::process::exit(1);
                                }
                                println!("✔ Canal padrão do Lauda atualizado para '{}'.", args[3]);
                                return;
                            }
                            "pin" => {
                                let ver = if args.len() > 3 {
                                    Some(args[3].as_str())
                                } else {
                                    None
                                };
                                if let Err(e) = lauda_client::update::set_pin(ver, None) {
                                    eprintln!("Erro ao fixar versão: {e}");
                                    std::process::exit(1);
                                }
                                let pinned_ver =
                                    lauda_client::update::get_config_value("pin", None)
                                        .unwrap_or_default();
                                println!(
                                    "✔ Versão do Lauda fixada com sucesso em 'v{}'.",
                                    pinned_ver
                                );
                                return;
                            }
                            "unpin" => {
                                if let Err(e) = lauda_client::update::unpin(None) {
                                    eprintln!("Erro ao desfixar versão: {e}");
                                    std::process::exit(1);
                                }
                                println!(
                                    "✔ Trava de versão removida (unpinned). Atualizações habilitadas."
                                );
                                return;
                            }
                            "set-auto-check" if args.len() > 3 => {
                                let val: bool = match args[3].to_lowercase().as_str() {
                                    "true" | "1" | "yes" | "sim" => true,
                                    "false" | "0" | "no" | "nao" => false,
                                    other => {
                                        eprintln!(
                                            "Valor inválido para auto-check: '{}'. Esperado: true ou false.",
                                            other
                                        );
                                        std::process::exit(1);
                                    }
                                };
                                if let Err(e) = lauda_client::update::set_auto_check(val, None) {
                                    eprintln!("Erro ao configurar auto-check: {e}");
                                    std::process::exit(1);
                                }
                                println!(
                                    "✔ Verificação periódica no startup configurada para '{}'.",
                                    val
                                );
                                return;
                            }
                            "get" if args.len() > 3 => {
                                let key = &args[3];
                                match lauda_client::update::get_config_value(key, None) {
                                    Some(val) => println!("{val}"),
                                    None => println!("(não configurado)"),
                                }
                                return;
                            }
                            "show" => {
                                let config_path = lauda_client::update::get_default_config_path();
                                let state_path = lauda_client::update::get_default_state_path();
                                let active_ch =
                                    lauda_client::update::resolve_channel(None, None, None);
                                let cfg = lauda_client::update::load_config(None);
                                let u = cfg.update.unwrap_or_default();
                                println!("Arquivo de configuração : {}", config_path.display());
                                println!("Arquivo de estado (XDG) : {}", state_path.display());
                                println!("Canal ativo resolvido  : {}", active_ch);
                                println!(
                                    "Canal configurado       : {}",
                                    u.channel.as_deref().unwrap_or("padrão (stable)")
                                );
                                println!(
                                    "Versão pinada           : {}",
                                    if u.pinned == Some(true) {
                                        u.pin.as_deref().unwrap_or("sim")
                                    } else {
                                        "não"
                                    }
                                );
                                println!(
                                    "Auto-check no startup   : {}",
                                    u.auto_check.unwrap_or(true)
                                );
                                println!(
                                    "Intervalo de verificação: {}h",
                                    u.check_interval_hours.unwrap_or(24)
                                );
                                return;
                            }
                            _ => {}
                        }
                    }
                    println!("Uso: lauda config <subcomando>");
                    println!("  show                      Exibe caminhos e configurações ativas");
                    println!("  get <chave>               Obtém o valor de uma chave específica");
                    println!(
                        "  pin [<versao>]            Fixa versão atual ou informada contra atualizações"
                    );
                    println!("  unpin                     Remove fixação de versão");
                    println!(
                        "  set-auto-check <bool>     Habilita ou desabilita verificação no startup"
                    );
                    println!(
                        "  set-channel <canal>       Define o canal padrão (stable ou nightly)"
                    );
                    return;
                }
                "sync-daemon" => {
                    let session = match lauda_client::domain::session::load_session(None) {
                        Some(s) if s.token.is_some() => s,
                        _ => {
                            eprintln!(
                                "Erro: Nenhuma sessão ativa encontrada. Efetue login previamente via interface ou TUI."
                            );
                            std::process::exit(1);
                        }
                    };

                    let token = session.token.unwrap();
                    let server_url = session.server_url;
                    let cache_dir = lauda_client::domain::cache::get_default_cache_dir();
                    let client = std::sync::Arc::new(
                        lauda_client::adapters::http::HttpApiClient::new(&server_url),
                    );
                    let config = lauda_client::domain::sync::SyncWorkerConfig::default();

                    println!(
                        "Iniciando Lauda Sync Daemon para {} em {}...",
                        session.username, server_url
                    );
                    let (worker, _handle) = lauda_client::domain::sync::BackgroundSyncWorker::new(
                        client,
                        token,
                        "INBOX".to_string(),
                        cache_dir,
                        config,
                    );

                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build()
                        .expect("Falha ao inicializar runtime tokio para Sync Daemon");

                    rt.block_on(worker.run());
                    return;
                }
                "tui" => {
                    // segue para execução tui
                }
                _ => {
                    println!("Lauda Smolclient — compile com --features desktop, mobile ou web.");
                    println!(
                        "Comandos disponíveis: tui, sync-daemon, update, check-update, config."
                    );
                    let _ = App;
                    return;
                }
            }
        }

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Falha ao inicializar runtime tokio para TUI");
        if let Err(e) = rt.block_on(lauda_client::tui::runner::run_tui()) {
            eprintln!("Erro na execução da TUI do Lauda: {e}");
        }
    }
}
