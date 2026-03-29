# Phase 6: Concurrency Control

Making the database safe for multiple simultaneous transactions.

**Milestone:** Can run concurrent transactions with correct isolation, detect and resolve conflicts, and pass a stress test that interleaves reads and writes without anomalies.

## Resources (whole phase)

- **CMU 15-445 (Andy Pavlo)** — This is the most conceptually dense phase. The lectures are essential viewing, not optional background. Lectures 15–18 cover concurrency control end-to-end.
  - Lecture 15: Concurrency Control Theory — serializability, conflict graphs, the "why" behind everything in this phase
  - Lecture 16: Two-Phase Locking — 2PL, strict 2PL, lock types, deadlock handling
  - Lecture 17: Timestamp Ordering — optimistic vs. pessimistic approaches, T/O protocols
  - Lecture 18: Multi-Version Concurrency Control — MVCC mechanics, version chains, visibility rules
  - YouTube playlist: search "CMU 15-445 Fall 2023"
- **"Database Internals" by Alex Petrov** — Chapters 12–14 cover concurrency control in detail: lock-based, optimistic, and multi-version approaches. Chapter 13 on MVCC is particularly good.
- **Postgres source code** — `src/backend/access/transam/` (transaction manager), `src/backend/storage/lmgr/` (lock manager), `src/backend/utils/time/snapmgr.c` (snapshot management). Postgres is MVCC-native, so the visibility logic is everywhere in `src/backend/access/heap/heapam_visibility.c`.

## Sub-phases

### 6.1 — Transaction Manager
The lifecycle controller. Every operation in the database happens within a transaction — this struct tracks them.

- `begin() -> TxnId` — assign a monotonically increasing transaction ID
- `commit(TxnId)` / `abort(TxnId)` — finalize the transaction's outcome
- Track active transactions in a set (needed later for visibility checks)
- Store each transaction's state: `Active`, `Committed`, `Aborted`
- Record commit timestamps (logical, not wall-clock) for ordering

**Test:** Begin 100 transactions, commit some, abort some, verify IDs are strictly increasing and the active set is accurate.

**Resources:**
- CMU 15-445 Lecture 15 (Concurrency Control Theory) — defines transactions, ACID properties, the problem space
- Petrov Ch. 12 — transaction processing fundamentals
- Rust: `AtomicU64` for the ID counter, `HashMap<TxnId, TxnState>` behind a `Mutex` for the active set

### 6.2 — Lock Manager
Controls concurrent access to data. Two granularities: row-level and table-level, two modes: shared (read) and exclusive (write).

- Lock table: maps a `LockTarget` (table ID or row ID) to a queue of lock requests
- `lock_shared(TxnId, LockTarget)` / `lock_exclusive(TxnId, LockTarget)`
- `unlock(TxnId, LockTarget)`
- `upgrade(TxnId, LockTarget)` — promote shared to exclusive
- Implement strict 2PL: locks are held until transaction commit/abort, then released all at once
- Block (or return `WouldBlock`) when a conflicting lock is held

**Test:** Two transactions lock the same row — shared+shared succeeds, shared+exclusive blocks. After the first commits and releases, the second acquires.

**Resources:**
- CMU 15-445 Lecture 16 (Two-Phase Locking) — lock modes, 2PL rules, strict 2PL, lock upgrade
- Petrov Ch. 12 — lock-based concurrency control, lock compatibility matrices
- Rust: `Condvar` for blocking waiters, `HashMap<LockTarget, LockState>` for the lock table. Avoid holding the lock table's `Mutex` while waiting — that deadlocks the manager itself.

### 6.3 — Deadlock Detection
Two transactions can each hold a lock the other needs. You must detect or prevent this.

- Build a wait-for graph: directed edge from T1 to T2 if T1 is waiting for a lock T2 holds
- Run cycle detection (DFS) periodically or on every wait
- On cycle: abort the youngest transaction (least work lost)
- Alternative: implement wait-die or wound-wait prevention (no graph needed, simpler but more aborts)

**Test:** Engineer a deadlock — T1 locks A then waits on B, T2 locks B then waits on A. Verify one is aborted and the other proceeds.

