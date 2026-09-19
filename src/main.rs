use std::fs::File;
use std::io;

use clap::Parser;
use log::error;
use rust_test::model::IncomingTransaction;
use rust_test::processor::TransactionProcessor;
use rust_test::store;
use tokio::select;
use tokio::sync::mpsc::{Sender, channel};
use tokio::task::JoinSet;

#[derive(Parser)]
#[command(version, about = "Process transaction records from CSV files")]
struct Cli {
    /// Paths to the transactions CSV files.
    #[clap(required = true)]
    paths: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Parse command-line arguments
    let cli = Cli::parse();

    // Prepare the processor
    let db_file = tempfile::NamedTempFile::new()?;
    let mut processor = TransactionProcessor::new(
        store::InMemoryAccountStore::new(),
        store::SqliteTransactionStore::new(db_file.path(), store::Capacity::Megabytes(100))
            .inspect_err(|err| error!("Failed to open the SQLite store: {err}"))?,
    );

    // NOTE: Size of bounded channel was chosen completely arbitrarily.
    // Consider the appropriate size based on expected workload and memory constraints, or switch to an unbounded channel if necessary.
    let (tx, mut rx) = channel::<IncomingTransaction>(100);

    // Spawn parser tasks for each CSV file
    let mut parser_tasks = spawn_parser_tasks(&cli.paths, tx)?;

    // Main event loop: process incoming transactions and handle parser task completions
    loop {
        select! {
            // Handle completion of parser tasks. If a parser task returns an error, log it and exit the program.
            Some(res) = parser_tasks.join_next() => {
                let res = res.unwrap(); // Don't handle JoinError for simplicity
                res.inspect_err(|err| error!("Error in data deserialization: {err}"))?;
            }
            // Handle incoming transactions from the channel.
            Some(transaction) = rx.recv() => {
                processor
                    .process_transaction(transaction)
                    .inspect_err(|err| error!("Critical error when processing transaction: {err}"))?;
            }
            else => break,
        }
    }

    // Serialize and output the final state of all accounts to stdout.
    serialize_accounts(processor.accounts(), std::io::stdout())?;

    Ok(())
}

fn serialize_accounts<'a>(
    accounts: impl Iterator<Item = &'a rust_test::model::Account>,
    writer: impl io::Write,
) -> Result<(), io::Error> {
    let mut writer = csv::Writer::from_writer(writer);
    for account in accounts {
        writer
            .serialize(account)
            .inspect_err(|err| error!("Failed to serialize an account: {err}"))?;
    }
    Ok(())
}

fn spawn_parser_tasks(
    paths: &[String],
    tx: Sender<IncomingTransaction>,
) -> Result<JoinSet<Result<(), io::Error>>, io::Error> {
    let mut tasks = JoinSet::new();
    for path in paths {
        let file = File::open(path).inspect_err(|_| error!("Failed to open the file: {path}"))?;
        let tx_clone = tx.clone();
        tasks.spawn(async move { deserialize_and_send(file, tx_clone).await });
    }
    Ok(tasks)
}

async fn deserialize_and_send(
    reader: impl io::Read,
    tx: Sender<IncomingTransaction>,
) -> Result<(), io::Error> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(reader);

    for transaction in csv_reader.deserialize::<IncomingTransaction>() {
        tx.send(transaction?).await.map_err(io::Error::other)?;
    }
    Ok(())
}
