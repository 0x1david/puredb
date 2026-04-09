# PureDB Roadmap

Build order is bottom-up: storage first, SQL last. Each phase produces a working, testable artifact.

Depth annotations: **deep** = invest heavily, build interview-grade understanding; **moderate** = solid implementation, not the focus; **minimal** = just enough to demo the system end-to-end.

Detailed tasks for each phase live in their respective `phases/<phase>/` directory.

## Phase 1: Storage Engine (deep)
Read and write fixed-size pages to disk, cache them in memory. This is the core of the project — go beyond textbook and into production-grade territory.

**Milestone:** Can allocate pages, write arbitrary bytes, evict under pressure, and survive concurrent access — using direct I/O, io_uring for async submission, flash-aware allocation, and inline compression.

## Phase 2: Tuple Layout & Catalog (moderate)
Store and retrieve structured rows. Know what tables exist.

**Milestone:** Can insert a row into a named table and read it back by scanning the heap.

## Phase 3: Access Methods (moderate)
Find rows without full table scans.

**Milestone:** Index-accelerated lookups via B+Tree in O(log n).

## Phase 4: Execution Engine (minimal)
Execute queries via programmatic plans (no parser yet).

**Milestone:** Compose basic operators (scan, filter, projection, one join type) into a plan tree and get result tuples out.

## Phase 5: SQL Parser & Planner (minimal)
Accept SQL strings, produce and execute plans.

**Milestone:** Parse basic SELECT/INSERT/CREATE TABLE, bind against catalog, execute. No optimizer required.

## Phase 6: Concurrency Control (deep)
Multiple transactions running safely in parallel, with emphasis on high-performance synchronization primitives.

**Milestone:** Concurrent transactions with correct isolation, lock-free buffer pool internals, and criterion benchmarks proving throughput under contention.

## Phase 7: Recovery (moderate)
Crash and come back with all committed data intact.

**Milestone:** Kill the process mid-transaction, restart, and verify data integrity.

## Phase 8: Network Layer (minimal)
Clients connect over TCP and run SQL.

**Milestone:** A basic client can connect, send a SQL query, and receive results. Postgres wire protocol compatibility is not a goal.

## Phase 9: Replication & Consensus (deep)
Ship the WAL to replicas, elect leaders, and handle failures.

**Milestone:** Leader-follower replication with automatic failover via simplified Raft. Can lose the leader, elect a new one, and continue serving reads from a follower with no data loss for committed transactions.

## Phase 10: Snapshots & Data Reduction (deep)
Copy-on-write snapshots and inline data reduction — the features that define modern flash storage systems.

**Milestone:** Instant point-in-time snapshots via page-level CoW. Inline LZ4 compression with measurable space savings. Block-level deduplication.

## Stretch Goals (Post-MVP)
- Cost-based optimizer with statistics
- Parallel query execution
- VACUUM / garbage collection for MVCC
- Tiered storage simulation (hot/cold data placement)
- CSI (Container Storage Interface) plugin exposing volumes backed by PureDB's storage engine
