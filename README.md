<h1 align="center">Vault watcher</h1>
<br />
<p align="center">
<img width="250" src="https://ftx.com/static/media/fida.ce20eedf.svg"/>
</p>
<br />

<h2 align="center">Monitoring critical spl-token accounts in real time</h2>
<br/>

<br />
<h2 align="center">Table of contents</h2>
<br />

1. [Introduction](#introduction)
2. [Usage](#usage)
3. [Configuration](#configuration)
4. [Configuration examples](#configuration-examples)

<br />
<a name="introduction"></a>
<h2 align="center">Introduction</h2>
<br />

This security utility can be deployed as a container on a server to enable the monitoring of mission-critical spl-token accounts. Thanks to compatibility with Slack notifications, it constitutes the basis for a simple early warning system able to detect suspicious variations in account balances. As such, it can help help detect critical bugs in production systems, as well as intentional attacks resulting from contract exploits, key theft, rogue agents/teams, etc.

<br />
<a name="usage"></a>
<h2 align="center">Usage</h2>
<br />

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

<br />
<a name="configuration"></a>
<h2 align="center">Configuration</h2>
<br />

### `config.json`

| Field Name    | Type    | Description                                                                          |
| ------------- | ------- | ------------------------------------------------------------------------------------ |
| endpoint      | string  | URL for the Solana RPC endpoint to connect to                                        |
| refreshPeriod | integer | Period between account polls in milliseconds. All polls are written to the database. |

### `accounts.json`

An array of accounts objects containing

| Field Name      | Type    | Description                                                                                                          |
| --------------- | ------- | -------------------------------------------------------------------------------------------------------------------- |
| name            | string  | User-readable identifier for the account to monitor. Maximum length is 50 characters.                                |
| address         | string  | The public key in base58 format for the account to monitor                                                           |
| maxChange       | float   | The maximum allowable amplitude of balance change.                                                                   |
| maxChangePeriod | integer | Maximum number of milliseconds over which a maxChange balance variation is allowed without triggering a notification |

<br />
<a name="configuration-examples"></a>
<h2 align="center">Configuration examples</h2>
<br />

For example, if your endpoint is `https://solana-api.projectserum.com` and you want to poll data every `5s`:

```json
{
  "refreshPeriod": 5000,
  "endpoint": "https://solana-api.projectserum.com"
}
```

For example if you want to monitor `2Av1qmnqjLcnA9cpNduUL9BQcitobBq1Fiu7ZA4t45a6` and allow a max variation of `1,000` tokens every `5s`:

```json
{
  "address": "2Av1qmnqjLcnA9cpNduUL9BQcitobBq1Fiu7ZA4t45a6",
  "maxChange": 1000,
  "maxChangePeriod": 5000,
  "name": "My token account"
}
```
