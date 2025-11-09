# bench

## ingest

- topk-rs (eu-central-1)
  - ~62MB/s
  - `-b 2000 -c 8`
  - `s3://jergu-test/results/topk-bench-2025-11-09-00-03-13-603d5eb1.parquet`
- topk-py (eu-central-1)
  - ~40MB/s
  - `-b 2000 -c 8`
  - `s3://jergu-test/results/topk-bench-2025-11-09-00-02-40-a253369e.parquet`
- tpuf-py (eu-central-1)
  - ~11MB/s
  - `-b 2000 -c 8`
  - `s3://jergu-test/results/topk-bench-2025-11-09-00-06-04-33a0382d.parquet`

## query
