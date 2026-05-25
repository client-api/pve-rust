// SC-61 — CT lifecycle on fixture CT 200. cgroup v2 gated.
//
// Note: PVE has no REST endpoint for `pct exec` (CLI-only). The TS
// reference test omits the exec step for the same reason, asserting
// start → status running → stop → status stopped only.

mod common;

use std::time::Duration;

use clientapi_pve::apis::lxc_api;
use common::*;

const FIXTURE_CTID: i32 = 200;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_61_ct_lifecycle() {
    let creds = Credentials::from_env();
    skip_if_no_cgroupv2!();

    let cfg = creds.config_with_token();
    let node = first_node(&cfg).await.expect("first_node");

    if lxc_api::lxc_vm_config(&cfg, &node, FIXTURE_CTID, None, None)
        .await
        .is_err()
    {
        eprintln!("SKIP: fixture CT {FIXTURE_CTID} is not present (seed disabled?)");
        return;
    }

    let _ = lxc_api::lxc_vm_stop(&cfg, &node, FIXTURE_CTID, None).await;
    wait_for_status(&creds, &node, FIXTURE_CTID, "stopped", 30)
        .await
        .expect("CT stopped at start");

    lxc_api::lxc_vm_start(&cfg, &node, FIXTURE_CTID, None)
        .await
        .expect("pct start 200");
    wait_for_status(&creds, &node, FIXTURE_CTID, "running", 60)
        .await
        .expect("CT running within 60 s");

    lxc_api::lxc_vm_stop(&cfg, &node, FIXTURE_CTID, None)
        .await
        .expect("pct stop 200");
    wait_for_status(&creds, &node, FIXTURE_CTID, "stopped", 60)
        .await
        .expect("CT stopped within 60 s");
}

async fn wait_for_status(
    creds: &Credentials,
    node: &str,
    vmid: i32,
    expected: &str,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    // Raw GET instead of the SDK's lxc_vm_status — the response model has
    // f64 fields (cpu, pressure*) that PVE wires as JSON strings ("0.00"),
    // making serde reject every poll. See common/raw_status.rs.
    wait_until(
        &format!("ct {vmid} → {expected}"),
        Duration::from_secs(timeout_secs),
        Duration::from_millis(500),
        || async {
            let status = raw_status(creds, node, "lxc", vmid)
                .await
                .map_err(|e| anyhow::anyhow!("ct_status: {e}"))?;
            if status == expected {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        },
    )
    .await
}
