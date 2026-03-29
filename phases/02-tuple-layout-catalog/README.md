# Phase 2: Tuple Layout & Catalog

How individual rows are encoded into bytes, and how the system knows what tables exist.

**Milestone:** Can define a table with typed columns, insert rows as serialized tuples on heap pages, and look up table/column metadata from an internal catalog.

## Resources (whole phase)

- **CMU 15-445 (Andy Pavlo)** — Lectures 4, 5, and 12 cover tuple representation, heap files, and system catalogs.
  - Lecture 4: Database Storage II — tuple layout, null bitmaps, variable-length data
  - Lecture 5: Buffer Pools — relevant when integrating heap pages with the buffer pool from Phase 1
  - Lecture 12: Query Processing I — touches on how the executor interacts with the catalog and tuple format
  - YouTube playlist: search "CMU 15-445 Fall 2023"
- **"Database Internals" by Alex Petrov** — Chapters 3–4 cover data encoding, slotted pages with real tuple data, and file organization.
- **Postgres source code** — `src/backend/access/heap/` (heap tuple operations), `src/backend/access/common/heaptuple.c` (tuple serialization), `src/backend/catalog/` (system catalog bootstrap).

## Sub-phases

### 2.1 — Data Types

Define the set of types the database supports. Each type needs a binary representation and comparison semantics.

- Supported types: `Integer` (i32), `BigInt` (i64), `Float` (f64), `Bool` (bool), `Varchar(n)` (variable-length UTF-8, max n bytes)
- `encode(&self, buf: &mut Vec<u8>)` and `decode(type_id, bytes) -> Value` for each type
- Fixed-size types write directly; `Varchar` writes a 2-byte length prefix followed by the UTF-8 bytes
- Implement `Ord` / `PartialOrd` for `Value` (needed for B+Tree keys in Phase 3)
- Implement `Hash` for `Value` (needed for hash joins / hash indexes later)
- Float comparison: handle NaN consistently (e.g., NaN sorts last) — this is a real Postgres behavior

**Test:** Round-trip every type through encode/decode. Verify sort order: integers sort numerically, varchars sort lexicographically, NaN floats have stable ordering.

**Resources:**
- CMU 15-445 Lecture 4 (Database Storage II) — type systems, data representation in tuples
- Petrov Ch. 3 — encoding integers, strings, variable-length data
- Postgres: `src/include/catalog/pg_type.dat` — how Postgres defines its built-in types
- Rust: derive `PartialEq`, `Eq`, `Hash` manually for the `Value` enum; use `total_cmp()` on `f64` (stabilized in Rust 1.62) for IEEE-correct float ordering

### 2.2 — Tuple Serialization

Encode a full row (an ordered list of `Value`s) into a flat byte buffer, and decode it back. This is the wire format that lives inside pages.

- Null bitmap: one bit per column at the start of the tuple, packed into bytes
- Fixed-length fields: written in column order after the bitmap
- Variable-length fields: field offset array (2 bytes per var-length column) pointing into a trailing data region
- `serialize(schema: &[ColumnDef], values: &[Option<Value>]) -> Vec<u8>`
- `deserialize(schema: &[ColumnDef], bytes: &[u8]) -> Vec<Option<Value>>`
- Handle all-null rows and rows with no variable-length columns as edge cases

**Test:** Serialize a row with mixed types (some null), deserialize it, assert equality. Verify the byte length matches expectations for known inputs.

**Resources:**
- CMU 15-445 Lecture 4 — tuple format, null indicators, field offsets
- Petrov Ch. 3 — record layout, variable-size data encoding
- Postgres: `src/backend/access/common/heaptuple.c` — `heap_form_tuple`, `heap_deform_tuple` are the real implementations
- Rust: `byteorder` crate or `{i32,i64,f64}::to_le_bytes()` / `from_le_bytes()` for endian-safe encoding. Prefer the stdlib methods — no dependency needed.

### 2.3 — Table Heap

Insert, read, update, and delete serialized tuples on heap pages, using the page layout from Phase 1. Manage free space across multiple pages.

- `TableHeap` struct: owns a table's set of pages, backed by the buffer pool
- `insert_tuple(tuple: &[u8]) -> TupleId` — find a page with enough free space, insert, return (page_id, slot_id)
- `get_tuple(TupleId) -> Option<Vec<u8>>` — fetch the page, read the slot
- `delete_tuple(TupleId)` — mark the slot as deleted (don't physically remove yet)
- `update_tuple(TupleId, new_data: &[u8])` — delete + insert (simple strategy; in-place update if it fits is an optimization for later)
- Free space map: a lightweight structure tracking approximate free space per page, so inserts don't scan every page — even a simple `Vec<(PageId, u16)>` works to start
- Sequential scan: `TableIterator` that walks every live tuple across all pages

**Test:** Insert 1000 tuples across multiple pages, delete every other one, scan and verify only the expected tuples remain. Insert more tuples, confirm they reuse freed space.

**Resources:**
- CMU 15-445 Lecture 4 — heap file organization, tuple identifiers
- Petrov Ch. 3–4 — heap files, free space management
- Postgres: `src/backend/access/heap/heapam.c` — heap access method, `src/backend/storage/freespace/` — free space map implementation
- Rust: the `TableIterator` is a natural fit for implementing `Iterator<Item = (TupleId, Vec<u8>)>`. Use the buffer pool's `fetch_page` / `unpin` from Phase 1.

### 2.4 — System Catalog

Bootstrap internal tables that describe all user tables and their columns. Without a catalog, the system has no memory of what it has created.

- Two core catalog tables (inspired by Postgres):
  - `pg_class` equivalent: table OID, table name, first page ID, column count
  - `pg_attribute` equivalent: table OID, column index, column name, type ID, nullable flag
- The catalog tables are themselves stored as tuples in heap pages — the catalog is self-describing
- `Catalog` struct with methods:
  - `create_table(name, columns) -> TableId`
  - `get_table(name) -> Option<TableInfo>`
  - `get_columns(table_id) -> Vec<ColumnInfo>`
  - `drop_table(name)` (mark deleted in catalog)
- Bootstrap problem: the catalog tables must exist before any table can be created. Hard-code their schemas and page locations at database init time.
- Assign stable OIDs to catalog tables (e.g., `pg_class` = OID 1, `pg_attribute` = OID 2)

**Test:** Create three tables with different schemas, restart the system (re-read from disk), verify all table and column metadata survives. Drop a table, confirm it no longer appears in lookups.

**Resources:**
- CMU 15-445 Lecture 12 (Query Processing I) — system catalog, schema information
- Petrov Ch. 4 — metadata management, catalog structures
- Postgres: `src/backend/catalog/pg_class.c`, `src/include/catalog/pg_class.h`, `src/include/catalog/pg_attribute.h` — the real catalog definitions. `src/backend/catalog/heap.c` — `heap_create_with_catalog` shows the bootstrap flow.
- Rust: consider a `Catalog` that wraps a `TableHeap` for each catalog table. The bootstrap sequence is a good place for a dedicated `init_catalog(buffer_pool)` function.
