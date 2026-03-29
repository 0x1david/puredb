# Phase 1: Storage Engine

The foundation layer. Manages how bytes are stored on and read from disk, and caches pages in memory.

**Milestone:** Can allocate pages, write arbitrary bytes, evict under pressure, and survive concurrent access.

## Resources (whole phase)

- **CMU 15-445 (Andy Pavlo)** — The go-to database systems course. Lectures 3–6 cover this entire phase.
  - Lecture 3: Database Storage I — disk-oriented architecture, pages, heap files
  - Lecture 4: Database Storage II — page layout, slotted pages, tuple layout
  - Lecture 5: Buffer Pools — buffer pool manager, eviction policies, dirty pages
  - Lecture 6: Hash Tables — useful background before B+Trees in Phase 3
  - YouTube playlist: search "CMU 15-445 Fall 2023"
  - Course projects (Bustub): the first project is literally "build a buffer pool manager"
- **"Database Internals" by Alex Petrov** — Chapters 1–5 cover storage engine fundamentals in depth. Good when you want more detail than a lecture gives.
- **Postgres source code** — `src/backend/storage/smgr/` (disk manager), `src/backend/storage/buffer/` (buffer pool). Dense but real.

## Sub-phases

### 1.1 — Disk Manager
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
