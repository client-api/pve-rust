use clientapi_pve::apis::configuration::{ApiKey, Configuration};
use reqwest::header::{HeaderMap, HeaderValue};

/// Credentials populated by `client-api/proxmox-docker-action@v1` into env.
///
/// `token_header_value` is pre-assembled by the container — it already has
/// the correct `=`/`:` separator for the product family. Tests must consume
/// it verbatim and never reassemble the header by hand.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub url: String,
    pub user: String,
    pub password: String,
    pub token_header_value: String,
    pub token_value: String,
}

impl Credentials {
    pub fn from_env() -> Self {
        Self {
            url: env_required("PROXMOX_URL"),
            user: env_required("PROXMOX_USER"),
            password: env_required("PROXMOX_PASSWORD"),
            token_header_value: env_required("PROXMOX_TOKEN_HEADER_VALUE"),
            token_value: env_required("PROXMOX_TOKEN_VALUE"),
        }
    }

    pub fn token_auth_supported(&self) -> bool {
        self.token_value != "(unsupported-by-pmg)"
    }

    fn base_client() -> reqwest::Client {
        reqwest::Client::builder()
            .danger_accept_invalid_certs(true) // self-signed CI cert
            .build()
            .expect("reqwest client")
    }

    /// Anonymous configuration — no auth headers. Used for SC-01 (/version
    /// on anonymous-allowed deployments) and for auth-failure scenarios.
    pub fn config_anonymous(&self) -> Configuration {
        let mut cfg = Configuration::new();
        cfg.base_path = self.url.trim_end_matches('/').to_string() + "/api2/json";
        cfg.client = Self::base_client();
        cfg.api_key = None;
        cfg
    }

    /// Token-authenticated configuration. The SDK puts api_key.key verbatim
    /// into both `Authorization` and `CSRFPreventionToken` headers; PVE
    /// ignores CSRF when token auth is presented, so this is harmless.
    pub fn config_with_token(&self) -> Configuration {
        let mut cfg = self.config_anonymous();
        cfg.api_key = Some(ApiKey {
            prefix: None,
            key: self.token_header_value.clone(),
        });
        cfg
    }

    /// Ticket-authenticated configuration. The SDK has no first-class
    /// support for cookie + CSRF separately, so we inject them as default
    /// headers on the underlying reqwest client and leave `api_key` unset.
    pub fn config_with_ticket(&self, ticket: &str, csrf: &str) -> Configuration {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::COOKIE,
            HeaderValue::from_str(&format!("PVEAuthCookie={ticket}"))
                .expect("ticket header value"),
        );
        headers.insert(
            "CSRFPreventionToken",
            HeaderValue::from_str(csrf).expect("csrf header value"),
        );
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .build()
            .expect("reqwest client");
        let mut cfg = self.config_anonymous();
        cfg.client = client;
        cfg
    }

    /// Like `config_with_ticket` but omits the CSRF header — used by SC-14
    /// to assert that state-changing requests reject ticket auth without CSRF.
    pub fn config_with_ticket_no_csrf(&self, ticket: &str) -> Configuration {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::COOKIE,
            HeaderValue::from_str(&format!("PVEAuthCookie={ticket}"))
                .expect("ticket header value"),
        );
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .build()
            .expect("reqwest client");
        let mut cfg = self.config_anonymous();
        cfg.client = client;
        cfg
    }
}

fn env_required(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("env var {key} must be set"))
}
