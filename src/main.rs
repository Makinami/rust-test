use clap::Parser;
use log::error;
use rust_test::model::IncomingTransaction;
use rust_test::processor::TransactionProcessor;
use rust_test::store;

#[derive(Parser)]
#[command(version, about = "Process transaction records from a CSV file")]
struct Cli {
    /// Path to the transactions CSV file.
    path: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let cli = Cli::parse();
    let path = cli.path;

    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(&path)
        .inspect_err(|_| error!("Failed to open the file: {path}"))?;

    let db_file = tempfile::NamedTempFile::new()?;

    let mut processor = TransactionProcessor::new(
        store::InMemoryAccountStore::new(),
        store::SqliteTransactionStore::new(db_file.path(), store::Capacity::Megabytes(100))
            .inspect_err(|err| error!("Failed to open the SQLite store: {err}"))?,
    );

    for transaction in reader.deserialize::<IncomingTransaction>() {
        let transaction = transaction.unwrap();
        processor
            .process_transaction(transaction)
            .inspect_err(|err| error!("Critical error when processing transaction: {err}"))?;
    }

    let mut writer = csv::Writer::from_writer(std::io::stdout());
    for account in processor.accounts() {
        writer
            .serialize(account)
            .inspect_err(|err| error!("Failed to serialize an account: {err}"))?;
    }

    Ok(())
}
