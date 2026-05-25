// SC-61 — CT lifecycle on fixture CT 200. cgroup v2 gated.
//
// Note: PVE has no REST endpoint for `pct exec` (CLI-only). The TS
// reference test omits the exec step for the same reason, asserting
// start → status running → stop → status stopped only.

mod common;

use std::time::Duration;

use clientapi_pve::apis::lxc_api;
use clientapi_pve::models::PveStatusEnum;
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
    wait_for_status(&cfg, &node, FIXTURE_CTID, PveStatusEnum::Stopped, 30)
        .await
        .expect("CT stopped at start");

    lxc_api::lxc_vm_start(&cfg, &node, FIXTURE_CTID, None)
        .await
        .expect("pct start 200");
    wait_for_status(&cfg, &node, FIXTURE_CTID, PveStatusEnum::Running, 60)
        .await
        .expect("CT running within 60 s");

    lxc_api::lxc_vm_stop(&cfg, &node, FIXTURE_CTID, None)
        .await
        .expect("pct stop 200");
    wait_for_status(&cfg, &node, FIXTURE_CTID, PveStatusEnum::Stopped, 60)
        .await
        .expect("CT stopped within 60 s");
}

async fn wait_for_status(
    cfg: &clientapi_pve::apis::configuration::Configuration,
    node: &str,
    vmid: i32,
    expected: PveStatusEnum,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    wait_until(
        &format!("ct {vmid} → {expected:?}"),
        Duration::from_secs(timeout_secs),
        Duration::from_millis(500),
        || async {
            let resp = lxc_api::lxc_vm_status(cfg, node, vmid)
                .await
                .map_err(|e| anyhow::anyhow!("ct_status: {e}"))?;
            if resp.data.status == expected {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        },
    )
    .await
}
