Setup — upload a document for ask:

  $ export TOPK_API_KEY=QZsw9LAxf47WZJXxoErASBptWqrBQZaFVaeNxYbhskVrY
  $ export TOPK_REGION=emulator TOPK_HOST=ddb TOPK_HTTPS=false
  $ export PATH="$TESTDIR/../.venv/bin:$PATH"
  $ P=t$$
  $ topk datasets create $P-ask 2>/dev/null
  $ cat > "$CRAMTMP/doc.md" << 'EOF'
  > # TopK Overview
  > TopK is a real-time AI-native database for building search and retrieval applications.
  > It supports vector search, full-text search, and structured filtering.
  > EOF
  $ topk dataset --name $P-ask upsert-file doc-1 --file "$CRAMTMP/doc.md" --metadata '{"source": "test"}' --wait --timeout 60 >/dev/null 2>&1

Ask returns an answer:

  $ topk --json ask "What does TopK support?" -s $P-ask | python3 -c "import sys,json; d=json.load(sys.stdin); print(type(d).__name__)"
  dict

Cleanup:

  $ topk datasets delete $P-ask --yes 2>/dev/null
