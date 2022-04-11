# Vault-watcher : monitoring critical spl-token accounts in real time

This security utility can be deployed as a container on a server to enable the monitoring of mission-critical spl-token accounts. Thanks to compatibility with Slack notifications, it constitutes the basis for a simple early warning system able to detect suspicious variations in account balances. As such, it can help help detect critical bugs in production systems, as well as intentional attacks resulting from contract exploits, key theft, rogue agents/teams, etc.

## Usage

Although the `vault-watcher` service can be used directly as a binary with a custom postgres instance, we recommend using `docker-compose`.

```bash
git clone git@github.com:Bonfida/vault-watcher.git
cd vault-watcher
cp _accounts.json accounts.json
cp _config.json config.json
```

The `accounts.json` and `config.json` should then be edited to configure the service. Optionally, the `config.env` file can be edited as well. Once this is done, we start the docker containers.

```bash
sudo docker-compose build
sudo docker-compose up
```

The Postgres database can be directly accessed. In addition, a grafana instance with a simple provisioned dashboard can be found running at `http://localhost:3000` by default.

## Configuration

### `config.json`

| Field Name    | Type    | Description                                                                          |
|---------------|---------|--------------------------------------------------------------------------------------|
| endpoint      | string  | URL for the Solana RPC endpoint to connect to                                        |
| refreshPeriod | integer | Period between account polls in milliseconds. All polls are written to the database. |

### `accounts.json`

An array of accounts objects containing

| Field Name      | Type    | Description                                                                                                          |
|-----------------|---------|----------------------------------------------------------------------------------------------------------------------|
| name            | string  | User-readable identifier for the account to monitor. Maximum length is 50 characters.                                |
| address         | string  | The public key in base58 format for the account to monitor                                                           |
| maxChange       | float   | The maximum allowable amplitude of balance change.                                                                   |
| maxChangePeriod | integer | Maximum number of milliseconds over which a maxChange balance variation is allowed without triggering a notification |
