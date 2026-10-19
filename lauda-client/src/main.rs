use lauda_client::App;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    #[cfg(any(
        feature = "desktop",
        feature = "mobile",
        feature = "web",
        target_arch = "wasm32"
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
        let is_tui = args.iter().any(|a| a == "tui") || args.len() <= 1;

        if is_tui {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Falha ao inicializar runtime tokio para TUI");
            if let Err(e) = rt.block_on(lauda_client::tui::runner::run_tui()) {
                eprintln!("Erro na execução da TUI do Lauda: {e}");
            }
        } else {
            println!("Lauda Smolclient — compile com --features desktop, mobile ou web.");
            println!("Ou execute com 'tui' para modo terminal interativo.");
            let _ = App;
        }
    }
}
