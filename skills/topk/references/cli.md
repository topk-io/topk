# TopK CLI reference

What an agent needs to run the `topk` CLI without surprises. For every flag, source and option, see the full reference at https://docs.topk.io/cli (as markdown: https://docs.topk.io/cli.md), or run `topk <command> --help`, which always matches the installed version.

## Contents
- Log in and select a project
- Inspect and run SQL
- Import (workflow, fields, sources, copy a collection, existing collections, failures and resuming)
- Troubleshooting

## Log in and select a project

1. Tell the user: "A browser window will open. Sign up or log in to TopK there, then come back here."
2. Run `topk login`. It waits until the user finishes, for up to 5 minutes, so run it in the background or with a timeout longer than 5 minutes. On success it prints a line starting with `✓ Logged in` to stderr. If no browser can open (SSH, a container, no display), it prints a URL instead and waits for a code; give the user the URL, and after they log in, the page shows the code, which goes into the command's prompt. When the user's browser is on another machine but this one could open a browser, pass `--no-browser` to get the URL.
3. Select a project:

   ```bash
   topk project list -o json          # one line per project: project_id, name, selected, collection_count
   topk project select <PROJECT_ID>
   ```

   If a project has `"selected": true`, keep it. Otherwise select the only project, or ask the user which one to use. If there are none, run `topk project create "<name>" -o json` and select the new one.
4. Check the region with `topk region current`. If one is selected, keep it. Otherwise `topk region` lists the available ones; select one with `topk region select <REGION>`, using `aws-us-east-1-elastica` if it's listed and the user has no preference.

`--project-id` and `--region` override the selection for one command. Environment variables you `export` may not survive between your commands, so rely on the selection or pass the flags.

## Inspect and run SQL

```bash
topk sql "SHOW TABLES STARTS WITH 'books'"
topk sql -o json "SELECT _id, title FROM books LIMIT 5"
topk sql -f - <<'SQL'
CREATE TABLE IF NOT EXISTS books (title TEXT NOT NULL INDEX keyword_index());
SQL
```

Statements separated by `;` run in order; the first error stops the rest. With `-o json`, each row is one JSON object per line. A failed statement prints `error: …` and exits 1. For the SQL itself, see [sql.md](sql.md).

## Import

`topk import` reads one source (a file, a glob, a table, an index or a collection) into one collection, named with `--to`, and creates the collection with the first write. Full options: https://docs.topk.io/cli#import.

### Workflow

```bash
topk import ./articles.jsonl --to articles --dry-run                                  # 1. fields, discovered types, sample rows
topk import ./articles.jsonl --to articles -f title=keyword,body=semantic --dry-run   # 2. check the plan and warnings
topk import ./articles.jsonl --to articles -f title=keyword,body=semantic --yes       # 3. import
topk sql "SELECT COUNT(*) FROM articles"                                              # 4. matches the source's row count
```

Always pass `--yes` to a real import: without a terminal, the confirmation prompt fails. `--dry-run` needs a login and a project, because it checks the plan against the destination.

### Fields

- **Every field is imported, and only fields given an index with `-f` are searchable.** Without `-f`, the data loads but nothing can be searched.
- Each `-f` entry is `name=choice`, comma-separated or repeated: an index (`keyword`, `semantic`, `ngram`), a type (`int`, `timestamp`, `text_list`, …), both (`image:semantic`), or a vector (`vector:768:cosine`). `-f title` alone is an error. `--exclude notes` leaves a field out; `--rename raw_body=body` renames first.
- **Text over 32,768 characters in a `semantic` field is cut off, not rejected** (SKILL.md rule 1). Chunk long text first, or use `keyword`, which has no such limit.
- A float list imports as `float_list`, which can't be searched; `-f embedding=vector:<dim>:cosine` makes it a vector. A conversion that isn't exact fails the row.
- Images: `-f page=image:semantic` for JPEG/PNG bytes or data URIs, at most 512KB each.

### Sources

