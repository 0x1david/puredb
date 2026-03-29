# PureDB Roadmap

Build order is bottom-up: storage first, SQL last. Each phase produces a working, testable artifact.

Detailed tasks for each phase live in their respective `phases/<phase>/` directory.

## Phase 1: Storage Engine
Read and write fixed-size pages to disk, cache them in memory.

**Milestone:** Can allocate pages, write arbitrary bytes, evict under pressure, and survive concurrent access.

## Phase 2: Tuple Layout & Catalog
Store and retrieve structured rows. Know what tables exist.

**Milestone:** Can insert a row into a named table and read it back by scanning the heap.

## Phase 3: Access Methods
Find rows without full table scans.

**Milestone:** Index-accelerated lookups via B+Tree in O(log n).

## Phase 4: Execution Engine
Execute queries via programmatic plans (no parser yet).

**Milestone:** Compose operators into a plan tree and get result tuples out.

## Phase 5: SQL Parser & Planner
Accept SQL strings, produce and execute plans.

**Milestone:** Full SQL round-trip: parse → plan → execute → return rows.

## Phase 6: Concurrency Control
Multiple transactions running safely in parallel.

**Milestone:** Concurrent transactions with correct isolation. Can demo anomalies being prevented.

## Phase 7: Recovery
Crash and come back with all committed data intact.

**Milestone:** Kill the process mid-transaction, restart, and verify data integrity.

## Phase 8: Network Layer
Clients connect over TCP and run SQL.

**Milestone:** Connect with `psql` or a custom client and run queries over the network.

## Stretch Goals (Post-MVP)
- Query caching / prepared statements
- Cost-based optimizer with statistics
- Parallel query execution
- VACUUM / garbage collection for MVCC
- Replication (WAL shipping)
