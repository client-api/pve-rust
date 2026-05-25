// SC-40..42 — error envelope handling.

mod common;

use clientapi_pve::apis::configuration::ApiKey;
use clientapi_pve::apis::{access_users_api, qemu_api, Error};
use clientapi_pve::models::{
    AccessUsersCreateUserRequest, AccessUsersGenerateTokenRequest, PveBoolean, QemuCreateVmRequest,
};
use common::*;

const PRIVSEP_USER: &str = "e2e-privsep@pve";
const PRIVSEP_TOKEN_ID: &str = "scope";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_40_unknown_vmid_surfaces_response_error() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();
    let node = first_node(&cfg).await.expect("first_node");

    let err = qemu_api::qemu_vm_status(&cfg, &node, 9999)
        .await
        .expect_err("status on a non-existent vmid must fail");

    // PVE returns 500 (Perl die) rather than 404 — assert it's a typed
    // response error on the 4xx/5xx boundary, not a transport error.
    match err {
        Error::ResponseError(rc) => {
            assert!(
                rc.status.is_client_error() || rc.status.is_server_error(),
                "expected HTTP error, got {}",
                rc.status
            );
        }
        other => panic!("expected ResponseError, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_41_invalid_input_returns_400() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();
    let node = first_node(&cfg).await.expect("first_node");
    cleanup_e2e(&cfg).await;

    // cores: -1 is rejected by PVE schema validation.
    let mut req = QemuCreateVmRequest::new(150);
    req.cores = Some(-1);

    let err = qemu_api::qemu_create_vm(&cfg, &node, req)
        .await
        .expect_err("invalid input must fail");
    assert_response_status(&err, 400);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_42_privsep_token_without_acl_returns_403() {
    let creds = Credentials::from_env();
    skip_if_pmg!(creds);

    let admin = creds.config_with_token();
    cleanup_e2e(&admin).await;

    // Create a user (no ACL grants) and a *privsep* token for them.
    // privsep means the token's effective perms are the intersection of
    // user perms AND token-specific perms — and we give the token none.
    access_users_api::access_users_create_user(
        &admin,
        AccessUsersCreateUserRequest::new(PRIVSEP_USER.to_string()),
    )
    .await
    .expect("create user");

    let tok = access_users_api::access_users_generate_token(
        &admin,
        PRIVSEP_TOKEN_ID,
        PRIVSEP_USER,
        Some(AccessUsersGenerateTokenRequest {
            privsep: Some(PveBoolean::Variant1),
            ..Default::default()
        }),
    )
    .await
    .expect("generate privsep token");

    let mut privsep_cfg = creds.config_anonymous();
    privsep_cfg.api_key = Some(ApiKey {
        prefix: None,
        key: format!(
            "PVEAPIToken={PRIVSEP_USER}!{PRIVSEP_TOKEN_ID}={value}",
            value = tok.data.value
        ),
    });

    // Any administrative read should be rejected — the token has empty
    // effective privs even if the user underneath does not.
    let err = access_users_api::access_users_create_user(
        &privsep_cfg,
        AccessUsersCreateUserRequest::new("e2e-should-fail@pve".to_string()),
    )
    .await
    .expect_err("privsep token without ACL must fail");
    assert_response_status(&err, 403);

    cleanup_e2e(&admin).await;
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
