use std::collections::HashMap;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use rust_decimal::Decimal;

use crate::model::{Amount, ClientId, DepositRecord, TransactionId};

pub trait TransactionStore {
    fn upsert(&mut self, record: DepositRecord) -> Result<(), Box<dyn std::error::Error>>;

    fn get(&self, tx: TransactionId) -> Result<Option<DepositRecord>, Box<dyn std::error::Error>>;
}

pub struct InMemoryTransactionStore {
    records: HashMap<TransactionId, DepositRecord>,
}

impl Default for InMemoryTransactionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryTransactionStore {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }
}

impl TransactionStore for InMemoryTransactionStore {
    fn upsert(&mut self, record: DepositRecord) -> Result<(), Box<dyn std::error::Error>> {
        self.records.insert(record.tx, record);
        Ok(())
    }

    fn get(&self, tx: TransactionId) -> Result<Option<DepositRecord>, Box<dyn std::error::Error>> {
        Ok(self.records.get(&tx).cloned())
    }
}

/// A [`TransactionStore`] that keeps records in memory up to a configurable
/// `capacity`, then spills them to a SQLite database in a single batched
/// transaction and clears the in-memory cache. Reads consult the in-memory
/// cache first and fall back to SQLite on a cache miss, so the amount of
/// memory used is bounded regardless of how many records have been seen.
pub struct SqliteTransactionStore {
    records: HashMap<TransactionId, DepositRecord>,
    conn: Connection,
    capacity: usize,
}

/// Specifies the capacity of the in-memory cache for the [`SqliteTransactionStore`].
pub enum Capacity {
    /// Capacity measured in number of elements (records).
    Elements(usize),
    /// Capacity measured in megabytes.
    /// This is an approximate measure, as the actual memory usage may vary depending on the system and Rust's memory layout.
    Megabytes(usize),
}

impl SqliteTransactionStore {
    /// Opens (creating if needed) a SQLite database at `db_path`, used to
    /// overflow records once more than `capacity` are held in memory.
    pub fn new(
        db_path: impl AsRef<Path>,
        capacity: Capacity,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS deposit_records (
                 tx INTEGER PRIMARY KEY,
                 client INTEGER NOT NULL,
                 amount TEXT NOT NULL,
                 disputed INTEGER NOT NULL
             );",
        )?;

        Ok(Self {
            records: HashMap::new(),
            conn,
            capacity: match capacity {
                Capacity::Elements(n) => n,
                Capacity::Megabytes(mb) => {
                    mb * 1_024 * 1_024 / std::mem::size_of::<DepositRecord>()
                }
            },
        })
    }

    /// Writes all currently cached records to SQLite in a single transaction
    /// (using one prepared statement reused for every row, rather than
    /// committing row by row), then clears the in-memory cache.
    fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.records.is_empty() {
            return Ok(());
        }

        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO deposit_records (tx, client, amount, disputed) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(tx) DO UPDATE SET
                     client = excluded.client,
                     amount = excluded.amount,
                     disputed = excluded.disputed",
            )?;

            for record in self.records.values() {
                stmt.execute(params![
                    record.tx.as_u32(),
                    record.client.as_u16(),
                    record.amount.as_decimal().to_string(),
                    record.disputed,
                ])?;
            }
        }
        tx.commit()?;

        self.records.clear();
        Ok(())
    }

    fn get_from_db(
        &self,
        tx: TransactionId,
    ) -> Result<Option<DepositRecord>, Box<dyn std::error::Error>> {
        let row = self
            .conn
            .query_row(
                "SELECT client, amount, disputed FROM deposit_records WHERE tx = ?1",
                params![tx.as_u32()],
                |row| {
                    let client: u16 = row.get(0)?;
                    let amount: String = row.get(1)?;
                    let disputed: bool = row.get(2)?;
                    Ok((client, amount, disputed))
                },
            )
            .optional()?;

        let Some((client, amount, disputed)) = row else {
            return Ok(None);
        };

        let decimal: Decimal = amount.parse()?;
        let mut record = DepositRecord::new(ClientId::new(client), tx, Amount::try_from(decimal)?);
        if disputed {
            record.mark_disputed();
        }
        Ok(Some(record))
    }
}

impl TransactionStore for SqliteTransactionStore {
    fn upsert(&mut self, record: DepositRecord) -> Result<(), Box<dyn std::error::Error>> {
        self.records.insert(record.tx, record);
        if self.records.len() >= self.capacity {
            self.flush()?;
        }
        Ok(())
    }

    fn get(&self, tx: TransactionId) -> Result<Option<DepositRecord>, Box<dyn std::error::Error>> {
        if let Some(record) = self.records.get(&tx) {
            return Ok(Some(record.clone()));
        }

        self.get_from_db(tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use tempfile::NamedTempFile;

    fn record(client: u16, tx: u32, amount: &str) -> DepositRecord {
        DepositRecord::new(
            ClientId::new(client),
            TransactionId::new(tx),
            Amount::try_from(amount.parse::<Decimal>().unwrap()).unwrap(),
        )
    }

    #[test]
    fn get_returns_none_for_unknown_tx() {
        let db_path = NamedTempFile::new().unwrap();
        let store = SqliteTransactionStore::new(db_path.path(), Capacity::Elements(10)).unwrap();

        assert!(store.get(TransactionId::new(1)).unwrap().is_none());
    }

    #[test]
    fn get_reads_back_record_still_in_memory() {
        let db_path = NamedTempFile::new().unwrap();
        let mut store =
            SqliteTransactionStore::new(db_path.path(), Capacity::Elements(10)).unwrap();

        store.upsert(record(1, 42, "1.5")).unwrap();

        let fetched = store.get(TransactionId::new(42)).unwrap().unwrap();
        assert_eq!(fetched.client, ClientId::new(1));
        assert_eq!(
            fetched.amount.as_decimal(),
            "1.5".parse::<Decimal>().unwrap()
        );
        assert!(!fetched.disputed);
    }

    #[test]
    fn upsert_flushes_to_sqlite_once_capacity_is_reached() {
        let db_path = NamedTempFile::new().unwrap();
        let mut store = SqliteTransactionStore::new(db_path.path(), Capacity::Elements(2)).unwrap();

        store.upsert(record(1, 1, "10")).unwrap();
        assert_eq!(store.records.len(), 1);

        // Reaching capacity should flush the cache to SQLite and clear it.
        store.upsert(record(2, 2, "20")).unwrap();
        assert_eq!(store.records.len(), 0);

        // Both records must still be retrievable, now via the DB read-through path.
        let first = store.get(TransactionId::new(1)).unwrap().unwrap();
        assert_eq!(first.amount.as_decimal(), "10".parse::<Decimal>().unwrap());
        let second = store.get(TransactionId::new(2)).unwrap().unwrap();
        assert_eq!(second.amount.as_decimal(), "20".parse::<Decimal>().unwrap());
    }

    #[test]
    fn upsert_after_flush_updates_existing_row_in_sqlite() {
        let db_path = NamedTempFile::new().unwrap();
        let mut store = SqliteTransactionStore::new(db_path.path(), Capacity::Elements(1)).unwrap();

        // capacity of 1 flushes on every upsert.
        store.upsert(record(1, 7, "5")).unwrap();

        let mut disputed = store.get(TransactionId::new(7)).unwrap().unwrap();
        disputed.mark_disputed();
        store.upsert(disputed).unwrap();

        let fetched = store.get(TransactionId::new(7)).unwrap().unwrap();
        assert!(fetched.disputed);
        assert_eq!(fetched.amount.as_decimal(), "5".parse::<Decimal>().unwrap());
    }
}
