# bench

## ingest

- topk-rs (eu-central-1; `-b 2000 -c 8`; ~60MB/s)
  - 100k; ~9s total; `s3://jergu-test/results/topk-bench-2025-11-10-14-19-24-0f5cf93a.parquet`
  - 1m; ~56s total; `s3://jergu-test/results/topk-bench-2025-11-10-14-21-01-a0a48367.parquet`
- topk-py (eu-central-1; `-b 2000 -c 8`; ~40MB/s)
  - 100k; ~11s total; `s3://jergu-test/results/topk-bench-2025-11-10-09-17-32-8f48ef2e.parquet`
  - 1m; ~90s total; `s3://jergu-test/results/topk-bench-2025-11-10-09-26-00-fcb33b6d.parquet`
  - 10m; ~760 total; `s3://jergu-test/results/topk-bench-2025-11-12-00-13-33-695156cc.parquet`
- tpuf-py (eu-central-1; `-b 2000 -c 8`; ~12MB/s)
  - 100k; ~30s total; `s3://jergu-test/results/topk-bench-2025-11-10-12-43-50-e82da6a5.parquet`
  - 1m; ~280s total; `s3://jergu-test/results/topk-bench-2025-11-10-12-49-00-0ec3fd0a.parquet`
- chroma (us-east-1; `-b 300 -c 8`; ~10MB/s)
  - 100k; ~45s total; `s3://jergu-test/results/topk-bench-2025-11-10-14-27-54-afe57bb5.parquet`
  - 1m; ~390s total; `s3://jergu-test/results/topk-bench-2025-11-10-13-31-38-86f2817e.parquet`
- milvus (eu-central-1; `-b 8000 -c 4`; ~15MB/s)
  - 100k; ~28s total; `s3://jergu-test/results/topk-bench-2025-11-11-17-55-09-4cc332a0.parquet`
  - 1m; ~230s total; `s3://jergu-test/results/topk-bench-2025-11-11-17-59-46-32891c8e.parquet`

## query
