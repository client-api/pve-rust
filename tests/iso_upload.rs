// SC-35 — ISO upload, list, delete.

mod common;

use std::time::Duration;

use clientapi_pve::apis::nodes_storage_api;
use clientapi_pve::models::PveContentEnum;
use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_35_iso_upload_list_delete() {
    let creds = Credentials::from_env();
    skip_if_pmg!(creds);
    skip_if_no_network!();

    let cfg = creds.config_with_token();
    let node = first_node(&cfg).await.expect("first_node");

    // 1. Pre-cleanup so a re-run starts clean.
    let pre_existing = format!("local:iso/{BOOT_ISO_FILENAME}");
    let _ = nodes_storage_api::nodes_storage_delete_content(
        &cfg,
        &node,
        "local",
        &pre_existing,
        None,
    )
    .await;

    // 2. Download + SHA256-verify.
    let iso_path = download_boot_iso().await.expect("download boot.iso");

    // 3. Upload via the SDK's multipart helper.
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

    // 4. Poll until the uploaded volume appears (the upload returns a UPID
    //    task ID; the file lands asynchronously once the import finishes).
    wait_until(
        "boot.iso visible in local:iso",
        Duration::from_secs(60),
        Duration::from_millis(500),
        || async {
            let listed = nodes_storage_api::nodes_storage_get_content(
                &cfg,
                &node,
                "local",
                Some("iso"),
                None,
            )
            .await
            .map_err(|e| anyhow::anyhow!("list content: {e}"))?;
            Ok(listed
                .data
                .iter()
                .find(|c| c.volid == pre_existing)
                .map(|c| c.volid.clone()))
        },
    )
    .await
    .expect("iso appeared");

    // 5. Delete and confirm gone.
    nodes_storage_api::nodes_storage_delete_content(
        &cfg,
        &node,
        "local",
        &pre_existing,
        None,
    )
    .await
    .expect("delete iso");

    wait_until(
        "boot.iso removed from local:iso",
        Duration::from_secs(15),
        Duration::from_millis(500),
        || async {
            let listed = nodes_storage_api::nodes_storage_get_content(
                &cfg,
                &node,
                "local",
                Some("iso"),
                None,
            )
            .await
            .map_err(|e| anyhow::anyhow!("list content: {e}"))?;
            if listed.data.iter().any(|c| c.volid == pre_existing) {
                Ok(None)
            } else {
                Ok(Some(()))
            }
        },
    )
    .await
    .expect("iso gone");
}
