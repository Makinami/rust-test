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

// Convenience constructors for test purposes. We can remove cfg(test) predicate if we want these available in non-test code as well.
impl IncomingTransaction {
    #[cfg(test)]
    pub fn deposit(client: ClientId, tx: TransactionId, amount: Amount) -> Self {
        IncomingTransaction::Deposit { client, tx, amount }
    }

    #[cfg(test)]
    pub fn withdrawal(client: ClientId, tx: TransactionId, amount: Amount) -> Self {
        IncomingTransaction::Withdrawal { client, tx, amount }
    }

    #[cfg(test)]
    pub fn dispute(client: ClientId, tx: TransactionId) -> Self {
        IncomingTransaction::Dispute { client, tx }
    }

    #[cfg(test)]
    pub fn resolve(client: ClientId, tx: TransactionId) -> Self {
        IncomingTransaction::Resolve { client, tx }
    }

    #[cfg(test)]
    pub fn chargeback(client: ClientId, tx: TransactionId) -> Self {
        IncomingTransaction::Chargeback { client, tx }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_deposit() {
        let csv_data = "type,client,tx,amount\n\
                        deposit,1,1,100.0";
        let mut rdr = csv::Reader::from_reader(csv_data.as_bytes());
        let record: IncomingTransaction = rdr.deserialize().next().unwrap().unwrap();
        match record {
            IncomingTransaction::Deposit { client, tx, amount } => {
                assert_eq!(client, ClientId::new(1));
                assert_eq!(tx, TransactionId::new(1));
                assert_eq!(amount, 100u32.into());
            }
            _ => panic!("Expected Deposit variant"),
        }
    }

    #[test]
    fn test_deserialize_withdrawal() {
        let csv_data = "type,client,tx,amount\n\
                        withdrawal,1,1,50.0";
        let mut rdr = csv::Reader::from_reader(csv_data.as_bytes());
        let record: IncomingTransaction = rdr.deserialize().next().unwrap().unwrap();
        match record {
            IncomingTransaction::Withdrawal { client, tx, amount } => {
                assert_eq!(client, ClientId::new(1));
                assert_eq!(tx, TransactionId::new(1));
                assert_eq!(amount, 50u32.into());
            }
            _ => panic!("Expected Withdrawal variant"),
        }
    }

    #[test]
    fn test_deserialize_dispute() {
        let csv_data = "type,client,tx,amount\n\
                        dispute,1,1,";
        let mut rdr = csv::Reader::from_reader(csv_data.as_bytes());
        let record: IncomingTransaction = rdr.deserialize().next().unwrap().unwrap();
        match record {
            IncomingTransaction::Dispute { client, tx } => {
                assert_eq!(client, ClientId::new(1));
                assert_eq!(tx, TransactionId::new(1));
            }
            _ => panic!("Expected Dispute variant"),
        }
    }

    #[test]
    fn test_deserialize_resolve() {
        let csv_data = "type,client,tx,amount\n\
                        resolve,1,1,";
        let mut rdr = csv::Reader::from_reader(csv_data.as_bytes());
        let record: IncomingTransaction = rdr.deserialize().next().unwrap().unwrap();
        match record {
            IncomingTransaction::Resolve { client, tx } => {
                assert_eq!(client, ClientId::new(1));
                assert_eq!(tx, TransactionId::new(1));
            }
            _ => panic!("Expected Resolve variant"),
        }
    }

    #[test]
    fn test_deserialize_chargeback() {
        let csv_data = "type,client,tx,amount\n\
                        chargeback,1,1,";
        let mut rdr = csv::Reader::from_reader(csv_data.as_bytes());
        let record: IncomingTransaction = rdr.deserialize().next().unwrap().unwrap();
        match record {
            IncomingTransaction::Chargeback { client, tx } => {
                assert_eq!(client, ClientId::new(1));
                assert_eq!(tx, TransactionId::new(1));
            }
            _ => panic!("Expected Chargeback variant"),
        }
    }
}
