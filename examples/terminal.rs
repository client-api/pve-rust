//! Example: open a terminal session against a QEMU VM.
//!
//! Run with:
//!
//! ```sh
//! PVE_HOST=https://pve.example.com:8006 \
//! PVE_TOKEN='root@pam!auto=...' \
//! PVE_NODE=orca PVE_VMID=100 \
//! cargo run --example terminal --features extras
//! ```

use std::env;
use std::time::Duration;

use clientapi_pve::apis::configuration::Configuration;
use clientapi_pve::websocket::{connect_terminal, TerminalTarget};

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

    println!("Opening terminal on {}:qemu/{}...", node, vmid);
    let mut session = connect_terminal(&cfg, TerminalTarget::Qemu { node, vmid }).await?;

    session.resize(120, 32).await?;
    session.send("uname -a\n").await?;

    // Read with a 5 s overall timeout.
    let _ = tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(Some(msg)) = session.recv().await {
            print!("{msg}");
        }
    })
    .await;

    session.close().await?;
    Ok(())
}
