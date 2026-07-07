use color_eyre::Result;
use rusqlite::Connection;

use crate::types::{BlockInfo, NoteInfo, TransactionInfo};

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn new(path: &std::path::Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS blocks (
                block_num INTEGER PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                version INTEGER NOT NULL,
                prev_block_commitment TEXT NOT NULL,
                chain_commitment TEXT NOT NULL,
                account_root TEXT NOT NULL,
                nullifier_root TEXT NOT NULL,
                note_root TEXT NOT NULL,
                tx_commitment TEXT NOT NULL,
                tx_kernel_commitment TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS transactions (
                tx_id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                account_storage_mode TEXT NOT NULL DEFAULT '',
                block_num INTEGER NOT NULL,
                input_note_count INTEGER NOT NULL,
                output_note_count INTEGER NOT NULL,
                FOREIGN KEY (block_num) REFERENCES blocks(block_num)
            );

            CREATE TABLE IF NOT EXISTS notes (
                note_id TEXT PRIMARY KEY,
                block_num INTEGER NOT NULL,
                sender TEXT NOT NULL,
                note_type TEXT NOT NULL,
                tag INTEGER NOT NULL,
                note_index INTEGER NOT NULL,
                standard_type TEXT,
                target TEXT,
                FOREIGN KEY (block_num) REFERENCES blocks(block_num)
            );

            CREATE INDEX IF NOT EXISTS idx_tx_block ON transactions(block_num);
            CREATE INDEX IF NOT EXISTS idx_note_block ON notes(block_num);
            CREATE INDEX IF NOT EXISTS idx_tx_account ON transactions(account_id);
            CREATE INDEX IF NOT EXISTS idx_note_sender ON notes(sender);
            ",
        )?;

        // Migrate DB files created before the standard_type/target columns existed.
        // On a fresh DB these columns already exist, so the ALTER fails with
        // "duplicate column name" and is harmlessly ignored.
        let _ = conn.execute("ALTER TABLE notes ADD COLUMN standard_type TEXT", []);
        let _ = conn.execute("ALTER TABLE notes ADD COLUMN target TEXT", []);

        // Index on `target` must be created after the migration guarantees the column exists.
        conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_note_target ON notes(target);")?;

        Ok(Self { conn })
    }

    pub fn insert_block(&self, block: &BlockInfo) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO blocks (
                block_num, timestamp, version, prev_block_commitment, chain_commitment,
                account_root, nullifier_root, note_root, tx_commitment, tx_kernel_commitment
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                block.block_num,
                block.timestamp,
                block.version,
                block.prev_block_commitment,
                block.chain_commitment,
                block.account_root,
                block.nullifier_root,
                block.note_root,
                block.tx_commitment,
                block.tx_kernel_commitment,
            ],
        )?;
        Ok(())
    }

    pub fn insert_transactions(&self, txs: &[TransactionInfo]) -> Result<()> {
        for tx in txs {
            self.conn.execute(
                "INSERT OR REPLACE INTO transactions (
                    tx_id, account_id, account_storage_mode, block_num, input_note_count, output_note_count
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    tx.tx_id,
                    tx.account_id,
                    tx.account_storage_mode,
                    tx.block_num,
                    tx.input_note_count,
                    tx.output_note_count,
                ],
            )?;
        }
        Ok(())
    }

    pub fn insert_notes(&self, notes: &[NoteInfo]) -> Result<()> {
        for note in notes {
            self.conn.execute(
                "INSERT OR REPLACE INTO notes (
                    note_id, block_num, sender, note_type, tag, note_index, standard_type, target
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    note.note_id,
                    note.block_num,
                    note.sender,
                    note.note_type,
                    note.tag,
                    note.note_index,
                    note.standard_type,
                    note.target,
                ],
            )?;
        }
        Ok(())
    }

    /// Count blocks in a range (for progress reporting)
    pub fn count_blocks_in_range(&self, from_block: u32, to_block: Option<u32>) -> Result<u32> {
        let upper = to_block.unwrap_or(u32::MAX);
        let count: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM blocks WHERE block_num >= ?1 AND block_num <= ?2",
            rusqlite::params![from_block, upper],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Load a page of blocks (oldest-first) with tx/note counts.
    /// Returns blocks ordered by block_num ASC.
    pub fn get_blocks_page(
        &self,
        from_block: u32,
        to_block: Option<u32>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<BlockInfo>> {
        let upper = to_block.unwrap_or(u32::MAX);
        let mut stmt = self.conn.prepare(
            "SELECT b.block_num, b.timestamp, b.version, b.prev_block_commitment,
                    b.chain_commitment, b.account_root, b.nullifier_root, b.note_root,
                    b.tx_commitment, b.tx_kernel_commitment,
                    (SELECT COUNT(*) FROM transactions t WHERE t.block_num = b.block_num) as tx_count,
                    (SELECT COUNT(*) FROM notes n WHERE n.block_num = b.block_num) as note_count
             FROM blocks b
             WHERE b.block_num >= ?1 AND b.block_num <= ?2
             ORDER BY b.block_num ASC
             LIMIT ?3 OFFSET ?4",
        )?;
        let rows = stmt.query_map(rusqlite::params![from_block, upper, limit, offset], |row| {
            Ok(BlockInfo {
                block_num: row.get(0)?,
                timestamp: row.get(1)?,
                version: row.get(2)?,
                prev_block_commitment: row.get(3)?,
                chain_commitment: row.get(4)?,
                account_root: row.get(5)?,
                nullifier_root: row.get(6)?,
                note_root: row.get(7)?,
                tx_commitment: row.get(8)?,
                tx_kernel_commitment: row.get(9)?,
                tx_count: row.get::<_, i64>(10)? as usize,
                note_count: row.get::<_, i64>(11)? as usize,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Load transactions for a range of blocks in one query
    pub fn get_transactions_for_blocks(
        &self,
        from_block: u32,
        to_block: u32,
    ) -> Result<Vec<TransactionInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT tx_id, account_id, account_storage_mode, block_num, input_note_count, output_note_count
             FROM transactions WHERE block_num >= ?1 AND block_num <= ?2
             ORDER BY block_num ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![from_block, to_block], |row| {
            Ok(TransactionInfo {
                tx_id: row.get(0)?,
                account_id: row.get(1)?,
                account_storage_mode: row.get(2)?,
                block_num: row.get(3)?,
                input_note_count: row.get::<_, i64>(4)? as usize,
                output_note_count: row.get::<_, i64>(5)? as usize,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Load notes for a range of blocks in one query
    pub fn get_notes_for_blocks(&self, from_block: u32, to_block: u32) -> Result<Vec<NoteInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT note_id, block_num, sender, note_type, tag, note_index, standard_type, target
             FROM notes WHERE block_num >= ?1 AND block_num <= ?2
             ORDER BY block_num ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![from_block, to_block], note_from_row)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Transactions authored by an account, newest block first.
    pub fn get_transactions_for_account(&self, account_id: &str) -> Result<Vec<TransactionInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT tx_id, account_id, account_storage_mode, block_num, input_note_count, output_note_count
             FROM transactions WHERE account_id = ?1
             ORDER BY block_num DESC",
        )?;
        let rows = stmt.query_map(rusqlite::params![account_id], |row| {
            Ok(TransactionInfo {
                tx_id: row.get(0)?,
                account_id: row.get(1)?,
                account_storage_mode: row.get(2)?,
                block_num: row.get(3)?,
                input_note_count: row.get::<_, i64>(4)? as usize,
                output_note_count: row.get::<_, i64>(5)? as usize,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Notes created by an account (sender), newest block first.
    pub fn get_notes_by_sender(&self, sender: &str) -> Result<Vec<NoteInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT note_id, block_num, sender, note_type, tag, note_index, standard_type, target
             FROM notes WHERE sender = ?1
             ORDER BY block_num DESC",
        )?;
        let rows = stmt.query_map(rusqlite::params![sender], note_from_row)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Notes addressed to an account (P2ID/P2IDE target), newest block first.
    pub fn get_notes_by_target(&self, target: &str) -> Result<Vec<NoteInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT note_id, block_num, sender, note_type, tag, note_index, standard_type, target
             FROM notes WHERE target = ?1
             ORDER BY block_num DESC",
        )?;
        let rows = stmt.query_map(rusqlite::params![target], note_from_row)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get_latest_block_num(&self) -> Result<Option<u32>> {
        let result = self
            .conn
            .query_row("SELECT MAX(block_num) FROM blocks", [], |row| {
                row.get::<_, Option<u32>>(0)
            })?;
        Ok(result)
    }
}

/// Build a [`NoteInfo`] from a row selecting the 8 note columns in schema order.
fn note_from_row(row: &rusqlite::Row) -> rusqlite::Result<NoteInfo> {
    Ok(NoteInfo {
        note_id: row.get(0)?,
        block_num: row.get(1)?,
        sender: row.get(2)?,
        note_type: row.get(3)?,
        tag: row.get(4)?,
        note_index: row.get(5)?,
        standard_type: row.get(6)?,
        target: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: &str, sender: &str, target: Option<&str>, std: Option<&str>, block: u32) -> NoteInfo {
        NoteInfo {
            note_id: id.to_string(),
            block_num: block,
            sender: sender.to_string(),
            note_type: "Public".to_string(),
            tag: 1,
            note_index: 0,
            standard_type: std.map(str::to_string),
            target: target.map(str::to_string),
        }
    }

    fn block(block_num: u32) -> BlockInfo {
        BlockInfo {
            block_num,
            timestamp: 0,
            version: 1,
            prev_block_commitment: String::new(),
            chain_commitment: String::new(),
            account_root: String::new(),
            nullifier_root: String::new(),
            note_root: String::new(),
            tx_commitment: String::new(),
            tx_kernel_commitment: String::new(),
            tx_count: 0,
            note_count: 0,
        }
    }

    fn tx(id: &str, account: &str, block: u32) -> TransactionInfo {
        TransactionInfo {
            tx_id: id.to_string(),
            account_id: account.to_string(),
            account_storage_mode: "regular".to_string(),
            block_num: block,
            input_note_count: 0,
            output_note_count: 1,
        }
    }

    fn temp_path(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("mw_{}_{}.db", tag, std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn note_columns_roundtrip_and_account_queries() {
        let path = temp_path("acct");
        let store = Store::new(&path).unwrap();
        for bn in [5, 6, 7] {
            store.insert_block(&block(bn)).unwrap();
        }
        store
            .insert_notes(&[
                note("0xaaa", "0xsender1", Some("0xtarget1"), Some("P2ID"), 5),
                note("0xbbb", "0xsender1", None, Some("SWAP"), 6),
                note("0xccc", "0xsender2", Some("0xsender1"), Some("P2ID"), 7),
            ])
            .unwrap();
        store
            .insert_transactions(&[tx("0xtx1", "0xsender1", 5), tx("0xtx2", "0xother", 6)])
            .unwrap();

        // New columns survive a round-trip.
        let all = store.get_notes_for_blocks(0, 100).unwrap();
        let p2id = all.iter().find(|n| n.note_id == "0xaaa").unwrap();
        assert_eq!(p2id.standard_type.as_deref(), Some("P2ID"));
        assert_eq!(p2id.target.as_deref(), Some("0xtarget1"));

        // Account-history queries.
        assert_eq!(store.get_notes_by_sender("0xsender1").unwrap().len(), 2);
        let received = store.get_notes_by_target("0xsender1").unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].note_id, "0xccc");
        let txs = store.get_transactions_for_account("0xsender1").unwrap();
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].tx_id, "0xtx1");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn migrates_db_predating_new_columns() {
        let path = temp_path("migrate");
        // Simulate an old DB whose notes table lacks standard_type/target.
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE notes (
                    note_id TEXT PRIMARY KEY, block_num INTEGER NOT NULL, sender TEXT NOT NULL,
                    note_type TEXT NOT NULL, tag INTEGER NOT NULL, note_index INTEGER NOT NULL
                );
                INSERT INTO notes VALUES ('0xold', 1, '0xs', 'Private', 0, 0);",
            )
            .unwrap();
        }

        // Opening via Store::new must add the columns + target index without error.
        let store = Store::new(&path).unwrap();
        let old = store.get_notes_for_blocks(0, 10).unwrap();
        assert_eq!(old.len(), 1);
        assert_eq!(old[0].standard_type, None);
        assert_eq!(old[0].target, None);

        // And new inserts using the new columns work against the migrated DB.
        store
            .insert_notes(&[note("0xnew", "0xs", Some("0xt"), Some("P2ID"), 2)])
            .unwrap();
        assert_eq!(store.get_notes_by_target("0xt").unwrap().len(), 1);

        let _ = std::fs::remove_file(&path);
    }
}
