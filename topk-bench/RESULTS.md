# bench

## ingest

- topk-rs (eu-central-1; `-b 2000 -c 8`; ~60MB/s)
  - 100k; ~9s total; `s3://jergu-test/results/topk-bench-2025-11-10-14-19-24-0f5cf93a.parquet`
  - 1m; ~56s total; `s3://jergu-test/results/topk-bench-2025-11-10-14-21-01-a0a48367.parquet`
- topk-py (eu-central-1; `-b 2000 -c 8`; ~40MB/s)
  - 100k; ~11s total; `s3://jergu-test/results/topk-bench-2025-11-10-09-17-32-8f48ef2e.parquet`
  - 1m; ~90s total; `s3://jergu-test/results/topk-bench-2025-11-10-09-26-00-fcb33b6d.parquet`
- tpuf-py (eu-central-1; `-b 2000 -c 8`; ~12MB/s)
  - 100k; ~30s total; `s3://jergu-test/results/topk-bench-2025-11-10-12-43-50-e82da6a5.parquet`
  - 1m; ~280s total; `s3://jergu-test/results/topk-bench-2025-11-10-12-49-00-0ec3fd0a.parquet`
- chroma (us-east-1; `-b 300 -c 8`; ~10MB/s)
  - 100k; ~45s total; `s3://jergu-test/results/topk-bench-2025-11-10-14-27-54-afe57bb5.parquet`
  - 1m; ~390s total; `s3://jergu-test/results/topk-bench-2025-11-10-13-31-38-86f2817e.parquet`

## query
