// SC-50..52 — type edge cases.
//
// Rust value: serde's strict deserialisation catches generator bugs that
// TypeScript silently absorbs. If int64 truncates or a oneOf misroutes,
// these tests fail at the type system level, not via fuzzy assertions.

mod common;

use clientapi_pve::apis::nodes_api;
use clientapi_pve::models::{
    pve_storage_dir_config, pve_storage_nfs_config, PveStorageDirConfig, PveStorageNfsConfig,
    StorageCreateStorageRequest,
};
use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_50_bigint_fields_deserialize_as_i64() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();

    let resp = nodes_api::nodes_get_nodes(&cfg)
        .await
        .expect("nodes list");
    let node = resp.data.first().expect("at least one node");

    // The fact this compiles is the contract: maxmem must be Option<i64>.
    // If the generator regressed it to f64 (which would silently truncate
    // anything > 2^53), this assertion's type wouldn't compile.
    let _: Option<i64> = node.maxmem;
    let _: Option<i64> = node.uptime;

    // Runtime validation: container reports real numbers, not literal 0.
    if let Some(maxmem) = node.maxmem {
        assert!(maxmem > 0, "maxmem must be a positive int64");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_51_nullable_fields_decode_as_option_none() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();

    let resp = nodes_api::nodes_get_nodes(&cfg).await.expect("nodes list");
    // The fixture's single node may or may not have ssl_fingerprint
    // populated; either Some or None is acceptable, but a present-but-null
    // wire value must decode to None (not "null" the string).
    for n in &resp.data {
        let _: Option<String> = n.ssl_fingerprint.clone();
        if let Some(fp) = &n.ssl_fingerprint {
            assert!(!fp.is_empty(), "ssl_fingerprint must be non-empty if present");
            assert_ne!(fp, "null", "string \"null\" leaking through Option layer");
        }
    }
}

#[test]
fn sc_52_oneof_discriminator_round_trip() {
    // Pure type-level check: the SDK's oneOf encoding emits the right
    // `type` discriminator for each variant. No API call needed — this
    // is the SDK's contract, not the server's.
    let dir = StorageCreateStorageRequest::Dir(Box::new(PveStorageDirConfig::new(
        "e2e-dir-probe".to_string(),
        "/tmp/e2e-probe".to_string(),
        pve_storage_dir_config::Type::Dir,
    )));
    let dir_json = serde_json::to_value(&dir).expect("serialize dir");
    assert_eq!(
        dir_json.get("type").and_then(|v| v.as_str()),
        Some("dir"),
        "Dir variant must carry type=dir discriminator"
    );

    let nfs = StorageCreateStorageRequest::Nfs(Box::new(PveStorageNfsConfig::new(
        "e2e-nfs-probe".to_string(),
        "/mnt/probe".to_string(),
        "203.0.113.1".to_string(), // RFC 5737 documentation address
        "/exports".to_string(),
        pve_storage_nfs_config::Type::Nfs,
    )));
    let nfs_json = serde_json::to_value(&nfs).expect("serialize nfs");
    assert_eq!(
        nfs_json.get("type").and_then(|v| v.as_str()),
        Some("nfs"),
        "Nfs variant must carry type=nfs discriminator"
    );
    assert_ne!(
        dir_json.get("type"),
        nfs_json.get("type"),
        "variants must encode distinct discriminator values"
    );
}