| Source | Example |
|---|---|
| Files | `topk import ./books.parquet --to books` (`.csv`, `.tsv`, `.json`, `.jsonl`, `.arrow`, `.avro`) |
| Globs and buckets | `topk import 's3://bucket/books/**/*.parquet' --to books` (`gs://`, `az://`, `r2://`, `hf://`, `http(s)://`) |
| Postgres | `topk import postgres://user@host/db#public.orders --to orders` (password in the URL or `PGPASSWORD`) |
| MySQL | `topk import mysql://root@host/shop#orders --to orders` (`MYSQL_PWD`) |
| SQLite | `topk import sqlite:./app.db#orders --to orders` |
| MongoDB | `topk import mongodb://host/shop#products --to products` |
| Elasticsearch | `topk import es+https://user@host/products --to products` (`ELASTIC_API_KEY` or `ELASTIC_PASSWORD`) |
| TopK | `topk import topk://books --to books_v2` |

One run imports one table or index, so import several with one command each. `--filter` takes SQL `WHERE` for files and databases, a JSON query for MongoDB and Elasticsearch; `--limit` reads a sample. `--partition tenant` writes each row to the partition named by its `tenant` value (SKILL.md, Multi-tenancy). In a glob of data files, a `{tenant}` folder becomes a column (`'tenants/{tenant}/*.jsonl'`); for documents such as Markdown or PDF, use `scripts/chunk.py --partition-from-dir`.

### Copy a collection

```bash
topk import topk://books --to books_v2 -f title=keyword,plot=semantic --dry-run
```

A copy keeps the source's fields and indexes unless `-f` changes them, and adds to whatever the target holds. To change a schema, copy into a new name; don't drop and recreate under the same name (SKILL.md rule 7). `topk://<PROJECT_ID>/<collection>` reads from another project, and `--region` picks the destination region. **A collection with partitions can't be copied.**

### Existing collections

An import into an existing collection adds to it and replaces documents with the same `_id` (SKILL.md rule 11: ask first). Its declared fields keep their types and indexes, and there's no schema migration: a `-f` choice that differs, or an index on a field it doesn't declare, fails the plan with `the collection's schema is set when it is created and cannot change`. `--exclude` on a field the collection has clears it on the documents written.

### Failures and resuming

- The first row that fails (a value that doesn't convert, a missing id, a document over 2MB) stops the run, which prints `Skip rows that fail and continue with: topk import --resume <RUN> --continue-on-error`.
- With `--continue-on-error`, failed rows are skipped and listed in the `<run>.rejected.jsonl` file the summary names. **The command still exits 0**, so read the summary's `<M> failed` count.
- A stopped run (an error, `^C`, a lost connection) prints `Continue with: topk import --resume <RUN>`. Run it as printed; it doesn't ask for confirmation. A re-run without `--resume` imports everything again, which is safe because upserts are idempotent.
- `-c <N>` sets concurrent upserts (default 16).

## Troubleshooting

CLI and import errors. SQL errors are covered by the rules in SKILL.md, quota errors by its Plan limits.

| Error or symptom | Fix |
|---|---|
| `not logged in` or `session expired` | Run `topk login` again |
| `--project-id (or TOPK_PROJECT_ID environment variable) is required` | `topk project select <PROJECT_ID>`, or pass `--project-id`. Also printed before a first login: check `topk project list` |
| `--region (or TOPK_REGION environment variable) is required`, or `no region selected` | `topk region select <REGION>`, or pass `--region` |
| `Table does not exist` | Check the name with `topk sql "SHOW TABLES"`, and the project and region |
| `couldn't detect an id column` | Pass `--id <column>` |
| Import finished but search returns nothing | No field had an index: it ran without `-f`. Copy into a new name with `-f <field>=semantic,…` |
| `the collection's schema is set when it is created and cannot change` | The lines after it name each field. Drop that `-f` choice, or import into a new collection name |
| `-f takes <field>=<type or index>` | Give the field a choice (`title=keyword`), drop it from `-f`, or leave it out with `--exclude` |
| `required arguments were not provided: --to <COLLECTION>` | Name the target collection with `--to` |
| `source requires #<table>` | Add the table or collection to the URI: `sqlite:./app.db#orders` |
| `field "x" was not found` | Check the source's field names in the `--dry-run` preview without `-f`; rename with `--rename` |
| `brew: command not found` | Tell the user the TopK CLI installs with Homebrew. With `TOPK_API_KEY` set, the SDK can load and query instead (SKILL.md, Setup) |
| Homebrew says the formula or tap isn't trusted | Install by its full name, `brew install topk-io/topk/topk`, which trusts it; or `brew trust --formula topk-io/topk/topk` |
