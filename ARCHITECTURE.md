# PureDB Architecture

A multi-threaded relational database built from scratch in Rust. Emphasis on storage-engine depth: flash-aware I/O, high-performance caching, replication, and data reduction.

## Subsystem Map

```
                        +-----------------------+
                        |    Network Layer      |
                        |  (minimal TCP,        |
                        |   basic wire proto)   |
                        +-----------+-----------+
                                    |
                        +-----------v-----------+
                        |    SQL Parser         |
                        |    & Planner          |
                        |  (minimal: parse,     |
                        |   bind, naive plan)   |
                        +-----------+-----------+
                                    |
                        +-----------v-----------+
                        |    Execution Engine   |
                        |  (minimal: scan,      |
                        |   filter, join)       |
                        +-----------+-----------+
                                    |
                  +----------------++-----------------+
                  |                                    |
        +---------v----------+            +-----------v-----------+
        |  Concurrency       |            |    Access Methods     |
        |  Control           |            |  (B+Tree, hash index) |
        |  (lock-free,       |            +-----------+-----------+
        |   MVCC, benchmarks)|                        |
        +---------+----------+            +-----------v-----------+
                  |                       |    Tuple Layout       |
                  |                       |    & Catalog          |
                  |                       +-----------+-----------+
                  |                                   |
                  +----------------+-----------------+
                                   |
                  +----------------v-----------------+
                  |         Storage Engine            |
                  |  (direct I/O, io_uring,           |
                  |   flash-aware allocation,         |
                  |   ARC/tiered cache, compression)  |
                  +----------------+-----------------+
                                   |
                    +--------------+--------------+
                    |              |               |
          +---------v-----+  +----v--------+  +---v--------------+
          | Recovery (WAL) |  | Replication |  | Snapshots &      |
          | (ARIES,        |  | & Consensus |  |  Data Reduction  |
          |  checkpoints)  |  | (Raft, WAL  |  | (CoW, LZ4,      |
          +----------------+  |  shipping)  |  |  dedup)          |
                              +-------------+  +------------------+
```

## Subsystems

### 1. Storage Engine (deep)
The foundation and the project's primary differentiator. Goes beyond textbook buffer pools into production storage-system territory.

- **Disk Manager** — reads/writes fixed-size pages to/from files
- **Direct I/O** — bypass OS page cache with `O_DIRECT` for predictable latency
- **io_uring Integration** — async I/O submission for high-throughput page reads/writes
- **Page Format** — internal layout of a page (header, slots, tuples)
- **Buffer Pool** — in-memory cache of disk pages with ARC (Adaptive Replacement Cache) eviction
- **Tiered Caching** — hot/cold tier separation for access-frequency-aware eviction
- **Inline Compression** — LZ4/zstd page-level compression, transparent to layers above
- **Flash-Aware Allocation** — log-structured or copy-on-write page allocation to minimize write amplification on SSDs
- **Freelist / Free Space Map** — tracks which pages have room for new tuples

### 2. Tuple Layout & Catalog (moderate)
How individual rows are encoded and how the system knows what tables exist.

- **Tuple format** — field offsets, null bitmap, variable-length data
- **Data types** — integer, float, varchar, bool, etc. + serialization
- **System catalog** — internal tables describing all user tables/columns/indexes

### 3. Access Methods / Indexing (moderate)
Structures that speed up lookups beyond sequential scan.

- **B+Tree Index** — ordered index supporting range scans
- **Hash Index** — equality-only lookups
- **Index scan vs. sequential scan** — choosing the right path

### 4. Execution Engine (minimal)
Runs physical plans and produces result rows. Kept minimal — just enough to exercise the storage stack.

- **Volcano/Iterator model** — `open()` / `next()` / `close()` per operator
- **Operators** — SeqScan, Filter, Projection, NestedLoopJoin or HashJoin
- **Expression evaluation** — arithmetic, comparisons, boolean logic

### 5. SQL Parser & Planner (minimal)
Transforms SQL text into an executable plan. Kept minimal — no optimizer.

- **Lexer** — SQL string → tokens
- **Parser** — tokens → AST (SELECT, INSERT, CREATE TABLE)
- **Binder** — resolves names against the catalog, type-checks
- **Naive Planner** — AST → physical plan (no rewriting, no index selection)

### 6. Concurrency Control (deep)
Makes the database safe for multiple threads, with emphasis on high-performance primitives.

- **Lock-free buffer pool internals** — concurrent hash map, epoch-based reclamation
- **Custom reader-writer latches** — fair scheduling, writer priority
- **Lock manager** — logical locks on rows/tables, deadlock detection
- **Transaction manager** — begin / commit / abort lifecycle
- **MVCC** — multi-version concurrency control
- **Isolation levels** — read committed, snapshot isolation
- **Benchmarks** — criterion benchmarks on all hot paths, p99 latency tracking

### 7. Recovery / WAL (moderate)
Ensures committed data survives crashes and uncommitted data is rolled back.

- **Write-Ahead Log (WAL)** — append-only log of all changes
- **ARIES protocol** — redo history, undo losers
- **Checkpointing** — periodic snapshots to bound recovery time

### 8. Network Layer (minimal)
How external clients connect. Minimal — exists to demo the system, not as a goal in itself.

- **TCP listener** — accept connections, spawn handlers
- **Simple wire protocol** — send SQL, receive results (custom, not Postgres-compatible)
- **Result serialization** — sending rows back to clients

### 9. Replication & Consensus (deep)
Distributed storage fundamentals — the layer that makes the system fault-tolerant.

- **WAL shipping** — stream log records from leader to followers
- **Simplified Raft** — leader election, log replication, commitment protocol
- **Failure detection** — heartbeats, timeouts, automatic failover
- **Consistency guarantees** — linearizable writes, eventually consistent reads from followers

### 10. Snapshots & Data Reduction (deep)
Data reduction and instant snapshots at the storage layer.

- **Copy-on-Write snapshots** — page-level CoW for instant point-in-time snapshots
- **Snapshot management** — create, list, delete, restore from snapshot
- **Inline compression** — LZ4 at the page level, transparent to upper layers
- **Block-level deduplication** — content-addressable pages, reference counting
