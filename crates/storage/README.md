# Phase 1: Storage Engine

The foundation layer and the project's primary focus. Manages how bytes are stored on and read from disk, caches pages in memory, and implements production-grade I/O techniques.

**Milestone:** Can allocate pages, write arbitrary bytes, evict under pressure, and survive concurrent access — using direct I/O, io_uring for async submission, flash-aware allocation, ARC eviction, and inline compression.

## Resources (whole phase)

- **CMU 15-445 (Andy Pavlo)** — The go-to database systems course. Lectures 3–6 cover the textbook portion of this phase.
  - Lecture 3: Database Storage I — disk-oriented architecture, pages, heap files
  - Lecture 4: Database Storage II — page layout, slotted pages, tuple layout
  - Lecture 5: Buffer Pools — buffer pool manager, eviction policies, dirty pages
  - Lecture 6: Hash Tables — useful background before B+Trees in Phase 3
  - YouTube playlist: search "CMU 15-445 Fall 2023"
  - Course projects (Bustub): the first project is literally "build a buffer pool manager"
- **"Database Internals" by Alex Petrov** — Chapters 1–5 cover storage engine fundamentals in depth. Good when you want more detail than a lecture gives.
- **Postgres source code** — `src/backend/storage/smgr/` (disk manager), `src/backend/storage/buffer/` (buffer pool). Dense but real.

## Sub-phases

### 1.1 — Disk Manager ✅
The simplest possible layer. A struct that owns a file and can read/write fixed-size byte arrays (pages) by page ID. No caching, no concurrency — just raw I/O.

- `allocate_page() → PageId`
- `read_page(PageId) → [u8; PAGE_SIZE]`
- `write_page(PageId, &[u8; PAGE_SIZE])`

**Test:** Allocate 10 pages, write data, close file, reopen, read them back.

**Resources:**
- CMU 15-445 Lecture 3 (Database Storage I) — covers how databases interact with disk
- Petrov Ch. 3 — file formats, page structure basics
- Rust: `std::fs::File`, `seek()`, `read_exact()`, `write_all()` — all you need here

### 1.2 — Page Layout
Define the binary format of a page. A slotted-page design:

- Page header: page ID, number of tuples, free space pointers
- Slot array growing forward, tuple data growing backward
- Helpers to insert/read/delete a variable-length byte slice within a page

**Test:** Insert entries into a page until full, read them back by slot index, delete one, compact.

**Resources:**
- CMU 15-445 Lecture 4 (Database Storage II) — slotted pages explained visually
- Petrov Ch. 3 — slotted page layout, cell pointers, free space management
- Postgres: `src/backend/access/heap/hio.c` — how Postgres manages heap page layout

### 1.3 — Buffer Pool Manager (single-threaded)
An in-memory cache of pages. Fixed number of frames. No thread safety yet.

