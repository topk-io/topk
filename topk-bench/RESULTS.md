# bench

## ingest

- topk-rs
  - ~62MB/s
  - `-b 8000 -c 4`
- topk-py
  - ~40MB/s
  - `-b 2000 -c 8`
- tpuf-py
  - ~13MB/s
  - `-b 4000 -c 4`
  - `s3://jergu-test/results/topk-bench-2025-11-08-22-26-08-c7918e22.parquet`

## query
