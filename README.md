# Transaction processor

This repository is a CLI program that reads a history of transactions from one or more CSV files, and outputs final accounts balance to stdout in CSV format.

## Possibly unclear assumptions

### Negative balance

Available balance can become negative. This can happen when dispute about deposit happens after the withdrawal.
This represents an account where user essentially must make a deposit to continue to use it.

### Maximum account balance

The program supports account balances of up to ~7e24. This is due to using [Decimal](https://docs.rs/rust_decimal/latest/rust_decimal/struct.Decimal.html) as an underlying type.
The overflow is detected and program exits with an error.

Should a support for higher balances be necessary, only Amount would need to be modified (as explained very briefly in src\model\amount.rs).

> [!WARNING]
> While overflow is properly checked during runtime, there are some edge cases of program "correctly" deserializing numbers that Decimal cannot hold without loosing precision.
> Some more work around deserialization is still needed.

### Withdrawals cannot be disputed

Currently only disputes against deposits are processed. Disputes against withdrawals are ignored.

When thinking of disputed I've had a mental model of the Seller's account on PayPal and was considering the following case:
1. Buyer makes an order an pays to the Seller (Deposit into Seller's account)
2. After a while, an item does not come or comes broken. Buyer opens a Dispute through PayPal. (available fund moved to held)
3. If the dispute is resolved, funds are available again; in case of chargeback, funds are removed and account locked.

I had trouble creating a similar story for disputing withdrawals.

#### What-if: disputing withdrawals

Algorithmically speaking, disputing withdrawals might behave along these lines:

For a withdrawal of A amount
- dispute  
  held += A
- resolve?
  held -= A
  available += A
- chargeback?
  held -= A
  lock account?

## Technicalities

### Correctness

The correctness is assured through the combination of type system modeling, careful ownership, localized algorithms and unit tests. I have also created a series of test input data in test_data directory.

### Error handling

Errors in critical parts of application (transaction processing) and modeled with custom enums, returned through Result and handled gracefully. In case of problems that we ignore (e.g. insufficient funds for a withdrawal), a log is outputted but execution continues.

In cases of errors that might potentially cause incorrect calculation or ones that come from broadly speaking "I/O", the error is logged and program stops.

### Database usage

Because transaction ID is a u32 number, for large enough input data we could run out of memory if we tried to keep even just the deposits in memory. For this reason runtime transaction store is backed by the SQLite database. However, inserting/selecting data from database for each row would be extremely slow, so I'm using a hybrid approach.

Up to ~100MB (hardcoded in main) of transaction data is held in memory. Once this limit is hit, all 100MB of records are moved to the DB (disk file) and data in memory is purged. When reading the records, first in memory hashmap in checked and DB is accessed only if the transaction is not in memory any more.

There are more elaborate (and often better) ways to move data between memory and disk, but for now this should be enough to avoid out-of-memory errors.

### Main() complexity explanation

As a simple CLI application current main() structure of spawning and synchronizing multiple tokio task is probably an overkill. This is done only to hint towards one possible implementation (using tokio's mpsc channel) in case such processor would need to be span as e.g. a server and it would accepts many CSV concurrently.