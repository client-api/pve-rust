//! Example: connect to a Proxmox host with a self-signed certificate.
//!
//! The PVE web UI ships with a self-signed cert by default. Production
//! setups should use a real CA-signed cert (Let's Encrypt via the
//! Proxmox UI), but home-lab and dev setups commonly need to opt out
//! of cert verification.
//!
//! **Security note:** disabling verification is vulnerable to MITM.
//! Use only on trusted networks.
//!
//! Run with:
//!
//! ```sh
//! PVE_HOST=https://pve.example.com:8006 \
//! PVE_TOKEN='root@pam!auto=...' \
//! PVE_NODE=orca PVE_VMID=100 \
//! cargo run --example insecure_tls --features extras
//! ```

use std::env;
use std::time::Duration;

use openapi::apis::configuration::Configuration;
use openapi::apis::nodes_api;
use openapi::websocket::{
    AuthAttacher, ConsoleConnector, TerminalSession, TerminalTarget, WebSocketTransport, WsError,
    WsStream,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = env::var("PVE_HOST").unwrap_or_else(|_| "https://localhost:8006".into());

    // ── 1. HTTP: build a reqwest client that accepts invalid certs and
    //    inject it into Configuration.client.
    let insecure_http = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let mut cfg = Configuration::new();
    cfg.base_path = format!("{}/api2/json", host);
    cfg.bearer_access_token = env::var("PVE_TOKEN").ok();
    cfg.client = insecure_http;

    let resp = nodes_api::nodes_get_nodes(&cfg).await?;
    let nodes = resp.data;
    println!("Connected (insecure TLS): {} node(s)", nodes.len());

    // ── 2. WebSocket: provide a custom WebSocketTransport that wires
    //    a permissive TLS connector into tokio-tungstenite.
    struct InsecureTransport;

    #[async_trait::async_trait]
    impl WebSocketTransport for InsecureTransport {
        async fn open(
            &self,
            url: &str,
            auth: &dyn AuthAttacher,
            cfg: &Configuration,
        ) -> Result<WsStream, WsError> {
            use tokio_tungstenite::tungstenite::client::IntoClientRequest;

            let mut req = url.into_client_request()?;
            auth.apply(&mut req, cfg)?;

            // Build a native-tls connector that ignores cert verification.
            let connector = native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true)
                .build()
                .map_err(|e| WsError::BadBasePath(e.to_string()))?;
            let tls = tokio_tungstenite::Connector::NativeTls(connector);

            let (stream, _) = tokio_tungstenite::connect_async_tls_with_config(
                req,
                None,
                false,
                Some(tls),
            )
            .await?;
            Ok(stream)
        }
    }

    let connector = ConsoleConnector::new(
        Box::new(DefaultAuthForExample),
        std::sync::Arc::new(InsecureTransport),
    );

    let target = TerminalTarget::Qemu {
        node: env::var("PVE_NODE").unwrap_or_else(|_| "pve1".into()),
        vmid: env::var("PVE_VMID").unwrap_or_else(|_| "100".into()).parse()?,
    };

    let mut session: TerminalSession = connector.open_terminal(&cfg, target).await?;
    session.send("uname -a\n").await?;
    let _ = tokio::time::timeout(Duration::from_secs(3), async {
        while let Ok(Some(msg)) = session.recv().await {
            print!("{msg}");
        }
    })
    .await;
    session.close().await?;
    Ok(())
}

/// Mirror of the SDK's default auth attacher (Authorization +
/// Cookie). The `default_auth()` symbol is private; reproducing the
/// behavior here keeps the example self-contained.
struct DefaultAuthForExample;

impl AuthAttacher for DefaultAuthForExample {
    fn apply(
        &self,
        req: &mut tokio_tungstenite::tungstenite::http::Request<()>,
        cfg: &Configuration,
    ) -> Result<(), WsError> {
        if let Some(token) = cfg.bearer_access_token.as_ref() {
            let header =
                tokio_tungstenite::tungstenite::http::HeaderValue::from_str(&format!("PVEAPIToken={}", token))?;
            req.headers_mut().insert("authorization", header);
        }
        Ok(())
    }
}
