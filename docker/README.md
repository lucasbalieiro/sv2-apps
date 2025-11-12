# SV2 Docker Compose Setup

This repository provides a ready-to-run Docker Compose setup for the full SV2 stack, including:

* **Pool SV2 Service**
* **Job Declarator (JD) Server**
* **JD Client**
* **Translator Proxy**

The services are wired together on a dedicated Docker network and can be enabled via Compose profiles depending on whether you're running pool-side components, miner-side components, or everything at once.

---

## Requirements

* Docker
* Docker Compose (v2+)
* A fully synced **Bitcoin Core v30+** node running on **testnet4** (or any network you prefer)
* Access to the `node.sock` file in your Bitcoin data directory

---

### Configuring Bitcoin Core

After downloading Bitcoin Core, you **must** configure it for the network you want to use and for the RPC settings required by the JD Server.

A minimal `bitcoin.conf` for **testnet4** looks like this:

```ini
[testnet4]
server=1
rpcuser=username
rpcpassword=password
rpcbind=0.0.0.0
rpcallowip=0.0.0.0/0
```

If you choose a different network (signet, mainnet, etc.), make sure the matching section exists and that your RPC credentials line up with your `jds-config.toml`.

---

### IPC Requirements (pool + jd_client)

Some components, like the **pool** and **jd_client**, communicate with Bitcoin Core over **IPC** (via `node.sock`).
For this to work, Bitcoin Core must be started with IPC enabled. Whatever network you run, you must start Bitcoin Core with `-ipcbind=unix`

Example: starting a **testnet4** node with IPC bindings:

```bash
./bitcoin-30.0/bin/bitcoin -m node -testnet4 -ipcbind=unix
```

You'll also need to wait for the node to complete Initial Block Download (IBD).

---

## Setting the Bitcoin Socket Path

These are the typical paths for the `node.sock` file.
| Network  | Default Path                               |
| -------- | ------------------------------------------ |
| mainnet  | `~/.bitcoin/node.sock`                     |
| testnet4 | `~/.bitcoin/testnet4/node.sock`            |
| signet   | `~/.bitcoin/signet/node.sock`              |
| macOS    | Inside `~/Library/Application Support/...` |

Two of the services (`pool_sv2` and `jd_client_sv2`) need access to your local Bitcoin Core `node.sock`.
Because this path differs across operating systems, it is **not hardcoded**.
Instead, you must provide it via an environment variable:

### 1. Create a `.env` file (recommended)

In the same directory as `docker-compose.yml`, create a `.env` file:

```
BITCOIN_SOCKET_PATH=/absolute/path/to/your/node.sock
```
Make sure the path is correct, if there are spaces (like `Application Support`), keep the value unquoted.

### 2. Or export it manually

```bash
export BITCOIN_SOCKET_PATH="/Users/<youruser>/Library/Application Support/Bitcoin/signet/node.sock"
```

---

## Running the Stack

This compose file uses *profiles* so you can run only what you need.

### Run everything

```bash
docker compose --profile all up --build
```

### Run only pool-side services

```bash
docker compose --profile pool_apps up --build
```

### Run only miner-side services

```bash
docker compose --profile miner_apps up --build
```

To run them in the background:

```bash
docker compose --profile all up -d
```

---

## Services Overview

Each service has **its own configuration file**, and you *must* review and adjust these files to match your local setup, Bitcoin network, and intended use.
If something behaves weirdly, the config file is usually where the problem lives.

### **pool_sv2**

* Uses `./config/pool-config.toml`
* Exposes port **34254**
* Connects to your Bitcoin Core `node.sock` via `BITCOIN_SOCKET_PATH`

### **jd_server_sv2**

* Uses `./config/jds-config.toml`
* Exposes port **34264**
* **Important:** settings like `core_rpc_port` need to match the network your Bitcoin node is running on.
  For reference, Bitcoin Core uses different REST/RPC ports per network:
  [https://github.com/bitcoin/bitcoin/blob/c66e988754391a094af93ef2a9127200d093b669/doc/REST-interface.md?plain=1#L6-L7](https://github.com/bitcoin/bitcoin/blob/c66e988754391a094af93ef2a9127200d093b669/doc/REST-interface.md?plain=1#L6-L7)

### **jd_client_sv2**

* Uses `./config/jdc-config.toml`
* Exposes port **34265**
* Also mounts the Bitcoin `node.sock` via `BITCOIN_SOCKET_PATH`

### **tproxy_sv2**

* Uses `./config/proxy-config.toml`
* Exposes port **34255**

---

All configs in this repository are preset for **testnet4**.
If you want to run on **signet**, **mainnet**, or any other network, double-check:

* `core_rpc_port`
* any network selectors
* Bitcoin Core paths
* node socket location

Ignoring these small details is the fastest way to get stuck, so give each config a quick pass before spinning things up.

---

## Notes

* Double-check file permissions if the Bitcoin socket fails to mount.

## Docker Image Tags

Each service image is available on Docker Hub with versioned tags.
Tags start at **`v0.1.0`** and will continue incrementing with future releases.

You can choose:

* A **specific version tag** (e.g. `v0.1.0`) for predictable, repeatable deployments.
* The **`latest`** tag if you simply want the most recent released image.

Example:

```yaml
image: pool_sv2:v0.1.0   # pinned version
# or
image: pool_sv2:latest   # latest release
```

This applies to all images: `pool_sv2`, `jd_server`, `jd_client_sv2`, and `translator_sv2`.
