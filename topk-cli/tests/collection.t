Setup:

  $ export TOPK_API_KEY=QZsw9LAxf47WZJXxoErASBptWqrBQZaFVaeNxYbhskVrY
  $ export TOPK_REGION=emulator TOPK_HOST=ddb TOPK_HTTPS=false
  $ export PATH="$TESTDIR/../.venv/bin:$PATH"
  $ P=t$$
  $ topk collections delete $P-docs --yes 2>/dev/null || true
  $ topk collections create $P-docs --schema '{"title": "text", "year": "int"}' 2>/dev/null

Upsert documents:

  $ topk collection --name $P-docs upsert '[{"_id": "1", "title": "Dune", "year": 1965}, {"_id": "2", "title": "1984", "year": 1949}]' 2>&1
  Upserted 2 document(s) into '*-docs'. (glob)

Get document by ID:

  $ topk --json collection --name $P-docs get 1
  {
    "1": {
      "_id": "1",
      "title": "Dune",
      "year": 1965
    }
  }

Get with --fields:

  $ topk --json collection --name $P-docs get 1 --fields title
  {
    "1": {
      "_id": "1",
      "title": "Dune"
    }
  }

Update (merge):

  $ topk collection --name $P-docs update '[{"_id": "1", "title": "Dune (revised)"}]' 2>&1
  Updated 1 document(s) in '*-docs'. (glob)

Verify update:

  $ topk --json collection --name $P-docs get 1 | python3 -c "import sys,json; print(json.load(sys.stdin)['1']['title'])"
  Dune (revised)

Delete a document:

  $ topk collection --name $P-docs delete 2 2>&1
  Deleted 1 document(s) from '*-docs'. (glob)

Count:

  $ topk --json collection --name $P-docs count
  \d+ (re)

Count with --consistency:

  $ topk --json collection --name $P-docs count --consistency indexed
  \d+ (re)

JSON output for upsert returns LSN:

  $ topk --json collection --name $P-docs upsert '[{"_id": "3", "title": "test"}]'
  ".+" (re)

JSON output for delete:

  $ topk --json collection --name $P-docs delete 3
  ".+" (re)

Stdin input with -:

  $ echo '[{"_id": "4", "title": "stdin test"}]' | topk collection --name $P-docs upsert - 2>&1
  Upserted 1 document(s) into '*-docs'. (glob)

File input with @:

  $ echo '[{"_id": "5", "title": "file test"}]' > "$CRAMTMP/docs.json"
  $ topk collection --name $P-docs upsert "@$CRAMTMP/docs.json" 2>&1
  Upserted 1 document(s) into '*-docs'. (glob)

Cleanup:

  $ topk collections delete $P-docs --yes 2>&1
  Collection '*-docs' deleted. (glob)
