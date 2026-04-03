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
                FOREIGN KEY (block_num) REFERENCES blocks(block_num)
            );

            CREATE INDEX IF NOT EXISTS idx_tx_block ON transactions(block_num);
            CREATE INDEX IF NOT EXISTS idx_note_block ON notes(block_num);
            ",
        )?;
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
                    tx_id, account_id, block_num, input_note_count, output_note_count
                ) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    tx.tx_id,
                    tx.account_id,
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
                    note_id, block_num, sender, note_type, tag, note_index
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    note.note_id,
                    note.block_num,
                    note.sender,
                    note.note_type,
                    note.tag,
                    note.note_index,
                ],
            )?;
        }
        Ok(())
    }

    pub fn get_blocks(&self, limit: usize, offset: usize) -> Result<Vec<BlockInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT b.block_num, b.timestamp, b.version, b.prev_block_commitment,
                    b.chain_commitment, b.account_root, b.nullifier_root, b.note_root,
                    b.tx_commitment, b.tx_kernel_commitment,
                    (SELECT COUNT(*) FROM transactions t WHERE t.block_num = b.block_num) as tx_count,
                    (SELECT COUNT(*) FROM notes n WHERE n.block_num = b.block_num) as note_count
             FROM blocks b
             ORDER BY b.block_num DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![limit, offset], |row| {
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

    pub fn get_transactions_for_block(&self, block_num: u32) -> Result<Vec<TransactionInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT tx_id, account_id, block_num, input_note_count, output_note_count
             FROM transactions WHERE block_num = ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![block_num], |row| {
            Ok(TransactionInfo {
                tx_id: row.get(0)?,
                account_id: row.get(1)?,
                block_num: row.get(2)?,
                input_note_count: row.get::<_, i64>(3)? as usize,
                output_note_count: row.get::<_, i64>(4)? as usize,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get_notes_for_block(&self, block_num: u32) -> Result<Vec<NoteInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT note_id, block_num, sender, note_type, tag, note_index
             FROM notes WHERE block_num = ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![block_num], |row| {
            Ok(NoteInfo {
                note_id: row.get(0)?,
                block_num: row.get(1)?,
                sender: row.get(2)?,
                note_type: row.get(3)?,
                tag: row.get(4)?,
                note_index: row.get(5)?,
            })
        })?;
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
