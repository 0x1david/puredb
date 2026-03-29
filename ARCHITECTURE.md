# PureDB Architecture

A multi-threaded relational database built from scratch in Rust.

## Subsystem Map

```
                        +-----------------------+
                        |    Network Layer      |
                        |  (wire protocol,      |
                        |   connections)         |
                        +-----------+-----------+
                                    |
                        +-----------v-----------+
                        |    SQL Parser         |
                        |    & Planner          |
                        |  (lexer, parser,      |
                        |   binder/analyzer)    |
                        +-----------+-----------+
                                    |
                        +-----------v-----------+
                        |    Execution Engine   |
                        |  (operators, iterator |
                        |   model, expressions) |
                        +-----------+-----------+
                                    |
                  +----------------++-----------------+
                  |                                    |
        +---------v----------+            +-----------v-----------+
        |  Concurrency       |            |    Access Methods     |
        |  Control           |            |  (B+Tree, hash index) |
        |  (locks, MVCC,     |            +-----------+-----------+
        |   transactions)    |                        |
        +---------+----------+            +-----------v-----------+
                  |                       |    Tuple Layout       |
                  |                       |    & Catalog          |
                  |                       +-----------+-----------+
                  |                                   |
                  +----------------+-----------------+
                                   |
                        +----------v-----------+
                        |    Storage Engine     |
                        |  (pages, buffer pool, |
                        |   disk manager)       |
                        +----------+-----------+
                                   |
                    +--------------+--------------+
                    |                              |
          +---------v----------+       +-----------v-----------+
          |  Recovery (WAL)    |       |      Disk / Files     |
          |  (write-ahead log, |       |  (data pages, index   |
          |   ARIES,           |       |   pages, catalog)     |
          |   checkpoints)     |       +-----------------------+
          +--------------------+
```

## Subsystems

### 1. Storage Engine
The foundation. Manages how bytes are stored on and read from disk.

- **Disk Manager** — reads/writes fixed-size pages to/from files
- **Page Format** — internal layout of a page (header, slots, tuples)
- **Buffer Pool** — in-memory cache of disk pages with eviction (LRU/clock)
- **Freelist / Free Space Map** — tracks which pages have room for new tuples

### 2. Tuple Layout & Catalog
How individual rows are encoded and how the system knows what tables exist.

- **Tuple format** — field offsets, null bitmap, variable-length data
- **Data types** — integer, float, varchar, bool, etc. + serialization
- **System catalog** — internal tables describing all user tables/columns/indexes

### 3. Access Methods (Indexing)
Structures that speed up lookups beyond sequential scan.

- **B+Tree Index** — ordered index supporting range scans
- **Hash Index** — equality-only lookups
- **Index scan vs. sequential scan** — choosing the right path

### 4. Execution Engine
Runs the physical plan and produces result rows.

- **Volcano/Iterator model** — `open()` / `next()` / `close()` per operator
- **Operators** — SeqScan, IndexScan, Filter, Projection, NestedLoopJoin, HashJoin, Sort, Aggregate
- **Expression evaluation** — arithmetic, comparisons, boolean logic

### 5. SQL Parser & Planner
Transforms SQL text into an executable plan.

- **Lexer** — SQL string → tokens
- **Parser** — tokens → AST
- **Binder/Analyzer** — resolves names against the catalog, type-checks
- **Planner/Optimizer** — AST → physical plan (join ordering, index selection)

### 6. Concurrency Control
Makes the database safe for multiple threads/transactions.

- **Latch management** — lightweight mutexes on in-memory structures
- **Lock manager** — logical locks on rows/tables, deadlock detection
- **Transaction manager** — begin / commit / abort lifecycle
- **MVCC** — multi-version concurrency control (Postgres-style)
- **Isolation levels** — read committed, snapshot isolation, serializable

### 7. Recovery (WAL & Crash Safety)
Ensures committed data survives crashes and uncommitted data is rolled back.

- **Write-Ahead Log (WAL)** — append-only log of all changes
- **ARIES protocol** — redo history, undo losers
- **Checkpointing** — periodic snapshots to bound recovery time

### 8. Network Layer
How external clients connect and communicate.

- **Wire protocol** — Postgres-compatible or custom
- **Connection handling** — thread-per-connection or async (tokio)
- **Result serialization** — sending rows back to clients
