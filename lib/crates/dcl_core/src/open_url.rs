/// A request from the core to the host to show a URL (browser or webview).
#[derive(Debug, Clone)]
pub struct OpenUrlRequest {
    pub url: String,
    pub description: String,
    pub use_webview: bool,
}
