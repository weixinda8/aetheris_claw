use tokio::time::{self, Duration};

pub async fn retry_with_backoff<F, Fut, T, E>(
    mut f: F,
    max_retries: u32,
    base_delay: Duration,
) -> std::result::Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, E>>,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(value) => return Ok(value),
            Err(_e) if retries < max_retries => {
                retries += 1;
                let delay = base_delay * 2u32.pow(retries - 1);
                time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}
