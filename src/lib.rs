use std::{
    collections::{hash_map::RandomState, HashMap},
    str::FromStr,
    time::Duration,
};

use db::Database;
use itertools::{izip, multizip};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    account::Account, account_utils::StateMut, bpf_loader_upgradeable::UpgradeableLoaderState,
};
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};
use utils::{Mattermost, SlackClient};

mod db;
mod utils;

// Hardcode keys as a workaround in order to match on them
pub const SYSTEM_PGR_ID: &str = "11111111111111111111111111111111";
pub const TOKEN_PGR_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const BPF_UPLOADER_PGR_ID: &str = "BPFLoaderUpgradeab1e11111111111111111111111";

pub const DEFAULT_CHANGE: f64 = 100.0;
pub const DEFAULT_CHANGE_PERIOD: u64 = 3_600_000;

////////////////////////////////////////
/// User Inputs

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    endpoint: String,
    refresh_period: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputAccountType {
    Vault,
    Program,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputAccountRaw {
    pub account_type: InputAccountType,
    pub address: String,
    #[serde(flatten)]
    pub max_change: Option<MaxChange>,
    pub name: String,
    pub min_amount_threshold: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MaxChange {
    max_change: f64,
    max_change_period: u64,
}

impl InputAccountRaw {
    pub fn parse((key, r, acc): (&Pubkey, InputAccountRaw, &Option<Account>)) -> CachedAccount {
        match acc
            .as_ref()
            .unwrap_or_else(|| panic!("Account {} could not be found", key))
            .owner
            .to_string()
            .as_str()
        {
            SYSTEM_PGR_ID => CachedAccount {
                address: *key,
                name: r.name,
                info: CachedAccountInfos::NativeSol(VaultAccountInfo {
                    max_change: r.max_change,
                    balance: 0.,
                    decimals: 9,
                    min_amount_threshold: r.min_amount_threshold,
                }),
            },
            TOKEN_PGR_ID => CachedAccount {
                address: *key,
                name: r.name,
                info: CachedAccountInfos::Token(VaultAccountInfo {
                    max_change: r.max_change,
                    balance: 0.,
                    decimals: 0,
                    min_amount_threshold: r.min_amount_threshold,
                }),
            },
            BPF_UPLOADER_PGR_ID => CachedAccount {
                address: *key,
                name: r.name,
                info: CachedAccountInfos::Program(ProgramAccountInfo {
                    last_deploy_slot: 0,
                    upgrade_auth: None,
                }),
            },
            _ => {
                println!("Found wrong owner for {}", key);
                panic!()
            }
        }
    }
}

////////////////////////////////////////
/// Cached Account types
///

pub struct CachedAccount {
    pub address: Pubkey,
    pub name: String,
    pub info: CachedAccountInfos,
}

#[derive(Debug)]
pub enum CachedAccountInfos {
    NativeSol(VaultAccountInfo),
    Token(VaultAccountInfo),
    Program(ProgramAccountInfo),
}

#[derive(Debug)]
pub struct VaultAccountInfo {
    balance: f64,
    decimals: i32,
    max_change: Option<MaxChange>,
    min_amount_threshold: Option<f64>,
}

#[derive(Debug)]
pub struct ProgramAccountInfo {
    last_deploy_slot: u64,
    upgrade_auth: Option<Pubkey>,
}

////////////////////////////////////////
/// Monitoring functions

pub async fn run(config: Config, accounts: Vec<InputAccountRaw>) {
    let Config {
        endpoint,
        refresh_period,
    } = config;
    // We try to initialize a new slack client in order to test it
    SlackClient::new();
    let connection = RpcClient::new(endpoint);
    let database = Database::new(refresh_period, accounts.len() as u64)
        .await
        .unwrap();
    let cache = initialize(&connection, accounts, refresh_period).await;
    monitor(refresh_period, &connection, cache, &database).await
}

pub async fn initialize(
    connection: &RpcClient,
    input_accounts: Vec<InputAccountRaw>,
    refresh_period: u64,
) -> Vec<CachedAccount> {
    let keys = &input_accounts
        .iter()
        .map(|a| Pubkey::from_str(&a.address).unwrap())
        .collect::<Vec<_>>();
    let accounts = connection.get_multiple_accounts(keys).await.unwrap();
    let mut parsed_accounts = multizip((keys, input_accounts, &accounts))
        .map(InputAccountRaw::parse)
        .collect::<Vec<_>>();

    // Fetch token mint decimals
    let parsed_token_accounts = parsed_accounts
        .iter()
        .zip(&accounts)
        .map(|(cached, acc)| {
            if let CachedAccountInfos::Token(_) = cached.info {
                Some(
                    spl_token::state::Account::unpack(&acc.as_ref().unwrap().data.clone()).unwrap(),
                )
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let mints = parsed_token_accounts
        .iter()
        .filter_map(|a| a.as_ref().map(|acc| acc.mint))
        .collect::<Vec<_>>();
    let mint_decimals = HashMap::<_, _, RandomState>::from_iter(
        connection
            .get_multiple_accounts(&mints)
            .await
            .unwrap()
            .into_iter()
            .zip(mints.into_iter())
            .map(|(a, k)| {
                (
                    k,
                    spl_token::state::Mint::unpack(&a.unwrap().data)
                        .unwrap()
                        .decimals,
                )
            }),
    );

    // Fetch the program data accounts, which are the ones to be cached
    let program_data_keys = parsed_accounts
        .iter()
        .zip(&accounts)
        .flat_map(|(cached, acc)| {
            if let CachedAccountInfos::Program(_) = cached.info {
                if let UpgradeableLoaderState::Program {
                    programdata_address,
                } = acc.as_ref().unwrap().state().unwrap()
                {
                    Some(programdata_address)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let program_data = HashMap::<_, _, RandomState>::from_iter(
        connection
            .get_multiple_accounts(&program_data_keys)
            .await
            .unwrap()
            .into_iter()
            .zip(program_data_keys.into_iter())
            .filter_map(|(a, k)| {
                if let UpgradeableLoaderState::ProgramData {
                    slot,
                    upgrade_authority_address,
                } = a.as_ref().unwrap().state().unwrap()
                {
                    Some((k, (slot, upgrade_authority_address)))
                } else {
                    None
                }
            }),
    );

    // Update the initial cache with the correct values
    for (cached, account, token_account) in
        izip!(&mut parsed_accounts, accounts, parsed_token_accounts)
    {
        let amount = match cached.info {
            CachedAccountInfos::NativeSol(_) => account.as_ref().unwrap().lamports,
            CachedAccountInfos::Token(ref mut v) => {
                v.decimals = *mint_decimals.get(&token_account.unwrap().mint).unwrap() as i32;
                token_account.unwrap().amount
            }
            _ => 0,
        };
        match cached.info {
            CachedAccountInfos::NativeSol(ref mut v) | CachedAccountInfos::Token(ref mut v) => {
                v.balance = (amount as f64) / 10.0f64.powi(v.decimals);
                // Amount of change in one refresh
                if let Some(v) = v.max_change.as_mut() {
                    v.max_change =
                        v.max_change * (refresh_period as f64) / (v.max_change_period as f64)
                }
            }

            CachedAccountInfos::Program(ref mut c) => {
                if let UpgradeableLoaderState::Program {
                    programdata_address,
                } = account.as_ref().unwrap().state().unwrap()
                {
                    let (slot, upgrade_authority_address) =
                        program_data.get(&programdata_address).unwrap();
                    cached.address = programdata_address;
                    c.last_deploy_slot = *slot;
                    c.upgrade_auth = *upgrade_authority_address;
                }
            }
        }
    }

    parsed_accounts
}

pub async fn monitor(
    interval: u64,
    connection: &RpcClient,
    mut cache: Vec<CachedAccount>,
    database: &Database,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(interval));
    let accounts_to_monitor = cache.iter().map(|c| c.address).collect::<Vec<_>>();
    loop {
        interval.tick().await;
        let accounts = utils::retry(
            &accounts_to_monitor,
            |c| connection.get_multiple_accounts(c),
            |e| e,
        )
        .await;
        for (i, a) in accounts.into_iter().enumerate() {
            let cached = &mut cache[i];

            // Check for changes if account is vault
            let amount = match &cached.info {
                CachedAccountInfos::NativeSol(_) => a.as_ref().unwrap().lamports,
                CachedAccountInfos::Token(_) => {
                    spl_token::state::Account::unpack(&a.as_ref().unwrap().data)
                        .unwrap()
                        .amount
                }
                _ => 0,
            };
            if let CachedAccountInfos::NativeSol(ref mut v) | CachedAccountInfos::Token(ref mut v) =
                cached.info
            {
                let new_balance = (amount as f64) / 10.0f64.powi(v.decimals);
                let delta = (new_balance - v.balance).abs();
                if v.max_change
                    .as_ref()
                    .map(|m| delta > m.max_change)
                    .unwrap_or(false)
                {
                    if let Some(c) = SlackClient::new() {
                        c.send_message(format!(
                            "Vault account spike detected for {} ({}) of {} - previous balance {} - current balance {}",
                            cached.name, cached.address, delta, v.balance, new_balance
                        ))
                        .await;
                    }
                    if let Some(mut c) = Mattermost::new() {
                        c.send_message(format!(
                            "Vault account spike detected for {} ({}) of {} - previous balance {} - current balance {}",
                            cached.name, cached.address, delta, v.balance, new_balance
                        ));
                    }
                }
                if v.min_amount_threshold
                    .map(|min_amount| new_balance < min_amount && v.balance > min_amount)
                    .unwrap_or(false)
                {
                    if let Some(c) = SlackClient::new() {
                        c.send_message(format!(
                            "Vault account low detected for {} ({}) with delta {} - previous balance {} - current balance {}",
                            cached.name, cached.address, delta, v.balance, new_balance
                        ))
                        .await;
                    }
                    if let Some(mut c) = Mattermost::new() {
                        c.send_message(format!(
                            "Vault account low detected for {} ({}) with delta {} - previous balance {} - current balance {}",
                            cached.name, cached.address, delta, v.balance, new_balance
                        ));
                    }
                }
                v.balance = new_balance;
            }

            // Check for changes if account is program
            let mut change_in_pgr = false;
            if let CachedAccountInfos::Program(ref mut p) = cached.info {
                if let UpgradeableLoaderState::ProgramData {
                    slot,
                    upgrade_authority_address,
                } = a.as_ref().unwrap().state().unwrap()
                {
                    if slot > p.last_deploy_slot {
                        if let Some(c) = SlackClient::new() {
                            c.send_message(format!(
                                "Program account deployment detected for {} (program data account: {}) | Old last_deploy slot {}, new last_deploy slot {}",
                                cached.name, cached.address, p.last_deploy_slot, slot
                            ))
                            .await;
                        }
                        if let Some(mut c) = Mattermost::new() {
                            c.send_message(format!(
                                "Program account deployment detected for {} (program data account: {}) | Old last_deploy slot {}, new last_deploy slot {}",
                                cached.name, cached.address, p.last_deploy_slot, slot
                            ));
                        }
                        p.last_deploy_slot = slot;
                        change_in_pgr = true;
                    }
                    if upgrade_authority_address != p.upgrade_auth {
                        if let Some(c) = SlackClient::new() {
                            c.send_message(format!(
                                "Program account upgrade authority change detected for {} (program data account: {}) | Old upgrade authority {:?} - New upgrade authority {:?}",
                                cached.name, cached.address, p.upgrade_auth, upgrade_authority_address
                            ))
                            .await;
                        }
                        if let Some(mut c) = Mattermost::new() {
                            c.send_message(format!(
                                "Program account upgrade authority change detected for {} (program data account: {}) | Old upgrade authority {:?} - New upgrade authority {:?}",
                                cached.name, cached.address, p.upgrade_auth, upgrade_authority_address
                            ));
                        }
                        p.upgrade_auth = upgrade_authority_address;
                        change_in_pgr = true;
                    }
                }
            };

            if let Err(e) = database.commit_account(cached, change_in_pgr).await {
                eprintln!("Failed to commit account to database with {}", e);
            }
        }
    }
}
