// SC-60 — VM lifecycle on fixture VM 100. KVM-gated.
//
// Per the rollout decision: call qemu_vm_shutdown first, poll up to 30 s
// for stopped, then fall back to qemu_vm_stop on timeout. This covers the
// happy ACPI path AND the flaky-fixture path the TS reference hit.

mod common;

use std::time::Duration;

use clientapi_pve::apis::qemu_api;
use clientapi_pve::models::{PveBoolean, PveStatusEnum, QemuVmShutdownRequest};
use common::*;

const FIXTURE_VMID: i32 = 100;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_60_vm_lifecycle() {
    let creds = Credentials::from_env();
    skip_if_no_kvm!();

    let cfg = creds.config_with_token();
    let node = first_node(&cfg).await.expect("first_node");

    // 1. The fixture VM must exist before we touch it. If it doesn't,
    //    the proxmox-docker container was started with PVE_SEED_FIXTURE_VM=0
    //    — skip rather than fail.
    if qemu_api::qemu_vm_config(&cfg, &node, FIXTURE_VMID, None, None)
        .await
        .is_err()
    {
        eprintln!("SKIP: fixture VM {FIXTURE_VMID} is not present (seed disabled?)");
        return;
    }

    // 2. Establish starting state. Force-stop best-effort so the test is
    //    re-runnable after an aborted run.
    let _ = qemu_api::qemu_vm_stop(&cfg, &node, FIXTURE_VMID, None).await;
    wait_for_status(&cfg, &node, FIXTURE_VMID, PveStatusEnum::Stopped, 30)
        .await
        .expect("VM stopped at start");

    // 3. Start the VM.
    qemu_api::qemu_vm_start(&cfg, &node, FIXTURE_VMID, None)
        .await
        .expect("qm start 100");
    wait_for_status(&cfg, &node, FIXTURE_VMID, PveStatusEnum::Running, 30)
        .await
        .expect("VM running within 30 s");

    // 4. Shutdown — ACPI first with server-side force-stop fallback (PVE
    //    falls back to qmstop after `timeout` seconds if ACPI doesn't
    //    reach the guest). Doing the fallback server-side avoids the
    //    config-lock contention that arises when a client-side qmstop
    //    races a still-running qmshutdown.
    qemu_api::qemu_vm_shutdown(
        &cfg,
        &node,
        FIXTURE_VMID,
        Some(QemuVmShutdownRequest {
            force_stop: Some(PveBoolean::Variant1),
            timeout: Some(10),
            ..QemuVmShutdownRequest::new()
        }),
    )
    .await
    .expect("qm shutdown 100 with force_stop=1");

    wait_for_status(&cfg, &node, FIXTURE_VMID, PveStatusEnum::Stopped, 60)
        .await
        .expect("VM stopped within 60 s");
}

async fn wait_for_status(
    cfg: &clientapi_pve::apis::configuration::Configuration,
    node: &str,
    vmid: i32,
    expected: PveStatusEnum,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    wait_until(
        &format!("vm {vmid} → {expected:?}"),
        Duration::from_secs(timeout_secs),
        Duration::from_millis(500),
        || async {
            let resp = qemu_api::qemu_vm_status(cfg, node, vmid)
                .await
                .map_err(|e| anyhow::anyhow!("vm_status: {e}"))?;
            if resp.data.status == expected {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        },
    )
    .await
}
