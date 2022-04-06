use std::{fs::File, path::Path};

use clap::{Arg, Command};
use vault_watcher::{run, AccountToMonitorRaw, Config};

#[tokio::main]
async fn main() {
    let command = Command::new("vault-watcher")
        .arg(Arg::new("accounts_json_path").required(true))
        .arg(Arg::new("config_path").required(true));
    let matches = command.get_matches();
    let config_path = Path::new(matches.value_of("config_path").unwrap());
    let accounts_json_path = Path::new(matches.value_of("accounts_json_path").unwrap());
    let accounts_to_monitor: Vec<AccountToMonitorRaw> = {
        let reader = File::open(accounts_json_path).unwrap();
        serde_json::from_reader(reader).unwrap()
    };
    let config: Config = {
        let reader = File::open(config_path).unwrap();
        serde_json::from_reader(reader).unwrap()
    };
    run(config, accounts_to_monitor).await
}
