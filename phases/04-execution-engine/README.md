# Phase 4: Execution Engine

Runs query plans and produces result rows. Plans are built programmatically (no SQL parser yet) using the volcano/iterator model.

**Milestone:** Can execute a programmatically-built plan involving scans, filters, joins, sorts, and aggregations, and stream result tuples one at a time.

## Resources (whole phase)

- **CMU 15-445 (Andy Pavlo)** — Lectures 11–13 cover this entire phase.
  - Lecture 11: Query Execution I — processing models (iterator, materialization, vectorized), expression evaluation
  - Lecture 12: Query Execution II — parallel execution, operator output
  - Lecture 13: Query Execution III — sorting, aggregations, joins
  - YouTube playlist: search "CMU 15-445 Fall 2023"
  - Course projects (Bustub): Project 3 is "build a query execution engine" — directly relevant
- **"Database Internals" by Alex Petrov** — Chapter 5 touches on query processing. Less depth here than CMU lectures; treat as supplemental.
- **Postgres source code** — `src/backend/executor/` (executor nodes), `src/include/executor/` (node definitions). Each operator is its own file: `nodeSeqscan.c`, `nodeHashjoin.c`, etc.

## Sub-phases

### 4.1 — Iterator/Volcano Model

Define the core execution abstraction. Every operator is an iterator that produces one tuple at a time, pulling from its child operators on demand.

- Define a `Tuple` type — a row of typed values passed between operators. Start simple: `Vec<Value>` where `Value` is an enum (`Integer(i64)`, `Text(String)`, `Boolean(bool)`, `Null`, etc.)
- Define the `Executor` trait:
  - `init()` — open resources, initialize child operators
  - `next() -> Option<Tuple>` — return the next tuple, or `None` when exhausted
  - `close()` — release resources
- Each operator holds its children as `Box<dyn Executor>`
- Build a trivial `Values` operator (emits hardcoded tuples) to test the wiring

**Test:** Create a `Values` operator with 5 tuples, call `next()` 5 times, assert each tuple matches, 6th call returns `None`.

**Resources:**
- CMU 15-445 Lecture 11 (Query Execution I) — iterator model explained, pull-based vs push-based tradeoffs
- Graefe, "Volcano — An Extensible and Parallel Query Evaluation System" (1994) — the original paper, short and readable
- Rust: trait objects (`Box<dyn Trait>`), `Option<T>` as the natural return for `next()`

### 4.2 — SeqScan Operator

Scan every tuple in a table heap, deserializing each page's slotted data into `Tuple` values. This connects the execution engine to the storage layer from Phase 1.

- Takes a table (heap file) reference and a schema describing column types/offsets
- Iterates page by page, slot by slot
- Deserializes raw bytes into `Tuple` using the schema
- Must handle deleted slots (skip them)

**Test:** Insert 200 tuples into a heap via the storage layer, run `SeqScan`, collect all results, verify count and contents match.

**Resources:**
- CMU 15-445 Lecture 11 — sequential scan as the simplest access method
- Postgres: `src/backend/executor/nodeSeqscan.c` — see `SeqNext()` for the real implementation
- This operator depends heavily on your Phase 1 page layout and buffer pool — if the storage API is awkward to iterate, fix it now

### 4.3 — Filter & Projection Operators

Two thin operators that sit above a scan (or any child) and transform its output.

- **Filter:** wraps a child executor and a predicate (`Fn(&Tuple) -> bool`). Calls `child.next()` in a loop until it finds a matching tuple or exhaustion. Start with closures; later you may want an `Expression` AST.
- **Projection:** wraps a child executor and a list of column indices (or expressions). Maps each input tuple to a narrower output tuple.
- Compose them: `Projection(Filter(SeqScan(...)))` should just work

**Test:** Scan a table of 100 rows, filter where column 0 > 50, project columns 1 and 2. Verify output tuple count and shape.

**Resources:**
- CMU 15-445 Lecture 11 — selection and projection in the iterator model
- Postgres: `src/backend/executor/nodeResult.c` (projection), `src/backend/executor/execQual.c` (predicate evaluation)
- Rust: `Box<dyn Fn(&Tuple) -> bool>` for predicates, or define an `Expr` enum if you want something serializable

