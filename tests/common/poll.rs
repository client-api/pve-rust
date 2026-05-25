use std::future::Future;
use std::time::{Duration, Instant};

/// Poll `probe` until it returns `Ok(Some(_))`, or the timeout elapses.
///
/// Mirrors the TS reference `waitUntil()`: `Ok(None)` → keep polling,
/// `Ok(Some(v))` → succeed, `Err(_)` → abort early. The last error is
/// attached as the failure context when the deadline is hit.
pub async fn wait_until<F, Fut, T>(
    label: &str,
    timeout: Duration,
    interval: Duration,
    mut probe: F,
) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = anyhow::Result<Option<T>>>,
{
    let deadline = Instant::now() + timeout;
    let mut last_err: Option<anyhow::Error> = None;
    loop {
        match probe().await {
            Ok(Some(v)) => return Ok(v),
            Ok(None) => {}
            Err(e) => last_err = Some(e),
        }
        if Instant::now() >= deadline {
            return Err(match last_err {
                Some(e) => e.context(format!("wait_until({label}) timed out after {timeout:?}")),
                None => anyhow::anyhow!("wait_until({label}) timed out after {timeout:?}"),
            });
        }
        tokio::time::sleep(interval).await;
    }
}
