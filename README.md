# miden-watch

A terminal UI for exploring Miden blockchain data in real-time. Connects to a Miden node via gRPC, polls for new blocks, and lets you browse blocks, transactions, and notes with vim-style navigation. All synced data is persisted locally in SQLite.

## Versioning

`miden-watch` versions match the Miden node version they are compatible with. For example, `miden-watch` v0.15.x is compatible with Miden node v0.15.x.

## Installation

```bash
cargo install miden-watch
```

## Usage

```
miden-watch [OPTIONS]
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--url <URL>` | Miden node RPC URL | `https://rpc.testnet.miden.io` |
| `--from <START>` | Where to start syncing: `tip`, `genesis`, or a block number | `tip` |
| `--to <BLOCK>` | Block number to stop syncing at | None (keeps tailing) |
| `--db-path <PATH>` | Path to SQLite database file | `~/.miden-watch/data.db` |

The `--url` flag can also be set via the `MIDEN_NODE_URL` environment variable.

### Examples

```bash
# Connect to testnet (the default) starting from the chain tip
miden-watch

# Sync all blocks from genesis
miden-watch --from genesis

# Sync a specific range
miden-watch --from 500 --to 1000

# Connect to a local node
miden-watch --url http://localhost:57291
```

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `k` | Move down / up |
| `{n}j` / `{n}k` | Jump n lines (e.g. `200j`) |
| `Ctrl+d` / `Ctrl+u` | Half-page down / up |
| `gg` | Go to top |
| `G` | Go to bottom |
| `Enter` / `l` | Drill into selection |
| `Esc` / `h` | Go back |
| `Ctrl+o` / `Ctrl+i` | Jump back / forward in history |
| `/` | Search by block number |
| `f` | Cycle block filter (All / Has txs / Has notes) |

### General

| Key | Action |
|-----|--------|
| `?` | Toggle help overlay |
| `!` | Toggle error log |
| `c` | Clear error log (when viewing) |
| `q` | Quit |
| `Ctrl+c` | Force quit |

## Features

### Block list
Scrollable list of blocks showing block number, timestamp, relative age, time delta between consecutive blocks, and transaction/note counts. New blocks appear at the top automatically when viewing the head, with a brief green flash highlight.

### Block filtering
Press `f` to cycle through filters: all blocks, only blocks with transactions, or only blocks with notes. The active filter is shown in the title bar.

### Block detail
Split view with block header fields on the left and the transaction list on the right. Press Enter on a transaction to see its details.

### Transaction detail
Shows transaction ID, account ID, and input/output note counts.

### Note detail
Shows note ID, sender, type, tag, and index.

### Error log
Errors from the sync process are collected and indicated by a warning symbol in the status bar. Press `!` to view the full error log, navigate with `j`/`k`, and press `c` to clear.

### Sync progress
The status bar shows sync progress while catching up (e.g. `Syncing 500/1000`) and a green checkmark when synced.

## Data Persistence

Blocks, transactions, and notes are stored in a local SQLite database (`~/.miden-watch/data.db` by default). On restart, previously synced data is loaded from the database and syncing resumes from where it left off, avoiding redundant fetches.
