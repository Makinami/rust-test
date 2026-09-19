use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use rust_decimal::Decimal;
use rust_test::model::{Amount, ClientId, DepositRecord, TransactionId};
use rust_test::store::{
    Capacity, InMemoryTransactionStore, SqliteTransactionStore, TransactionStore,
};

/// Record counts chosen to simulate small, medium, and large real-world data sets.
const SIZES: [u64; 2] = [100, 10_000];

/// SQLite capacity used for benchmarking: small enough that bigger of the data sets will trigger a flush-to-disk cycle.
/// NOTE: Avoid SIZES being CAPACITY multiples to avoid predictable but pathological benchmarking patterns.
const SQLITE_CAPACITY: usize = 1_500;

fn make_record(i: u64) -> DepositRecord {
    let client = ClientId::new((i % 1_000) as u16);
    let tx = TransactionId::new(i as u32);
    let amount = Amount::try_from(Decimal::new(12345, 4)).unwrap(); // 1.2345
    DepositRecord::new(client, tx, amount)
}

fn bench_upsert(c: &mut Criterion) {
    let mut group = c.benchmark_group("upsert");

    for &size in &SIZES {
        // Report the per-record (amortized) cost rather than the total batch
        // time, since `size` records are upserted per iteration.
        group.throughput(Throughput::Elements(size));

        group.bench_with_input(BenchmarkId::new("in_memory", size), &size, |b, &size| {
            b.iter_batched(
                InMemoryTransactionStore::new,
                |mut store| {
                    for i in 0..size {
                        store.upsert(black_box(make_record(i))).unwrap();
                    }
                    store
                },
                BatchSize::LargeInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("sqlite", size), &size, |b, &size| {
            let db_path = tempfile::NamedTempFile::new().unwrap();
            b.iter_batched(
                || {
                    SqliteTransactionStore::new(db_path.path(), Capacity::Elements(SQLITE_CAPACITY))
                        .unwrap()
                },
                |mut store| {
                    for i in 0..size {
                        store.upsert(black_box(make_record(i))).unwrap();
                    }
                    store
                },
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

fn bench_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("get");

    for &size in &SIZES {
        // A sample of transaction ids spread across the whole range, so the
        // sqlite store's benchmark exercises both in-memory-cache hits (most
        // recently inserted, not yet flushed) and reads that fall through to
        // disk (flushed earlier due to reaching SQLITE_CAPACITY).
        let step = (size / 100).max(1);
        let sample_txs: Vec<TransactionId> = (0..size)
            .step_by(step as usize)
            .map(|i| TransactionId::new(i as u32))
            .collect();

        // Report the per-lookup (amortized) cost rather than the total batch
        // time, since `sample_txs.len()` lookups are performed per iteration.
        group.throughput(Throughput::Elements(sample_txs.len() as u64));

        let mut in_memory_store = InMemoryTransactionStore::new();
        for i in 0..size {
            in_memory_store.upsert(make_record(i)).unwrap();
        }

        group.bench_with_input(
            BenchmarkId::new("in_memory", size),
            &sample_txs,
            |b, sample_txs| {
                b.iter(|| {
                    for &tx in sample_txs {
                        black_box(in_memory_store.get(tx).unwrap());
                    }
                });
            },
        );

        let db_path = tempfile::NamedTempFile::new().unwrap();
        let mut sqlite_store =
            SqliteTransactionStore::new(db_path.path(), Capacity::Elements(SQLITE_CAPACITY)).unwrap();
        for i in 0..size {
            sqlite_store.upsert(make_record(i)).unwrap();
        }

        group.bench_with_input(
            BenchmarkId::new("sqlite", size),
            &sample_txs,
            |b, sample_txs| {
                b.iter(|| {
                    for &tx in sample_txs {
                        black_box(sqlite_store.get(tx).unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_upsert, bench_get);
criterion_main!(benches);
