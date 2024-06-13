use core::fmt::Debug;
use minimal_matrix::notif_trait::Notifier;
use reqwest::Client;
use std::{future::Future, time::SystemTime};
use tokio::task;
pub struct SlackClient {
    pub client: Client,
    pub url: String,
}

impl SlackClient {
    pub fn new() -> Option<Self> {
        let var = std::env::var("SLACK_URL").ok();
        let r = var.map(|v| Self {
            client: Client::new(),
            url: v,
        });
        if r.is_none() {
            println!("The SLACK_URL variable was not set. Slack notification are disabled");
        }
        r
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
        if let Ok(r) = res {
            return r;
        }
        counter += 1;
        let error = res.err().unwrap();
        if counter % 10 == 0 {
            if let Some(c) = SlackClient::new() {
                c.send_message(format!("Failed task with {:#?}, retrying", error))
                    .await;
            }
            if let Some(mut c) = Mattermost::new() {
                c.send_message(format!("Failed task with {:#?}, retrying", error));
            }
        }

        println!("Failed task with {:#?}, retrying", error);
        task::yield_now().await;
    }
}

pub struct Mattermost {
    pub client: minimal_matrix::mattermost::MatterMost,
}

impl Mattermost {
    pub fn new() -> Option<Self> {
        let url = std::env::var("MATTERMOST_URL").ok();
        url.map(|val| Self {
            client: minimal_matrix::mattermost::MatterMost::new(&val),
        })
    }
    pub fn send_message(&mut self, message: String) {
        match self.client.send_message(message) {
            Ok(_) => (),
            Err(err) => eprintln!("Failed to send Matrix message: {}", err),
        }
    }
}
