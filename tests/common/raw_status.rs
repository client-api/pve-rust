use super::credentials::Credentials;

/// Raw GET of /nodes/{node}/{kind}/{vmid}/status/current that returns
/// just the `data.status` string. Bypasses the SDK's strict deserialiser:
/// PVE wires some `f64` fields (`cpu`, `pressure*`) as JSON strings
/// (`"0.00"`), which the generated `QemuVmStatusResponseData` /
/// `LxcVmStatusResponseData` reject. Filed upstream as a generator bug;
/// in the meantime the lifecycle tests only need the `status` field, so
/// this helper sidesteps the offending fields entirely.
pub async fn raw_status(
    creds: &Credentials,
    node: &str,
    kind: &str,
    vmid: i32,
) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let url = format!(
        "{}/api2/json/nodes/{node}/{kind}/{vmid}/status/current",
        creds.url.trim_end_matches('/')
    );
    let body: serde_json::Value = client
        .get(&url)
        .header("Authorization", &creds.token_header_value)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    body.get("data")
        .and_then(|d| d.get("status"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("status field missing in response"))
}
