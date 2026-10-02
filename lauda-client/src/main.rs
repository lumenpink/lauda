use lauda_client::App;

fn main() {
    #[cfg(feature = "desktop")]
    {
        dioxus::launch(App);
    }

    #[cfg(not(feature = "desktop"))]
    {
        println!("Lauda Smolclient — compile com --features desktop para interface de janela.");
        let _ = App;
    }
}