- `fetch_page(PageId)` — return from cache or load from disk
- `new_page()` — allocate on disk and bring into the pool
- Pin/unpin semantics (a page can't be evicted while pinned)
- Eviction policy: start with LRU, upgrade to clock later
- Dirty flag: only write back pages that were modified

**Test:** Create a pool with 3 frames, fetch 5 different pages, verify eviction and write-back happen correctly.

**Resources:**
- CMU 15-445 Lecture 5 (Buffer Pools) — pin/unpin, dirty flags, eviction policies
- Petrov Ch. 5 — buffer management, page replacement algorithms
- LRU: straightforward with a `HashMap` + doubly-linked list (or use `LinkedHashMap`)
- Clock eviction: simpler to implement, better real-world performance — look it up after LRU works

### 1.4 — Buffer Pool Concurrency
Make the buffer pool safe for multiple threads. This is the first real concurrency challenge.

- Latch the page table (hash map from PageId → frame)
- Latch individual frames so two threads can work on different pages simultaneously
- Pin count becomes atomic

**Test:** Spawn N threads, each doing random fetch/unpin cycles, assert no data corruption.

**Resources:**
- CMU 15-445 Lecture 5 — discusses latching strategy for the buffer pool
- Rust: `Mutex`, `RwLock`, `Arc`, `AtomicU32` — these are your building blocks
- "Latches are not locks" — latches protect in-memory structures (short-held, no deadlock detection); locks protect logical data (transactions). This distinction matters a lot in Phase 6.

### 1.5 — Direct I/O
Bypass the OS page cache to avoid double-buffering. The buffer pool *is* the cache — the kernel's page cache just wastes memory and adds unpredictability.

- Open data files with `O_DIRECT` (requires aligned buffers and aligned offsets)
- Allocate page-aligned memory for I/O buffers (e.g., `posix_memalign` or Rust's `std::alloc::Layout` with alignment = PAGE_SIZE)
- All reads and writes must be page-size-aligned
- Benchmark: compare latency and throughput with and without `O_DIRECT`

**Test:** Write and read 10,000 pages via direct I/O. Verify data integrity. Run with `/proc/meminfo` checks to confirm minimal kernel page cache usage.

**Resources:**
- `open(2)` man page — `O_DIRECT` semantics and alignment requirements
- LWN article "O_DIRECT and the page cache" — tradeoffs and when it makes sense
- Rust: `std::os::unix::fs::OpenOptionsExt` for passing `O_DIRECT`, `std::alloc::alloc` with explicit layout for aligned buffers
- RocksDB source: `env/io_posix.cc` — real-world direct I/O usage patterns

### 1.6 — io_uring Integration
Replace blocking `pread`/`pwrite` with Linux's io_uring for async I/O submission. Enables high-throughput I/O without one thread per outstanding request.

- Set up an io_uring instance with a submission queue and completion queue
- Submit batches of read/write requests and poll for completions
- Integrate with the buffer pool: prefetch pages asynchronously, write back dirty pages in batches
- Handle partial completions and errors

**Test:** Submit 100 concurrent page reads via io_uring, verify all complete correctly. Benchmark vs synchronous `pread` at high queue depths.

**Resources:**
- `io_uring(7)` man page + "Lord of the io_uring" guide (unixism.net)
- `io-uring` crate — safe Rust bindings (thin wrapper over the kernel interface)
- Jens Axboe's liburing examples — canonical usage patterns
- TigerBeetle source — a database that uses io_uring extensively for storage I/O

### 1.7 — Flash-Aware Page Allocation
SSDs have write amplification and wear-leveling concerns. In-place page updates are wasteful — a log-structured or copy-on-write allocation strategy reduces write amplification.

**Important:** Design the page mapping table (logical PageId → physical location) with Phase 10 (CoW snapshots, dedup) in mind from the start. Snapshots need to freeze a mapping and keep old physical pages alive; dedup needs to map multiple logical pages to one physical page. If the mapping supports reference counting and versioning now, Phase 10 becomes an extension rather than a rewrite.

- Implement a log-structured allocator: new/modified pages are always appended, never overwritten in place
- Maintain a page mapping table (logical PageId → physical location on disk). Include a reference count per physical page — even if only the active state uses it for now, snapshots and dedup will need it later.
- Garbage collection: reclaim space from stale page versions when refcount drops to zero
- Track write amplification factor (bytes written to SSD / bytes of actual data) as a metric

**Test:** Write 10,000 pages, update 5,000 of them, measure write amplification. Run GC, verify all live pages are still accessible and stale space is reclaimed.

**Resources:**
- "The Design of a Log-Structured File System" (Rosenblum & Ousterhout, 1992) — the foundational paper
- Petrov Ch. 7 — log-structured storage, compaction strategies
- "F2FS: A New File System for Flash Storage" — real flash-aware design decisions
- RocksDB / LevelDB documentation on LSM-tree compaction — same principles at a different layer

### 1.8 — ARC Eviction Policy
Replace LRU with ARC (Adaptive Replacement Cache), which automatically tunes between recency and frequency to adapt to changing workloads.

- Maintain four lists: T1 (recent), T2 (frequent), B1 (ghost entries evicted from T1), B2 (ghost entries evicted from T2)
- On cache hit in T1: promote to T2. On hit in B1: increase T1 target size. On hit in B2: increase T2 target size.
- Dynamically balances between scan-resistant and recency-biased behavior
- Benchmark against LRU and Clock on different workloads (sequential scan, Zipfian, mixed)

**Test:** Run a workload that mixes sequential scans with hot-key lookups. Verify ARC achieves higher hit rate than LRU. Test adaptation: start with scan-heavy, switch to point-lookup-heavy, verify ARC adjusts.

**Resources:**
- "ARC: A Self-Tuning, Low Overhead Replacement Cache" (Megiddo & Modha, 2003) — the original IBM paper. Clear algorithm description.
- ZFS source code — ZFS uses ARC as its primary cache eviction policy
- PostgreSQL `src/backend/storage/buffer/freelist.c` — Postgres uses Clock, but the contrast is instructive

### 1.9 — Inline Compression
Compress pages transparently before writing to disk. Layers above see uncompressed pages; the disk manager handles compression/decompression.

- Compress pages with LZ4 (fast) before writing, decompress after reading
- Store compressed size in a page header or mapping table — compressed pages may be smaller than PAGE_SIZE
- Handle incompressible pages gracefully (store uncompressed, set a flag)
- Track compression ratio as a metric
- Optional: support zstd as a higher-ratio alternative for cold data

**Test:** Write 1000 pages of compressible data (e.g., repeated patterns), verify compression ratio is significant. Write incompressible data (random bytes), verify it falls back to uncompressed. Read all pages back, verify byte-for-byte equality with originals.

**Resources:**
- `lz4` and `zstd` Rust crates — bindings to the C libraries, well-maintained
- ZFS block-level compression — design reference for transparent compression in a storage stack
- Facebook's zstd documentation — compression levels, dictionary compression, benchmarks
