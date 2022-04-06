use core::fmt::Debug;
use dotenv::var;
use reqwest::Client;
use std::{future::Future, time::SystemTime};
use tokio::task;

pub struct SlackClient {
    pub client: Client,
    pub url: String,
}

impl Default for SlackClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SlackClient {
    pub fn new() -> Self {
        dotenv::dotenv().unwrap();
        Self {
            client: Client::new(),
            url: var("SLACK_URL").unwrap(),
        }
    }
    pub async fn send_message(&self, message: String) {
        let slack_message = format!("{{ text: '{0}' }}", message);
        if self
            .client
            .post(&self.url)
            .body(slack_message.clone())
            .header("Content-Type", "application/json")
            .send()
            .await
            .is_err()
        {
            println!(
                "Failed to send message {:?} at {:?}",
                slack_message,
                SystemTime::now()
            )
        }
    }
}

pub async fn retry<F, T, K, E, R, Fut>(arg: T, f: F, e: R) -> K
where
    Fut: Future<Output = Result<K, E>>,
    F: Fn(&T) -> Fut,
    E: Debug,
    R: Fn(Result<K, E>) -> Result<K, E>,
{
    loop {
        let res = e(f(&arg).await);
        let mut counter = 1;
        if res.is_ok() {
            return res.unwrap();
        }
        counter += 1;
        let error = res.err().unwrap();
        if counter % 10 == 0 {
            SlackClient::new()
                .send_message(format!("Failed task with {:#?}, retrying", error))
                .await;
        }

        println!("Failed task with {:#?}, retrying", error);
        task::yield_now().await;
    }
}
