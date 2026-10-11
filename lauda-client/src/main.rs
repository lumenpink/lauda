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
        println!("Lauda Smolclient — compile com --features desktop, mobile ou web.");
        let _ = App;
    }
}
