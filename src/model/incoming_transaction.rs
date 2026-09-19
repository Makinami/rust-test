use crate::model::{Amount, ClientId, TransactionId};
use serde::Deserialize;
use serde::de::{self, Deserializer};

/// A single transaction record, shaped by its `type` column.
#[derive(Debug)]
pub enum IncomingTransaction {
    Deposit {
        client: ClientId,
        tx: TransactionId,
        amount: Amount,
    },
    Withdrawal {
        client: ClientId,
        tx: TransactionId,
        amount: Amount,
    },
    Dispute {
        client: ClientId,
        tx: TransactionId,
    },
    Resolve {
        client: ClientId,
        tx: TransactionId,
    },
    Chargeback {
        client: ClientId,
        tx: TransactionId,
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
    tx: TransactionId,
    #[serde(default, deserialize_with = "csv::invalid_option")]
    amount: Option<Amount>,
}

impl<'de> Deserialize<'de> for IncomingTransaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawRecord::deserialize(deserializer)?;

        let with_amount = |make: fn(ClientId, TransactionId, Amount) -> IncomingTransaction| {
            raw.amount
                .map(|amount| make(raw.client, raw.tx, amount))
                .ok_or_else(|| de::Error::missing_field("amount"))
        };

        match raw.kind.as_str() {
            "deposit" => with_amount(|client, tx, amount| IncomingTransaction::Deposit {
                client,
                tx,
                amount,
            }),
            "withdrawal" => with_amount(|client, tx, amount| IncomingTransaction::Withdrawal {
                client,
                tx,
                amount,
            }),
            "dispute" => Ok(IncomingTransaction::Dispute {
                client: raw.client,
                tx: raw.tx,
            }),
            "resolve" => Ok(IncomingTransaction::Resolve {
                client: raw.client,
                tx: raw.tx,
            }),
            "chargeback" => Ok(IncomingTransaction::Chargeback {
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
