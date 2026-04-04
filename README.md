# miden-watch

A terminal UI for exploring Miden blockchain data in real-time. Connects to a Miden node via gRPC, polls for new blocks, and lets you browse blocks, transactions, and notes with vim-style navigation. All synced data is persisted locally in SQLite.

## Usage

```
cargo run -- [OPTIONS]
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--url <URL>` | Miden node RPC URL | `http://localhost:57291` |
| `--from <START>` | Where to start syncing: `tip`, `genesis`, or a block number | `tip` |
| `--to <BLOCK>` | Block number to stop syncing at | None (keeps tailing) |
| `--db-path <PATH>` | Path to SQLite database file | `~/.miden-watch/data.db` |

The `--url` flag can also be set via the `MIDEN_NODE_URL` environment variable.

### Examples

```bash
# Connect to a local node starting from the chain tip
cargo run

# Sync all blocks from genesis
cargo run -- --from genesis

# Sync a specific range
cargo run -- --from 500 --to 1000

# Connect to testnet
cargo run -- --url https://rpc.testnet.miden.io
```

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `k` | Move down / up |
| `Ctrl+d` / `Ctrl+u` | Half-page down / up |
| `gg` | Go to top |
| `G` | Go to bottom |
| `Enter` / `l` | Drill into selection |
| `Esc` / `h` | Go back |
| `Ctrl+o` / `Ctrl+i` | Jump back / forward in history |
| `/` | Search by block number |
| `{n}j` / `{n}k` | Jump n lines (e.g. `200j`) |

### General

| Key | Action |
|-----|--------|
| `?` | Toggle help |
| `!` | Toggle error log |
| `c` | Clear error log (when viewing) |
| `q` | Quit |
| `Ctrl+c` | Force quit |

## Data Persistence

Blocks, transactions, and notes are stored in a local SQLite database (`~/.miden-watch/data.db` by default). On restart, previously synced data is loaded from the database and syncing resumes from where it left off.

## Views

- **Block list** -- scrollable list of blocks showing block number, timestamp, and transaction/note counts. New blocks appear at the top automatically when you're viewing the head.
- **Block detail** -- split view with block header fields on the left and the transaction list on the right.
- **Transaction detail** -- shows transaction ID, account ID, and input/output note counts.
- **Note detail** -- shows note ID, sender, type, tag, and index.
