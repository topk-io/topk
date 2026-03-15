Setup — upload a document for search:

  $ export TOPK_API_KEY=QZsw9LAxf47WZJXxoErASBptWqrBQZaFVaeNxYbhskVrY
  $ export TOPK_REGION=emulator TOPK_HOST=ddb TOPK_HTTPS=false
  $ export PATH="$TESTDIR/../.venv/bin:$PATH"
  $ P=t$$
  $ topk datasets create $P-search 2>/dev/null
  $ cat > "$CRAMTMP/doc.md" << 'EOF'
  > # TopK Overview
  > TopK is a real-time AI-native database for building search and retrieval applications.
  > It supports vector search, full-text search, and structured filtering.
  > EOF
  $ topk dataset --name $P-search upsert-file doc-1 --file "$CRAMTMP/doc.md" --metadata '{"source": "test"}' --wait --timeout 60 >/dev/null 2>&1

Search returns results:

  $ topk search "vector search" -s $P-search 2>/dev/null | head -1
  DATASET* (glob)

Search with --json and --top-k:

  $ topk --json search "vector search" -s $P-search --top-k 1 | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d))"
  1

Cleanup:

  $ topk datasets delete $P-search --yes 2>/dev/null
