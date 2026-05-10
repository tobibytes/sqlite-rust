# sqlite-rust

A SQLite reader implemented from scratch in Rust — no external SQL libraries. Parses the raw SQLite file format directly: 100-byte database header, B-tree page headers, cell pointer arrays, varint-encoded payloads, and the `sqlite_master` schema table.

Built as a solution to the [CodeCrafters "Build Your Own SQLite"](https://codecrafters.io/challenges/sqlite) challenge.

## What it does

| Command | Behavior |
| --- | --- |
| `<db> .dbinfo` | Prints the database page size and number of tables, parsed from the file header and root page |
| `<db> .tables` | Lists user-defined table names by walking `sqlite_master` records on page 1 |

## How it works

The implementation reads the database byte-by-byte against the [SQLite file format spec](https://www.sqlite.org/fileformat.html):

- **Header parsing** — extracts page size from offsets 16–17 of the 100-byte database header.
- **B-tree pages** — distinguishes leaf vs. interior page headers (8 vs. 12 bytes) by the type byte at offset 100.
- **Cell pointer array** — walks the array of 16-bit cell offsets following the page header.
- **Varints** — a hand-written 1–9 byte big-endian varint decoder for payload sizes, row IDs, and serial types.
- **Record format** — parses `sqlite_master` rows (`type`, `name`, `tbl_name`, `rootpage`, `sql`) to enumerate the schema.

## Run it

```sh
# Get sample databases (sample.db is included; superheroes.db / companies.db via script)
./download_sample_databases.sh

# Build and query
cargo build --release
./your_program.sh sample.db .dbinfo
./your_program.sh sample.db .tables
```

Bundled `sample.db` has two tables (`apples`, `oranges`); the downloaded `superheroes.db` and `companies.db` are larger fixtures used for scan/index stages.

## Stack

Rust 2021 · `anyhow` · `bytes` · zero SQL/DB dependencies — everything is built up from the SQLite file format spec.
