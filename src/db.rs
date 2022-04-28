use std::time::Duration;

use sysinfo::SystemExt;
use tokio_postgres::{tls::MakeTlsConnect, types::Type, NoTls, Socket, Statement};

use crate::CachedAccount;

pub struct Database {
    client: tokio_postgres::Client,
    insertion_statement: Statement,
}

impl Database {
    pub const ENTRY_SIZE: u64 = 110; // Size in bytes of a single db entry
    pub const RELATIVE_CHUNK_SIZE: f64 = 0.10; // Size of a timescaledb chunk
    pub async fn new(
        refresh_period_ms: u64,
        number_of_accounts_to_monitor: u64,
    ) -> Result<Self, tokio_postgres::Error> {
        let (client, connection) = connect_to_database().await;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        initialize(&client, refresh_period_ms, number_of_accounts_to_monitor).await?;
        let insertion_statement = client
            .prepare("INSERT INTO vault_watcher VALUES ($1, $2, $3, $4);")
            .await
            .unwrap();
        Ok(Self {
            client,
            insertion_statement,
        })
    }

    pub async fn commit_account(&self, a: &CachedAccount) -> Result<(), tokio_postgres::Error> {
        let pubkey_str = a.address.to_string();
        self.client
            .execute(
                &self.insertion_statement,
                &[&chrono::Utc::now(), &pubkey_str, &a.name, &a.balance],
            )
            .await?;
        Ok(())
    }
}

async fn connect_to_database() -> (
    tokio_postgres::Client,
    tokio_postgres::Connection<Socket, <tokio_postgres::NoTls as MakeTlsConnect<Socket>>::Stream>,
) {
    let password = std::env::var("POSTGRES_PASSWORD")
        .expect("POSTGRES_PASSWORD environment variable must be set!");
    let config_str = format!("host=db port=5432 password={password} user=postgres dbname=postgres");
    loop {
        let res = tokio_postgres::connect(&config_str, NoTls).await;
        if let Ok(r) = res {
            return r;
        }
        println!("Failed to connect to database, retrying");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

async fn initialize(
    client: &tokio_postgres::Client,
    refresh_period_ms: u64,
    mut number_of_accounts_to_monitor: u64,
) -> Result<(), tokio_postgres::Error> {
    number_of_accounts_to_monitor = std::cmp::max(10, number_of_accounts_to_monitor);
    println!("=== Initializing database ===");
    client
        .execute(
            "CREATE TABLE IF NOT EXISTS vault_watcher (
        timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
        address VARCHAR(44),
        name VARCHAR(50),
        balance DOUBLE PRECISION,
        PRIMARY KEY (timestamp, name, address)
    );",
            &[],
        )
        .await
        .unwrap();
    // We convert the table to a hypertable
    let o = client
        .query(
            "SELECT create_hypertable('vault_watcher', 'timestamp', if_not_exists => TRUE);",
            &[],
        )
        .await
        .unwrap();
    println!("Output from create_hypertable");
    println!("{o:?}");

    // Implements the best practice detailed here
    // https://docs.timescale.com/timescaledb/latest/how-to-guides/hypertables/best-practices/#time-intervals
    let system_memory_kb = sysinfo::System::new_all().total_memory();
    let chunk_size_ms = refresh_period_ms * system_memory_kb * 1024
        / Database::ENTRY_SIZE
        / number_of_accounts_to_monitor;
    let shrunk_chunk_size = ((chunk_size_ms as f64) * Database::RELATIVE_CHUNK_SIZE) as i64;
    let s = client
        .prepare_typed(
            "SELECT set_chunk_time_interval('vault_watcher', $1);",
            &[Type::INT8],
        )
        .await
        .unwrap();
    let o = client.query(&s, &[&shrunk_chunk_size]).await?;
    println!("Output from set_chunk_time_interval");
    println!("{o:?}");
    Ok(())
}
