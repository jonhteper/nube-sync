use std::{future::Future, sync::LazyLock, time::Duration};

pub static DEFAULT_CONN_RETRY: LazyLock<ConnRetry> = LazyLock::new(ConnRetry::default);

#[derive(Clone, Copy)]
pub struct ConnRetry {
    tries: usize,
    delay: Duration,
}

impl ConnRetry {
    pub async fn execute_with_retries<F, Fut, T, E>(&self, f: F) -> Result<T, E>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::error::Error,
    {
        let mut tries = self.tries;
        loop {
            match f().await {
                Ok(res) => return Ok(res),
                Err(err) => {
                    tries -= 1;
                    if tries == 0 {
                        return Err(err);
                    }
                    tokio::time::sleep(self.delay).await;
                }
            }
        }
    }
}

impl Default for ConnRetry {
    fn default() -> Self {
        Self {
            tries: 3,
            delay: Duration::from_millis(250),
        }
    }
}
