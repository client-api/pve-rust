use clientapi_pve::apis::configuration::Configuration;
use clientapi_pve::apis::{
    access_users_api, nodes_api, qemu_api, storage_api,
};

pub const E2E_PREFIX: &str = "e2e-";

/// Test-private VM ID window. The fixture VM at 100 is reserved; we own
/// 101..=199. Each test that creates a VM stays inside this band so
/// cleanup_e2e can sweep without consulting state.
pub const E2E_VMID_MIN: i32 = 101;
pub const E2E_VMID_MAX: i32 = 199;

/// First node name from the fixture cluster. The proxmox-docker container
/// is single-node, so this is the only node any test will reference.
pub async fn first_node(cfg: &Configuration) -> anyhow::Result<String> {
    let resp = nodes_api::nodes_get_nodes(cfg)
        .await
        .map_err(|e| anyhow::anyhow!("nodes_get_nodes: {e}"))?;
    resp.data
        .into_iter()
        .next()
        .map(|n| n.node)
        .ok_or_else(|| anyhow::anyhow!("nodes list was empty"))
}

/// Best-effort sweep of any e2e-* leftovers. Idempotent, error-swallowing.
/// Each test calls this at setup so it can assume a clean slate even if a
/// prior run aborted mid-flight.
pub async fn cleanup_e2e(cfg: &Configuration) {
    cleanup_users(cfg).await;
    cleanup_storages(cfg).await;
    cleanup_vms(cfg).await;
}

async fn cleanup_users(cfg: &Configuration) {
    let Ok(resp) = access_users_api::access_users_get_users(cfg, None, None).await else {
        return;
    };
    for u in resp.data {
        if u.userid.starts_with(E2E_PREFIX) {
            let _ = access_users_api::access_users_delete_user(cfg, &u.userid).await;
        }
    }
}

async fn cleanup_storages(cfg: &Configuration) {
    let Ok(resp) = storage_api::storage_get_storage(cfg, None).await else {
        return;
    };
    for s in resp.data {
        if s.storage.starts_with(E2E_PREFIX) {
            let _ = storage_api::storage_delete_storage(cfg, &s.storage).await;
        }
    }
}

async fn cleanup_vms(cfg: &Configuration) {
    let Ok(node) = first_node(cfg).await else {
        return;
    };
    for vmid in E2E_VMID_MIN..=E2E_VMID_MAX {
        // Force-stop first (best-effort) so a left-running VM can still be destroyed.
        let _ = qemu_api::qemu_vm_stop(cfg, &node, vmid, None).await;
        let _ = qemu_api::qemu_destroy_vm(
            cfg,
            &node,
            vmid,
            Some("1"), // destroy_unreferenced_disks
            Some("1"), // purge
            None,
        )
        .await;
    }
}
