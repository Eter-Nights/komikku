mod frb_generated;

mod api {
    #[flutter_rust_bridge::frb(sync)] // Synchronous mode for simplicity of the demo
    pub fn greet(name: String) -> String {
        format!("Hello, {name}!")
    }
}
