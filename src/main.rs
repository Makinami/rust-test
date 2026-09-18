mod model;
mod processor;
mod store;

use clap::Parser;
use model::IncomingTransaction;

use crate::processor::TransactionProcessor;

#[derive(Parser)]
#[command(version, about = "Process transaction records from a CSV file")]
struct Cli {
    /// Path to the transactions CSV file.
    path: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let path = cli.path;

    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(&path)?;

    let mut processor = TransactionProcessor::new(
        store::InMemoryAccountStore::new(),
        store::InMemoryTransactionStore::new(),
    );

    for transaction in reader.deserialize::<IncomingTransaction>() {
        let transaction = transaction.unwrap();
        if let Err(err) = processor.process_transaction(transaction) {
            eprintln!("Critical error when processing transaction: {err}");
            return Err(err);
        }
    }

    let mut writer = csv::Writer::from_writer(std::io::stdout());
    for account in processor.accounts() {
        writer.serialize(account)?;
    }

    Ok(())
}
