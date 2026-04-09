# Phase 6: Concurrency Control (deep)

Making the database safe for multiple simultaneous transactions, with emphasis on high-performance synchronization primitives and measurable throughput.

**Milestone:** Concurrent transactions with correct isolation, lock-free buffer pool internals, and criterion benchmarks proving throughput under contention.

## Resources (whole phase)

- **CMU 15-445 (Andy Pavlo)** — Lectures 15–18 cover concurrency control end-to-end.
  - Lecture 15: Concurrency Control Theory — serializability, conflict graphs
  - Lecture 16: Two-Phase Locking — 2PL, strict 2PL, lock types, deadlock handling
  - Lecture 17: Timestamp Ordering — optimistic vs. pessimistic approaches
  - Lecture 18: Multi-Version Concurrency Control — MVCC mechanics, visibility rules
  - YouTube playlist: search "CMU 15-445 Fall 2023"
- **"Database Internals" by Alex Petrov** — Chapters 12–14 cover concurrency control in detail.
- **"The Art of Multiprocessor Programming" (Herlihy & Shavit)** — the reference for lock-free data structures and concurrent algorithms.
- **crossbeam crate** — epoch-based memory reclamation for lock-free structures in Rust.

## Sub-phases

### 6.1 — Lock-Free Buffer Pool Internals
Replace the `Mutex<HashMap>` page table with a concurrent hash map. This is foundational — the buffer pool is the hottest path in the system.

- Implement or use a lock-free concurrent hash map (crossbeam epoch-based, or hand-roll with atomic CAS)
- Atomic pin counts per frame
- Latch-free fast path for cache hits (the common case should touch no locks at all)
- Benchmark: measure throughput (ops/sec) and p99 latency under varying thread counts (1, 2, 4, 8, 16)

**Test:** Spawn 16 threads doing random fetch/unpin on a hot working set. Assert no data corruption. Benchmark with criterion.

**Resources:**
- crossbeam's `SkipMap` or hand-rolling with `crossbeam-epoch` for safe lock-free memory reclamation
- "Lock-Free Hash Tables" — research papers by Cliff Click (used in Java's ConcurrentHashMap)
- Rust: `std::sync::atomic::{AtomicU64, AtomicPtr}`, `Ordering::*`

### 6.2 — Custom Reader-Writer Latch
Build your own reader-writer lock with fairness guarantees instead of relying on `std::sync::RwLock`.

- Writer-priority mode: writers don't starve under heavy read load
- Fair mode: FIFO ordering prevents either side from starving
- Compare performance against `std::sync::RwLock` and `parking_lot::RwLock`
- Use this latch for per-frame protection in the buffer pool

**Test:** Benchmark: 8 reader threads + 2 writer threads, measure writer wait time under both policies. Verify no starvation.

**Resources:**
- "The Art of Multiprocessor Programming" Ch. 8 — reader-writer locks, fairness
- `parking_lot` source code — a well-optimized RwLock to study
- Rust: `AtomicU32` + `Futex` (Linux) or `std::thread::park`/`unpark` for blocking

### 6.3 — Transaction Manager
The lifecycle controller.

- `begin() -> TxnId` — monotonically increasing transaction IDs
- `commit(TxnId)` / `abort(TxnId)` — finalize outcome
- Track active transactions (needed for visibility checks)
- Store transaction state: `Active`, `Committed`, `Aborted`
- Commit timestamps for ordering

**Test:** Begin 100 transactions, commit some, abort some, verify IDs are strictly increasing and the active set is accurate.

**Resources:**
- CMU 15-445 Lecture 15 — defines transactions, ACID properties
- Rust: `AtomicU64` for the ID counter, concurrent map for the active set

### 6.4 — Lock Manager & Deadlock Detection
Controls concurrent access to data.

- Lock table: maps `LockTarget` (table ID or row ID) to a queue of lock requests
- Shared and exclusive lock modes, strict 2PL
- Wait-for graph with cycle detection (DFS), abort youngest on deadlock
- Alternative: wound-wait prevention (no graph needed, simpler)

**Test:** Engineer a deadlock — T1 locks A then waits on B, T2 locks B then waits on A. Verify one is aborted.

**Resources:**
- CMU 15-445 Lecture 16 — lock modes, 2PL, deadlock detection
- Rust: `Condvar` for blocking waiters. Avoid holding the lock table mutex while waiting.

### 6.5 — MVCC Implementation
Multiple versions per tuple. Readers see consistent snapshots without locks.

- `xmin`/`xmax` fields on every tuple header
- INSERT: `xmin = current_txn`, `xmax = 0`
- DELETE: `xmax = current_txn`
- UPDATE: delete old + insert new
- Visibility check using snapshot of committed transactions

**Test:** T1 inserts a row. T2 started before T1 committed cannot see it. T3 started after can.

**Resources:**
- CMU 15-445 Lecture 18 — version storage schemes, visibility rules
- Postgres: `heapam_visibility.c` — `HeapTupleSatisfiesMVCC`

### 6.6 — Isolation Levels
Implement Read Committed and Snapshot Isolation on top of MVCC.

- **Read Committed:** new snapshot per statement
- **Snapshot Isolation:** snapshot at transaction start, first-committer-wins for write-write conflicts

**Test:** Under RC — T1 reads, T2 updates and commits, T1 reads again and sees new value. Under SI — T1's second read still sees old value.

**Resources:**
- CMU 15-445 Lecture 18 — isolation levels, anomalies
- "A Critique of ANSI SQL Isolation Levels" (Berenson et al., 1995)

### 6.7 — Benchmarks & Profiling
Quantify everything. This sub-phase is about proving performance, not adding features.

- criterion benchmarks for: buffer pool fetch (hit/miss), lock acquire/release, transaction begin/commit, MVCC visibility check
- Measure throughput (ops/sec) at 1, 2, 4, 8, 16 threads
- Measure p99 latency for each operation
- Flame graph profiling to identify bottlenecks
- Document results: what scaled, what didn't, and why

**Test:** All benchmarks run without errors. Results are reproducible across runs (low variance).

**Resources:**
- `criterion` crate — statistical benchmarking for Rust
- `flamegraph` crate / `perf` + `inferno` — flame graph generation
- "Systems Performance" by Brendan Gregg — methodology for performance analysis
