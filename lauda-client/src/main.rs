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
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            let cmd = args[1].as_str();
            match cmd {
                "update" => {
                    let mut channel = None;
                    let mut pin = None;
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
                        save,
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
                    if args.len() > 3 && args[2] == "set-channel" {
                        if let Err(e) = lauda_client::update::save_default_channel(&args[3], None) {
                            eprintln!("Erro ao salvar canal: {e}");
                            std::process::exit(1);
                        }
                        println!("✔ Canal padrão do Lauda atualizado para '{}'.", args[3]);
                        return;
                    } else if args.len() > 2 && args[2] == "show" {
                        let path = lauda_client::update::get_default_config_path();
                        let active = lauda_client::update::resolve_channel(None, None, None);
                        println!("Arquivo de configuração : {}", path.display());
                        println!("Canal ativo resolvido  : {}", active);
                        return;
                    } else {
                        println!("Uso: lauda config set-channel <channel> | lauda config show");
                        return;
                    }
                }
                "tui" => {
                    // segue para execução tui
                }
                _ => {
                    println!("Lauda Smolclient — compile com --features desktop, mobile ou web.");
                    println!("Comandos disponíveis: tui, update, check-update, config.");
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
