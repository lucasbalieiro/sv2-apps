# Stratum V2 Docker Compose Setup

This repository contains a `docker-compose.yml` setup to run various **Stratum V2 roles** (template provider, pool, job declarator, miner, translator proxy) in a cross-platform environment.
It is intended as an **example setup** that you can adapt to your own infrastructure.

---

## Overview

The compose file defines the following services:

| Service         | Role                                                         |
| --------------- | ------------------------------------------------------------ |
| `bitcoin_sv2`   | Template Provider (Bitcoin node) for Stratum V2 apps.        |
| `pool_sv2`      | Stratum V2 Pool server.                                      |
| `jds_sv2`       | Job Declarator Server for pool use.                          |
| `jd_client_sv2` | Job Declarator Client for miners.                            |
| `tproxy_sv2`    | Translator Proxy for Stratum V2 <=> Stratum V1 compatibility. |

All services are connected via a dedicated Docker network (`sv2_apps`) with fixed IP addresses for predictable inter-service communication. And also because the SV2 Apps until now, does not support hostname lookup, so it needs static IPs

---

## Why Dockerize the Template Provider?

One of the main design decisions was to **dockerize the template provider (`bitcoin_sv2`)**.
This choice was made because **Docker networking behaves differently across platforms**:

* **Linux**: You could use `network_mode: host` for direct host networking.
* **macOS**: Host networking is not supported; you could use `host.docker.internal`, but this is **not** available on Linux.
* **Cross-platform goal**: Using host-dependent workarounds would require separate configurations.

By dockerizing the template provider, all services can **"see"** it directly via the internal Docker network without special configuration.

### Drawbacks

* The container will need to perform **Initial Block Download (IBD)**, which is time and resource intensive.
* To skip IBD, you can mount an existing `.bitcoin` data directory using the `BITCOIN_DATA_DIR` environment variable.

---

## Sharing Bitcoin Data

The `BITCOIN_DATA_DIR` environment variable controls where Bitcoin data is stored:

* **If provided**: The container will mount your existing `.bitcoin` folder.
  Example:

  ```bash
  BITCOIN_DATA_DIR=/path/to/.bitcoin docker compose --profile pool_apps up
  ```

  **Warning:** On some platforms, Docker may change file permissions. Make a backup before using your real data directory.

* **If not provided**: Defaults to a relative path `./.bitcoin` in this repository.

This variable is necessary because the `.bitcoin` path can vary across operating systems.

---

## Profiles

The compose file uses Docker Compose [profiles](https://docs.docker.com/compose/profiles/) to control which services start:

| Profile             | Runs Services                                            |
| ------------------- | -------------------------------------------------------- |
| `template_provider` | Runs only the Bitcoin template provider (`bitcoin_sv2`). |
| `pool_apps`         | Runs `bitcoin_sv2`, `pool_sv2`, and `jds_sv2`.           |
| `miner_apps`        | Runs `bitcoin_sv2`, `jd_client_sv2`, and `tproxy_sv2`.   |

**Examples:**

```bash
# Run only the pool setup
docker compose --profile pool_apps up

# Run only the miner setup
docker compose --profile miner_apps up

# Run everything
docker compose --profile pool_apps --profile miner_apps up
```

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

## Usage Example

### Running with custom Bitcoin data directory:

```bash
BITCOIN_DATA_DIR=/home/user/.bitcoin docker compose --profile pool_apps up
```

### Running with default `.bitcoin` directory in project root:

```bash
docker compose --profile pool_apps up
```

---

## Important Notes

* This setup is **not production-ready**.
  It is an **example** showing how to use the Stratum V2 Docker images together.
* Production deployments will require:

  * Custom configuration files.
  * Security hardening.
  * Adjustments for your network and infrastructure layout.
* If you mount a real `.bitcoin` data directory, **back it up first** to avoid possible data corruption or permission changes from Docker.

---

## Network Details

* All services are on a custom `bridge` network `sv2_apps` with fixed IPs.
* This ensures stable internal addresses across restarts and avoids hostname resolution issues.
* The choice of fixed IPs is to keep inter-container connectivity predictable, especially when debugging or testing protocol interactions.

