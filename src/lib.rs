use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use std::future::Future;
use std::sync::Arc;
use anyhow::Result;

pub struct RateLimiter {
    _runtime: Runtime,
    tokens: Arc<Mutex<i32>>,
    wait_time_millis: u64,
}

impl RateLimiter {
    /// max_tokens: トークンの最大数
    /// tokens_to_add: トークンを一度に追加する数
    /// add_token_per_millis: トークンを追加する間隔
    /// wait_time_millis: トークンがない場合の待機時間
    pub fn new(max_tokens: i32, tokens_to_add: i32, add_token_per_millis: u64, wait_time_millis: u64) -> Result<Self> {
        let arc_tokens = Arc::new(Mutex::new(max_tokens));
        let runtime = Runtime::new()?;
        runtime.spawn(handle_token(arc_tokens.clone(), tokens_to_add, max_tokens, add_token_per_millis));
        Ok(Self {
            _runtime: runtime,
            tokens: arc_tokens,
            wait_time_millis,
        })
    }

    async fn consume_token(&self) {
        loop {
            {
                let mut tokens = self.tokens.lock().await;
                if *tokens > 0 {
                    *tokens -= 1;
                    return;
                }
            } // このブロックの終わりでロックは自動的に解放されます。
            // トークンがない場合は待機
            // あまり早いと、デッドロックの可能性があるので、適度に待機
            sleep(Duration::from_millis(self.wait_time_millis)).await;
        }
    }

    pub async fn request_with_limiting<F, Fut, R>(&self, make_request: F) -> R
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = R> + Send,
        R: Send + 'static,
    {
        self.consume_token().await;
        make_request().await
    }

}

async fn handle_token(tokens: Arc<Mutex<i32>>, tokens_to_add: i32, max_tokens: i32, add_token_per_millis: u64) {
    loop {
        sleep(Duration::from_millis(add_token_per_millis)).await;
        add_token(tokens.clone(), max_tokens, tokens_to_add).await;
    }
}

async fn add_token(tokens: Arc<Mutex<i32>>, max_tokens: i32, tokens_to_add: i32) {
    let mut tokens = tokens.lock().await;
    if max_tokens > *tokens + tokens_to_add {
        *tokens += tokens_to_add;
    } else {
        *tokens = max_tokens;
    }
}