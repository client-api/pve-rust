// SC-20..22 — authorization.

mod common;

use clientapi_pve::apis::configuration::ApiKey;
use clientapi_pve::apis::{access_acl_api, access_api, access_users_api, Error};
use clientapi_pve::models::{
    AccessAclUpdateAclRequest, AccessUsersCreateUserRequest, AccessUsersGenerateTokenRequest,
};
use common::*;

const READONLY_USER: &str = "e2e-readonly@pve";
const READONLY_TOKEN_ID: &str = "probe";
const ADMIN_PROBE_USER: &str = "e2e-admin-probe@pve";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_20_readonly_user_cannot_create() {
    let creds = Credentials::from_env();
    skip_if_pmg!(creds);

    let admin = creds.config_with_token();
    cleanup_e2e(&admin).await;

    // 1. Create the read-only user.
    access_users_api::access_users_create_user(
        &admin,
        AccessUsersCreateUserRequest::new(READONLY_USER.to_string()),
    )
    .await
    .expect("create read-only user");

    // 2. Grant PVEAuditor (read-only) on /.
    access_acl_api::access_acl_update_acl(
        &admin,
        AccessAclUpdateAclRequest {
            path: "/".to_string(),
            roles: "PVEAuditor".to_string(),
            users: Some(READONLY_USER.to_string()),
            propagate: Some(clientapi_pve::models::PveBoolean::Variant1),
            ..Default::default()
        },
    )
    .await
    .expect("grant PVEAuditor");

    // 3. Generate a token (no privsep — token inherits user's perms).
    let tok = access_users_api::access_users_generate_token(
        &admin,
        READONLY_TOKEN_ID,
        READONLY_USER,
        Some(AccessUsersGenerateTokenRequest::default()),
    )
    .await
    .expect("generate read-only token");
    let token_value = tok.data.value;
    let token_header =
        format!("PVEAPIToken={READONLY_USER}!{READONLY_TOKEN_ID}={token_value}");

    // 4. Use the read-only token to attempt creating a new user → 403.
    let mut readonly_cfg = creds.config_anonymous();
    readonly_cfg.api_key = Some(ApiKey {
        prefix: None,
        key: token_header,
    });
    let err = access_users_api::access_users_create_user(
        &readonly_cfg,
        AccessUsersCreateUserRequest::new("e2e-blocked@pve".to_string()),
    )
    .await
    .expect_err("read-only must not be able to create users");
    assert_response_status(&err, 403);

    cleanup_e2e(&admin).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_21_admin_can_create() {
    let creds = Credentials::from_env();
    skip_if_pmg!(creds);

    let admin = creds.config_with_token();
    let _ = access_users_api::access_users_delete_user(&admin, ADMIN_PROBE_USER).await;

    access_users_api::access_users_create_user(
        &admin,
        AccessUsersCreateUserRequest::new(ADMIN_PROBE_USER.to_string()),
    )
    .await
    .expect("admin user creation");

    let _ = access_users_api::access_users_delete_user(&admin, ADMIN_PROBE_USER).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_22_permissions_returns_effective_acls() {
    let creds = Credentials::from_env();
    let admin = creds.config_with_token();

    let resp = access_api::access_permissions(&admin, None, Some(&creds.user))
        .await
        .expect("GET /access/permissions");

    // The endpoint returns { path: { perm: 1, … }, … } — a map. Assert at
    // least one path is present for an admin user.
    let obj = resp
        .data
        .as_object()
        .expect("permissions data must be an object");
    assert!(!obj.is_empty(), "admin should have at least one ACL path");
}

fn assert_response_status<T: std::fmt::Debug>(err: &Error<T>, expected: u16) {
    match err {
        Error::ResponseError(rc) => assert_eq!(
            rc.status.as_u16(),
            expected,
            "expected HTTP {expected}, got {} (body: {})",
            rc.status,
            rc.content
        ),
        other => panic!("expected ResponseError({expected}), got {other:?}"),
    }
}
