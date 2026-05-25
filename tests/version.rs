// SC-01 — /version returns expected shape.
//
// PVE allows /version anonymous in some 9.x versions and requires auth in
// others; the safest assertion is to call it with a valid token and
// validate the response shape. The container fixture always exposes a
// 9.x release, so we additionally assert release starts with "9".

mod common;

use clientapi_pve::apis::version_api;
use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_01_version_returns_expected_shape() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();

    let resp = version_api::version_version(&cfg)
        .await
        .expect("GET /version");

    assert!(
        !resp.data.version.is_empty(),
        "version field must be non-empty, got {:?}",
        resp.data.version
    );
    assert!(
        resp.data.release.starts_with('9'),
        "expected release 9.x, got {:?}",
        resp.data.release
    );
}