### 4.4 — Nested Loop Join

The simplest join algorithm. For each tuple in the outer, scan the entire inner. Slow but correct — use it as the correctness baseline for fancier joins.

- Takes two child executors (outer, inner) and a join predicate
- For each outer tuple, rewind the inner (call `close()` + `init()`, or add a `rewind()` method)
- Emit concatenated tuples that satisfy the predicate
- Support inner join first; left outer join is a natural extension

**Test:** Two tables: `departments` (5 rows) and `employees` (20 rows). Join on `dept_id`. Verify output count matches expected join cardinality.

**Resources:**
- CMU 15-445 Lecture 13 — nested loop join, cost analysis (O(M * N) pages)
- Postgres: `src/backend/executor/nodeNestloop.c`
- Consider adding `rewind()` to the `Executor` trait — nested loop join needs to restart the inner for every outer tuple

### 4.5 — Hash Join

Build a hash table on the smaller (inner) relation, then probe with each tuple from the outer. Much faster than nested loop for equi-joins.

- **Build phase:** consume all tuples from the inner child, hash on the join key, store in a `HashMap<Value, Vec<Tuple>>`
- **Probe phase:** for each outer tuple, hash the join key and look up matches
- Handle multiple matches per key (emit one result per call to `next()`)
- Only supports equi-joins (equality on join key)

**Test:** Same departments/employees tables. Hash join on `dept_id`. Compare results with nested loop join — must be identical (order may differ).

**Resources:**
- CMU 15-445 Lecture 13 — hash join build/probe, grace hash join for when the hash table doesn't fit in memory
- Postgres: `src/backend/executor/nodeHashjoin.c`, `src/backend/executor/nodeHash.c`
- Rust: `HashMap` from std, implement `Hash` + `Eq` for your `Value` type (watch out for floats)

### 4.6 — Sort Operator

Materializes all input tuples, sorts them, then emits them one at a time. Required for ORDER BY and as a building block for merge join and sort-based aggregation.

- **In-memory sort:** consume all child tuples into a `Vec`, sort by the specified key columns using `sort_by`
- **External merge sort:** when data exceeds a memory budget, sort in chunks (runs), write runs to temp files, merge them back. This is the first time the execution engine spills to disk.
- Support multi-column sort keys and ASC/DESC per column

**Test (in-memory):** Sort 1000 random tuples by column 0 ascending, verify output is ordered.
**Test (external):** Set memory budget to hold ~50 tuples, sort 500. Verify output is ordered and temp files were created/cleaned up.

**Resources:**
- CMU 15-445 Lecture 13 — external merge sort algorithm, number of passes, I/O cost
- Petrov Ch. 5 — external sorting briefly covered
- Postgres: `src/backend/utils/sort/tuplesort.c` — the real external sort, complex but well-commented
- Rust: `Vec::sort_by()` for in-memory, `BufWriter`/`BufReader` with temp files for external sort

### 4.7 — Aggregation Operator

Groups tuples by key columns and computes aggregate functions. Two strategies: hash aggregation (default) and sort-based aggregation (if input is already sorted).

- Supported aggregates: `COUNT`, `SUM`, `AVG`, `MIN`, `MAX`
- **Hash aggregation:** build a `HashMap<GroupKey, Accumulators>`, consume all input, then emit one result tuple per group
- **Sort-based aggregation:** if input is pre-sorted by the group key, accumulate in a single pass without a hash table
- Handle `GROUP BY` with zero group keys (whole-table aggregation, e.g., `SELECT COUNT(*) FROM t`)
- `AVG` = track both sum and count, compute at emit time

**Test:** Table with 1000 rows, 10 distinct values in the group column. `GROUP BY` that column, compute `COUNT`, `SUM`, `AVG`, `MIN`, `MAX`. Verify each group's results against a brute-force calculation.

**Resources:**
- CMU 15-445 Lecture 13 — hash aggregation vs sort aggregation, running aggregates
- Postgres: `src/backend/executor/nodeAgg.c` — handles both strategies
- Rust: define an `Accumulator` trait with `accumulate(&mut self, &Value)` and `finalize(self) -> Value` — keeps aggregate logic modular
