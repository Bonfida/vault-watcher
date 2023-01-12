use core::fmt::Debug;
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
        if res.is_ok() {
            return res.unwrap();
        }
        counter += 1;
        let error = res.err().unwrap();
        if counter % 10 == 0 {
            if let Some(c) = SlackClient::new() {
                c.send_message(format!("Failed task with {:#?}, retrying", error))
                    .await;
            }
            if let Some(mut c) = MatrixClient::new().await {
                c.send_message(format!("Failed task with {:#?}, retrying", error));
            }
        }

        println!("Failed task with {:#?}, retrying", error);
        task::yield_now().await;
    }
}

pub struct MatrixClient {
    pub client: minimal_matrix::matrix::MatrixClient,
}

impl MatrixClient {
    pub async fn new() -> Option<Self> {
        let home_server_name = std::env::var("HOME_SERVER_NAME").ok();
        let room_id = std::env::var("ROOM_ID").ok();
        let access_token = std::env::var("MATRIX_TOKEN").ok();

        if let (Some(home_server_name), Some(room_id), Some(access_token)) =
            (home_server_name, room_id, access_token)
        {
            let client =
                minimal_matrix::matrix::MatrixClient::new(home_server_name, room_id, access_token)
                    .await
                    .unwrap();

            Some(Self { client })
        } else {
            None
        }
    }
    pub fn send_message(&mut self, message: String) {
        match self.client.send_message(message) {
            Ok(_) => (),
            Err(err) => eprintln!("Failed to send Matrix message: {}", err),
        }
    }
}
