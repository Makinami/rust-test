mod amount;

pub use amount::Amount;

use serde::de::{self, Deserializer};
use serde::Deserialize;

/// New Type wrapper around a client identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct ClientId(pub u16);

/// New Type wrapper around a transaction identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct TxId(pub u32);

/// A single transaction record, shaped by its `type` column.
#[derive(Debug)]
pub enum Transaction {
    Deposit {
        client: ClientId,
        tx: TxId,
        amount: Amount,
    },
    Withdrawal {
        client: ClientId,
        tx: TxId,
        amount: Amount,
    },
    Dispute {
        client: ClientId,
        tx: TxId,
    },
    Resolve {
        client: ClientId,
        tx: TxId,
    },
    Chargeback {
        client: ClientId,
        tx: TxId,
    },
}

/// Flat, positional representation of a CSV row, matching the raw columns.
///
/// CSV rows aren't self-describing the way JSON objects are, so we can't rely
/// on serde's built-in enum representations to pick a `Transaction` variant.
/// Instead we deserialize into this intermediate struct and then dispatch on
/// `type` ourselves in `Transaction`'s `Deserialize` impl below.
#[derive(Debug, Deserialize)]
struct RawRecord {
    #[serde(rename = "type")]
    kind: String,
    client: ClientId,
    tx: TxId,
    #[serde(default, deserialize_with = "csv::invalid_option")]
    amount: Option<Amount>,
}

impl<'de> Deserialize<'de> for Transaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawRecord::deserialize(deserializer)?;

        let with_amount = |make: fn(ClientId, TxId, Amount) -> Transaction| {
            raw.amount
                .map(|amount| make(raw.client, raw.tx, amount))
                .ok_or_else(|| de::Error::missing_field("amount"))
        };

        match raw.kind.as_str() {
            "deposit" => with_amount(|client, tx, amount| Transaction::Deposit {
                client,
                tx,
                amount,
            }),
            "withdrawal" => with_amount(|client, tx, amount| Transaction::Withdrawal {
                client,
                tx,
                amount,
            }),
            "dispute" => Ok(Transaction::Dispute {
                client: raw.client,
                tx: raw.tx,
            }),
            "resolve" => Ok(Transaction::Resolve {
                client: raw.client,
                tx: raw.tx,
            }),
            "chargeback" => Ok(Transaction::Chargeback {
                client: raw.client,
                tx: raw.tx,
            }),
            other => Err(de::Error::unknown_variant(
                other,
                &["deposit", "withdrawal", "dispute", "resolve", "chargeback"],
            )),
        }
    }
}