**Resources:**
- CMU 15-445 Lecture 16 — deadlock detection vs. prevention, wait-for graphs, timeout-based approaches
- Petrov Ch. 12 — deadlock handling strategies
- Rust: model the wait-for graph as `HashMap<TxnId, HashSet<TxnId>>`. Cycle detection is a textbook DFS — nothing exotic.

### 6.4 — MVCC Implementation
Instead of blocking readers, keep multiple versions of each tuple. Readers see a consistent snapshot without acquiring locks.

- Add `xmin` (creating transaction ID) and `xmax` (deleting transaction ID) to every tuple header
- On INSERT: set `xmin = current_txn`, `xmax = 0`
- On DELETE: set `xmax = current_txn` (don't physically remove yet)
- On UPDATE: delete old version + insert new version (new tuple, new `xmin`)
- Version chains: link old and new versions so you can walk back to the right one
- Visibility check: a tuple is visible to transaction T if `xmin` is committed and (`xmax` is 0 or `xmax` is not committed)

**Test:** T1 inserts a row. T2 (started before T1 commits) cannot see it. T2 (started after T1 commits) can. Delete the row in T3, verify T2 still sees it if T2 started before T3 committed.

**Resources:**
- CMU 15-445 Lecture 18 (MVCC) — version storage schemes, visibility rules, garbage collection
- Petrov Ch. 13 — MVCC in depth, append-only vs. delta storage, version ordering
- Postgres: `heapam_visibility.c` — the actual visibility check logic. `HeapTupleSatisfiesMVCC` is the function to study.
- Rust: store versions in a `Vec` per primary key, or thread a linked list through the page. Start with the simple `Vec` approach.

### 6.5 — Isolation Levels
Different guarantees for different needs. Implement two practical levels on top of MVCC.

- **Read Committed:** each statement sees only data committed before that statement started. Take a new snapshot per statement.
- **Snapshot Isolation (SI):** the transaction sees a consistent snapshot from its start time. All reads within the transaction see the same data, regardless of concurrent commits.
- Snapshot = the set of committed transaction IDs at a point in time
- Modify the visibility check to use the snapshot instead of checking "is it committed right now"
- Detect write-write conflicts under SI: if two concurrent transactions modify the same row, the second to commit must abort ("first committer wins")

**Test:** Under Read Committed — T1 reads row, T2 updates and commits, T1 reads again and sees the new value. Under Snapshot Isolation — same scenario, T1's second read still sees the old value.

**Resources:**
- CMU 15-445 Lecture 18 — isolation levels, anomalies each level permits, snapshot isolation vs. serializability
- Petrov Ch. 13 — isolation levels in MVCC systems, write skew anomaly
- Postgres: `snapmgr.c` — how snapshots are taken and compared. `GetSnapshotData()` builds the active transaction list.
- The paper "A Critique of ANSI SQL Isolation Levels" (Berenson et al., 1995) — defines snapshot isolation precisely and catalogs the anomalies. Worth reading once.

### 6.6 — Serializable Isolation
Snapshot isolation still allows write skew. SSI (Serializable Snapshot Isolation) closes the gap without falling back to full locking.

- Start from snapshot isolation
- Track read-write dependencies: if T1 reads data that T2 later writes (rw-conflict), record the dependency
- Track write-read dependencies: if T1 writes data that T2 later reads
- Detect dangerous structures: two consecutive rw-dependencies forming a cycle (T1 ->rw T2 ->rw T3 ->rw T1, or the "pivot" pattern)
- On detection: abort one transaction to break the cycle
- This is the hardest sub-phase. Get 6.5 solid before starting.

**Test:** Classic write skew — two doctors on call, each checks "is the other on call?" and tries to go off call. Under SI both succeed (bug). Under SSI one is aborted.

**Resources:**
- CMU 15-445 Lecture 18 — mentions SSI briefly; the paper below is the real source
- The paper "Serializable Snapshot Isolation in PostgreSQL" (Ports & Grittner, 2012) — the actual design Postgres uses. Readable and practical.
- Postgres: `src/backend/storage/lmgr/predicate.c` — the SSI implementation. Large file, but the comments at the top are a mini-paper.
- Petrov Ch. 14 — discusses serializable isolation approaches
- Rust: track rw-dependencies with a `HashMap<TxnId, Vec<TxnId>>`. The detection logic is graph-based, similar to deadlock detection but over dependency edges.
