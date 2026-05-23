//! Example: resilient terminal session with auto-reconnect.
//!
//! Run with:
//!
//! ```sh
//! PVE_HOST=https://pve.example.com:8006 \
//! PVE_TOKEN='root@pam!auto=...' \
//! PVE_NODE=orca PVE_VMID=100 \
//! cargo run --example resilient_terminal --features extras
//! ```

use std::env;
use std::time::Duration;

use clientapi_pve::apis::configuration::Configuration;
use clientapi_pve::websocket::TerminalTarget;
use clientapi_pve::websocket_resilient::{connect_terminal_resilient, RetryOptions};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = Configuration::new();
    cfg.base_path = format!(
        "{}/api2/json",
        env::var("PVE_HOST").unwrap_or_else(|_| "https://localhost:8006".into()),
    );
    cfg.bearer_access_token = env::var("PVE_TOKEN").ok();

    let node = env::var("PVE_NODE").unwrap_or_else(|_| "pve1".into());
    let vmid: i32 = env::var("PVE_VMID")
        .unwrap_or_else(|_| "100".into())
        .parse()?;

    let opts = RetryOptions {
        max_retries: 20,
        initial_delay: Duration::from_millis(250),
        max_delay: Duration::from_secs(30),
        backoff_multiplier: 2.0,
    };

    let mut session = connect_terminal_resilient(
        cfg,
        TerminalTarget::Qemu { node, vmid },
        opts,
    )
    .await?;

    session.send("date\n").await?;

    // Long-running session: command every 30 s for 5 minutes, with
    // recv() driving the inbound pump (transparently reconnects).
    let deadline = std::time::Instant::now() + Duration::from_secs(5 * 60);
    let mut next_cmd = std::time::Instant::now() + Duration::from_secs(30);
    while std::time::Instant::now() < deadline {
        // timeout returns Result<Result<Option<String>, WsError>, Elapsed>
        match tokio::time::timeout(Duration::from_secs(1), session.recv()).await {
            Ok(Ok(Some(text))) => print!("{text}"),
            Ok(Ok(None)) => break,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {} // 1 s timeout — keep looping
        }
        if std::time::Instant::now() >= next_cmd {
            session.send("date\n").await?;
            next_cmd = std::time::Instant::now() + Duration::from_secs(30);
        }
    }

    session.close().await?;
    Ok(())
}
