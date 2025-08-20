# ⚠️ Important: Sync Bitcoin First!

Before running pool or miner apps, you **must have a fully synced Bitcoin testnet4 node**.
If no `.bitcoin` data directory is provided, the container will fallback to `./.bitcoin` in the repo and start a full **Initial Block Download (IBD)**.

It is **highly recommended** to run only the template provider first and wait for it to complete IBD:

```bash
docker compose --profile template_provider up
```

Only after the node is synced should you start the pool or miner apps.

If you already have testnet4 data from another node, see [Using Pre-Downloaded Data](#using-pre-downloaded-data) to skip IBD.

---

# Stratum V2 Docker Compose Setup

This repository contains a `docker-compose.yml` setup to run various **Stratum V2 roles** (template provider, pool, job declarator, miner, translator proxy) in a cross-platform environment.
It is intended as an **example setup** that you can adapt to your own infrastructure.

---

## Quickstart

Clone the repo and run one of the following commands depending on your use case:

```bash
# 0. (Required!) First let the Bitcoin node complete IBD on testnet4
docker compose --profile template_provider up

# 1. Run only the Pool setup (Bitcoin node + Pool + Job Declarator Server)
docker compose --profile pool_apps up

# 2. Run Miner setup (Bitcoin node + Job Declarator Client)
docker compose --profile miner_apps up

# 3. Run Miner with Translator Proxy (Bitcoin node + JD Client + TProxy)
docker compose --profile miner_apps_with_tproxy up

# 4. Run absolutely everything
docker compose --profile all up
```

---

## Using Pre-Downloaded Data

If you already have a **synced testnet4 chain** from another Bitcoin node, you can reuse it and avoid redoing IBD.

### Option 1 – Point `BITCOIN_DATA_DIR` to your existing `.bitcoin`

Suppose your existing node stores blocks in `/home/user/.bitcoin/testnet4`. You can point directly to it:

```bash
BITCOIN_DATA_DIR=/home/user/.bitcoin docker compose --profile template_provider up
```

This will mount your full `.bitcoin` directory into the container.
⚠️ **Warning:** Docker may change file permissions — make a backup before mounting your production data.

---

### Option 2 – Copy only the `testnet4` folder into this repo

The repo is already prepared with a `docker/.bitcoin/bitcoin.conf` configured for the Template Provider.
You can simply copy your `testnet4` folder here:

```bash
cp -r /home/user/.bitcoin/testnet4 ./docker/.bitcoin/testnet4
```

Now you don’t need to set `BITCOIN_DATA_DIR` at all. Since the compose file falls back to the repo’s `./.bitcoin` folder, it will use the copied chainstate directly:

```bash
docker compose --profile template_provider up
```

This is the **simplest method** if you want to avoid touching your live node’s `.bitcoin` directory.

---

## Overview

The compose file defines the following services:

| Service         | Role                                                          |
| --------------- | ------------------------------------------------------------- |
| `bitcoin_sv2`   | Template Provider (Bitcoin node) for Stratum V2 apps.         |
| `pool_sv2`      | Stratum V2 Pool server.                                       |
| `jds_sv2`       | Job Declarator Server for pool use.                           |
| `jd_client_sv2` | Job Declarator Client for miners.                             |
| `tproxy_sv2`    | Translator Proxy for Stratum V2 <=> Stratum V1 compatibility. |

All services are connected via a dedicated Docker network (`sv2_apps`) with fixed IP addresses for predictable inter-service communication.
This is necessary because **Stratum V2 apps do not support hostname lookup yet**, so static IPs are required.

---

## Profiles

The compose file uses Docker Compose [profiles](https://docs.docker.com/compose/profiles/) to control which services start:

| Profile                  | Runs Services                                            |
| ------------------------ | -------------------------------------------------------- |
| `template_provider`      | Runs only the Bitcoin template provider (`bitcoin_sv2`). |
| `pool_apps`              | Runs `bitcoin_sv2`, `pool_sv2`, and `jds_sv2`.           |
| `miner_apps`             | Runs `bitcoin_sv2` and `jd_client_sv2`.                  |
| `miner_apps_with_tproxy` | Runs `bitcoin_sv2`, `jd_client_sv2`, and `tproxy_sv2`.   |
| `all`                    | Runs everything.                                         |

---

## Customizing Configurations

Each service can be customized by mounting your own config file via `volumes`.
Example for the pool:

```yaml
volumes:
  - ./config/my-custom-pool-config.toml:/app/config.toml:ro
```

If no custom config is provided, the image will fall back to an **example configuration** pointing to the Stratum V2 Community Server.

You should replace these configs with ones tailored to your infrastructure.

---

## Important Notes

* This setup is **not production-ready**.
  It is an **example** showing how to use the Stratum V2 Docker images together.
* Production deployments will require:

  * Custom configuration files.
  * Security hardening.
  * Adjustments for your network and infrastructure layout.
* If you mount a real `.bitcoin` data directory, **back it up first** to avoid possible data corruption or permission changes from Docker.
* **Do not skip IBD**: the Template Provider must be fully synced before the other services can function properly.

