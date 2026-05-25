use std::path::PathBuf;

use sha2::{Digest, Sha256};

pub const BOOT_ISO_URL: &str =
    "https://github.com/client-api/256-byte-vm/releases/download/v1.0.0/boot.iso";
pub const BOOT_ISO_SHA256: &str =
    "356703056dc4c605084411ef8614d9520d1cc14bb6727d39456e3464dc84bb02";
pub const BOOT_ISO_FILENAME: &str = "boot.iso";

/// Fetch `boot.iso` from the `256-byte-vm` v1.0.0 release, verify its
/// SHA256 against the pinned hash, and cache it in `$TMPDIR`. Returns the
/// cached path. Fails loudly on hash mismatch — the test must not proceed.
pub async fn download_boot_iso() -> anyhow::Result<PathBuf> {
    // Cache filename MUST match BOOT_ISO_FILENAME — the SDK's multipart
    // upload derives the wire filename from PathBuf::file_name(), and the
    // test asserts on `local:iso/<BOOT_ISO_FILENAME>` after upload.
    let cache = std::env::temp_dir().join(BOOT_ISO_FILENAME);
    if cache.exists() {
        if let Ok(true) = verify_file_sha256(&cache, BOOT_ISO_SHA256).await {
            return Ok(cache);
        }
        let _ = tokio::fs::remove_file(&cache).await;
    }

    let client = reqwest::Client::builder().build()?;
    let bytes = client
        .get(BOOT_ISO_URL)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let actual = sha256_hex(&bytes);
    if actual != BOOT_ISO_SHA256 {
        anyhow::bail!(
            "boot.iso SHA256 mismatch: got {actual}, expected {BOOT_ISO_SHA256}"
        );
    }

    tokio::fs::write(&cache, &bytes).await?;
    Ok(cache)
}

pub async fn verify_file_sha256(path: &PathBuf, expected: &str) -> anyhow::Result<bool> {
    let bytes = tokio::fs::read(path).await?;
    Ok(sha256_hex(&bytes) == expected)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
