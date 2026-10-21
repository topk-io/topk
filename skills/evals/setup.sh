#!/usr/bin/env bash
# Build a fresh working directory for one eval: ./setup.sh <eval-id> <dir>
# Fixtures for e1/e3 are copied from this repo's docs so they stay realistic.
set -euo pipefail
id="$1"; dir="$2"
root="$(cd "$(dirname "$0")/../.." && pwd)"
mkdir -p "$dir"
case "$id" in
  docs-hybrid-search-python)
    mkdir -p "$dir/docs"
    for f in introduction architecture concepts limits regions; do cp "$root/docs/$f.mdx" "$dir/docs/$f.md"; done
    for f in "$root"/docs/collections/*.mdx "$root"/docs/guides/*.mdx; do
      cp "$f" "$dir/docs/$(basename "$(dirname "$f")")-$(basename "${f%.mdx}").md"
    done ;;
  product-search-typescript)
    cp "$(dirname "$0")/fixtures/products.json" "$dir/" ;;
  multi-tenant-rag-python)
    mkdir -p "$dir/tenants/acme" "$dir/tenants/globex"
    cp "$root/docs/guides/semantic-search.mdx" "$dir/tenants/acme/semantic-search.md"
    cp "$root/docs/guides/keyword-search.mdx" "$dir/tenants/acme/keyword-search.md"
    cp "$root/docs/limits.mdx" "$dir/tenants/acme/limits.md"
    cp "$root/docs/collections/write.mdx" "$dir/tenants/globex/write.md"
    cp "$root/docs/concepts.mdx" "$dir/tenants/globex/concepts.md"
    cp "$root/docs/regions.mdx" "$dir/tenants/globex/regions.md" ;;
  *) echo "unknown eval id: $id" >&2; exit 1 ;;
esac
echo "ready: $dir"
