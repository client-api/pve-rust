// SC-62 — VM boot-from-CDROM lifecycle. KVM + network gated. Depends on
// SC-35 having validated the ISO upload pathway.

mod common;

use std::collections::HashMap;
use std::time::Duration;

use clientapi_pve::apis::{nodes_storage_api, qemu_api};
use clientapi_pve::models::{
    PveBiosEnum, PveBoolean, PveContentEnum, PveIdeField, PveMemoryField,
    QemuCreateVmRequest, QemuVmShutdownRequest,
};
use common::*;

const CDROM_VMID: i32 = 101;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_62_vm_cdrom_boot_lifecycle() {
    let creds = Credentials::from_env();
    skip_if_no_kvm!();
    skip_if_no_network!();

    let cfg = creds.config_with_token();
    let node = first_node(&cfg).await.expect("first_node");

    // Cleanup any leftover.
    let _ = qemu_api::qemu_vm_stop(&cfg, &node, CDROM_VMID, None).await;
    let _ = qemu_api::qemu_destroy_vm(&cfg, &node, CDROM_VMID, Some("1"), Some("1"), None).await;
    let volid = format!("local:iso/{BOOT_ISO_FILENAME}");
    let _ = nodes_storage_api::nodes_storage_delete_content(&cfg, &node, "local", &volid, None)
        .await;

    // 1. Upload boot.iso (SC-35 path — exercising the SDK helper, not the test).
    let iso_path = download_boot_iso().await.expect("download boot.iso");
    nodes_storage_api::nodes_storage_upload(
        &cfg,
        &node,
        "local",
        PveContentEnum::Iso,
        iso_path,
        None,
        None,
    )
    .await
    .expect("upload boot.iso");

    wait_until(
        "boot.iso visible",
        Duration::from_secs(60),
        Duration::from_millis(500),
        || async {
            let listed = nodes_storage_api::nodes_storage_get_content(
                &cfg, &node, "local", Some("iso"), None,
            )
            .await
            .map_err(|e| anyhow::anyhow!("list content: {e}"))?;
            Ok(listed.data.iter().find(|c| c.volid == volid).map(|_| ()))
        },
    )
    .await
    .expect("iso visible");

    // 2. Create VM 101 with ide2 = CDROM boot.
    let mut ides = HashMap::new();
    ides.insert(
        2,
        PveIdeField::String(format!("{volid},media=cdrom")),
    );

    let mut req = QemuCreateVmRequest::new(CDROM_VMID);
    req.name = Some("cdrom-test".to_string());
    req.bios = Some(PveBiosEnum::Seabios);
    req.memory = Some(Box::new(PveMemoryField::String("64".to_string())));
    req.cores = Some(1);
    req.boot = Some("order=ide2".to_string());
    req.ides = Some(ides);

    qemu_api::qemu_create_vm(&cfg, &node, req)
        .await
        .expect("create cdrom VM");

    // 3. Start, wait running, then shutdown with force-stop fallback.
    qemu_api::qemu_vm_start(&cfg, &node, CDROM_VMID, None)
        .await
        .expect("start cdrom VM");
    wait_for_status(&creds, &node, CDROM_VMID, "running", 30)
        .await
        .expect("cdrom VM running");

    // Use server-side force-stop fallback (see vm_lifecycle.rs for the
    // race rationale).
    qemu_api::qemu_vm_shutdown(
        &cfg,
        &node,
        CDROM_VMID,
        Some(QemuVmShutdownRequest {
            force_stop: Some(PveBoolean::Variant1),
            timeout: Some(10),
            ..QemuVmShutdownRequest::new()
        }),
    )
    .await
    .expect("shutdown cdrom VM with force_stop=1");
    wait_for_status(&creds, &node, CDROM_VMID, "stopped", 60)
        .await
        .expect("cdrom VM stopped within 60 s");

    // 4. Destroy + iso cleanup.
    qemu_api::qemu_destroy_vm(&cfg, &node, CDROM_VMID, Some("1"), Some("1"), None)
        .await
        .expect("destroy cdrom VM");
    let _ = nodes_storage_api::nodes_storage_delete_content(&cfg, &node, "local", &volid, None)
        .await;
}

async fn wait_for_status(
    creds: &Credentials,
    node: &str,
    vmid: i32,
    expected: &str,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    // Raw GET — see common/raw_status.rs for the SDK f64-string mismatch.
    wait_until(
        &format!("vm {vmid} → {expected}"),
        Duration::from_secs(timeout_secs),
        Duration::from_millis(500),
        || async {
            let status = raw_status(creds, node, "qemu", vmid)
                .await
                .map_err(|e| anyhow::anyhow!("vm_status: {e}"))?;
            if status == expected {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        },
    )
    .await
}
