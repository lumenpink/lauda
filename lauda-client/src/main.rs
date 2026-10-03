use lauda_client::App;

fn main() {
    #[cfg(any(feature = "desktop", feature = "mobile"))]
    {
        dioxus::launch(App);
    }

    #[cfg(not(any(feature = "desktop", feature = "mobile")))]
    {
        println!("Lauda Smolclient — compile com --features desktop ou --features mobile.");
        let _ = App;
    }
}
