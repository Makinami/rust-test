mod model;
mod processor;
mod store;

use clap::Parser;
use model::IncomingTransaction;
use std::process::ExitCode;

use crate::processor::TransactionProcessor;

#[derive(Parser)]
#[command(version, about = "Process transaction records from a CSV file")]
struct Cli {
    /// Path to the transactions CSV file.
    path: String,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let path = cli.path;

    let mut reader = match csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(&path)
    {
        Ok(reader) => reader,
        Err(err) => {
            eprintln!("Failed to open '{path}': {err}");
            return ExitCode::FAILURE;
        }
    };

    let mut processor = TransactionProcessor::new(
        store::InMemoryAccountStore::new(),
        store::InMemoryTransactionStore::new(),
    );

    for transaction in reader.deserialize::<IncomingTransaction>() {
        let transaction = transaction.unwrap();
        processor.process_transaction(transaction);
    }

    let mut writer = csv::Writer::from_writer(std::io::stdout());
    for account in processor.accounts() {
        if let Err(err) = writer.serialize(account) {
            eprintln!("Failed to serialize account: {err}");
            return ExitCode::FAILURE;
        }
    }
    if let Err(err) = writer.flush() {
        eprintln!("Failed to flush output: {err}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
