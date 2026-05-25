// SC-30..34 — CRUD baseline.

mod common;

use clientapi_pve::apis::{
    access_acl_api, access_users_api, storage_api,
};
use clientapi_pve::models::{
    pve_storage_dir_config, AccessAclUpdateAclRequest, AccessUsersCreateUserRequest, PveBoolean,
    PveStorageDirConfig, StorageCreateStorageRequest,
};
use common::*;

const E2E_USER: &str = "e2e-user-01@pve";
const E2E_STORAGE: &str = "e2e-store-01";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_30_list_users_includes_root() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();

    let users = access_users_api::access_users_get_users(&cfg, None, None)
        .await
        .expect("list users");
    let has_root = users.data.iter().any(|u| u.userid == "root@pam");
    assert!(
        has_root,
        "expected root@pam in users list; got {:?}",
        users.data.iter().map(|u| &u.userid).collect::<Vec<_>>()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_31_user_crud_roundtrip() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();
    let _ = access_users_api::access_users_delete_user(&cfg, E2E_USER).await;

    access_users_api::access_users_create_user(
        &cfg,
        AccessUsersCreateUserRequest::new(E2E_USER.to_string()),
    )
    .await
    .expect("create user");

    let listed = access_users_api::access_users_get_users(&cfg, None, None)
        .await
        .expect("list users");
    assert!(
        listed.data.iter().any(|u| u.userid == E2E_USER),
        "newly-created user must appear in listing"
    );

    access_users_api::access_users_delete_user(&cfg, E2E_USER)
        .await
        .expect("delete user");

    let listed = access_users_api::access_users_get_users(&cfg, None, None)
        .await
        .expect("list users after delete");
    assert!(
        !listed.data.iter().any(|u| u.userid == E2E_USER),
        "deleted user must be gone"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_32_storage_crud_roundtrip() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();
    let _ = storage_api::storage_delete_storage(&cfg, E2E_STORAGE).await;

    let req = StorageCreateStorageRequest::Dir(Box::new(PveStorageDirConfig {
        storage: E2E_STORAGE.to_string(),
        path: format!("/tmp/{E2E_STORAGE}"),
        content: Some("iso,vztmpl".to_string()),
        mkdir: Some(PveBoolean::Variant1),
        r#type: pve_storage_dir_config::Type::Dir,
        ..PveStorageDirConfig::new(
            E2E_STORAGE.to_string(),
            format!("/tmp/{E2E_STORAGE}"),
            pve_storage_dir_config::Type::Dir,
        )
    }));
    storage_api::storage_create_storage(&cfg, req)
        .await
        .expect("create storage");

    let listed = storage_api::storage_get_storage(&cfg, None)
        .await
        .expect("list storages");
    assert!(listed.data.iter().any(|s| s.storage == E2E_STORAGE));

    storage_api::storage_delete_storage(&cfg, E2E_STORAGE)
        .await
        .expect("delete storage");

    let listed = storage_api::storage_get_storage(&cfg, None)
        .await
        .expect("list storages after delete");
    assert!(!listed.data.iter().any(|s| s.storage == E2E_STORAGE));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_33_acl_crud_roundtrip() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();

    cleanup_e2e(&cfg).await;
    access_users_api::access_users_create_user(
        &cfg,
        AccessUsersCreateUserRequest::new(E2E_USER.to_string()),
    )
    .await
    .expect("create user for ACL test");

    // Grant.
    access_acl_api::access_acl_update_acl(
        &cfg,
        AccessAclUpdateAclRequest {
            path: "/".to_string(),
            roles: "PVEAuditor".to_string(),
            users: Some(E2E_USER.to_string()),
            propagate: Some(PveBoolean::Variant1),
            ..Default::default()
        },
    )
    .await
    .expect("grant ACL");

    let acl = access_acl_api::access_acl_read_acl(&cfg)
        .await
        .expect("read ACL");
    let granted = acl
        .data
        .iter()
        .any(|e| matches!(&e.ugid, ugid if ugid == E2E_USER) && e.roleid == "PVEAuditor");
    assert!(granted, "ACL entry for {E2E_USER}/PVEAuditor must be present");

    // Revoke (PVE uses the same endpoint with delete: 1).
    access_acl_api::access_acl_update_acl(
        &cfg,
        AccessAclUpdateAclRequest {
            path: "/".to_string(),
            roles: "PVEAuditor".to_string(),
            users: Some(E2E_USER.to_string()),
            delete: Some(PveBoolean::Variant1),
            ..Default::default()
        },
    )
    .await
    .expect("revoke ACL");

    let acl = access_acl_api::access_acl_read_acl(&cfg)
        .await
        .expect("read ACL after revoke");
    let still_granted = acl
        .data
        .iter()
        .any(|e| e.ugid == E2E_USER && e.roleid == "PVEAuditor");
    assert!(!still_granted, "ACL entry must be gone after revoke");

    let _ = access_users_api::access_users_delete_user(&cfg, E2E_USER).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_34_pagination_walks_users_endpoint() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();

    // PVE doesn't paginate /access/users — the endpoint returns the full
    // list in one response. The SDK surface still has to handle that
    // contract; we exercise it to catch silent shape regressions.
    let listed = access_users_api::access_users_get_users(&cfg, Some(PveBoolean::Variant1), None)
        .await
        .expect("list enabled users");
    assert!(
        !listed.data.is_empty(),
        "at least the admin should be enabled"
    );
    for u in &listed.data {
        assert!(!u.userid.is_empty(), "every entry must have a userid");
    }
}
