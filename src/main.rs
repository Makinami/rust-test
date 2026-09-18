mod model;

use model::Transaction;
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("Usage: rust-test <transactions.csv>");
        return ExitCode::FAILURE;
    };

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

    for result in reader.deserialize::<Transaction>() {
        match result {
            Ok(transaction) => println!("{transaction:?}"),
            Err(err) => eprintln!("Skipping invalid row: {err}"),
        }
    }

    ExitCode::SUCCESS
}
