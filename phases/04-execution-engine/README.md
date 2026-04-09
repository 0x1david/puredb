# Phase 4: Execution Engine (minimal)

Runs query plans and produces result rows. Kept minimal — the goal is to exercise the storage and index layers, not to build a feature-complete query engine.

**Milestone:** Can execute a programmatically-built plan involving scans, filters, projections, and one join type, and stream result tuples one at a time.

## Resources

- **CMU 15-445 (Andy Pavlo)** — Lectures 11–13 cover query execution.
  - Lecture 11: Query Execution I — iterator model, expression evaluation
  - Lecture 13: Query Execution III — sorting, aggregations, joins
  - YouTube playlist: search "CMU 15-445 Fall 2023"
- **Postgres source code** — `src/backend/executor/` (executor nodes). Each operator is its own file: `nodeSeqscan.c`, `nodeHashjoin.c`, etc.

## Sub-phases

### 4.1 — Iterator/Volcano Model

Define the core execution abstraction. Every operator produces one tuple at a time.

- Define a `Tuple` type: `Vec<Value>` where `Value` is an enum (`Integer(i64)`, `Text(String)`, `Boolean(bool)`, `Null`, etc.)
- Define the `Executor` trait: `init()`, `next() -> Option<Tuple>`, `close()`
- Each operator holds its children as `Box<dyn Executor>`
- Build a trivial `Values` operator (emits hardcoded tuples) to test the wiring

**Test:** Create a `Values` operator with 5 tuples, call `next()` 5 times, assert each tuple matches, 6th call returns `None`.

### 4.2 — SeqScan, Filter, Projection

The three operators needed for basic SELECT queries.

- **SeqScan:** iterate page by page, slot by slot over a table heap, deserialize into tuples
- **Filter:** wraps a child executor and a predicate, skips non-matching tuples
- **Projection:** wraps a child executor and a column list, maps each tuple to a narrower output
- Compose them: `Projection(Filter(SeqScan(...)))` should just work

**Test:** Insert 200 tuples, run `Projection(Filter(SeqScan(...)))`, verify correct count and column shape.

### 4.3 — Join Operator

One join algorithm — either nested loop join (simpler) or hash join (faster). Pick one.

- **Nested Loop Join:** for each outer tuple, scan the entire inner. Needs `rewind()` on the inner.
- **Hash Join:** build a hash table on the inner, probe with the outer. Better for equi-joins.

**Test:** Join two tables on a shared key, verify output count matches expected cardinality.
