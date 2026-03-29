# Design Decisions

Fundamental trade-offs that affect the entire system. Each decision is recorded here with the options, what real databases chose, and PureDB's choice.

Format: **Undecided** choices have an empty "PureDB" column — fill them in before starting the relevant phase.

---

## Storage Layer (Phase 1)

### Page Size

| Option | Used by | Trade-off |
|--------|---------|-----------|
| 4 KB | SQLite | Matches OS page size on many systems. Less wasted space for small rows. More I/O ops for large scans. |
| 8 KB | Postgres | Good balance. Fits ~100–400 rows depending on width. What CMU 15-445 assumes. |
| 16 KB | MySQL/InnoDB | Fewer I/O ops for large scans. More wasted space. Better for wide rows. |
| 64 KB | SQL Server (optional) | Extreme end. Good for analytics workloads. |

**PureDB choice:**

**Why it matters:** Baked into every page read/write, every buffer pool frame, every B+Tree node. Changing this later means reformatting every file on disk.

---

### Byte Order (Endianness)

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Little-endian | Most modern systems | Native on x86/ARM — no conversion overhead. Can't memcmp integers for sort order. |
| Big-endian (network order) | Java DBs, some wire protocols | Memcmp gives correct sort order for unsigned integers. Requires byte-swapping on x86/ARM. |
| Native endian | Postgres | Fastest — no conversion at all. Files aren't portable across architectures (Postgres doesn't care). |

**PureDB choice:**

**Why it matters:** Every integer serialized to disk or sent over the wire uses this. Changing it means a full data migration.

---

### Page ID Size

| Option | Max addressable | Trade-off |
|--------|-----------------|-----------|
| `u32` | ~32 TB at 8KB pages | Compact pointers (4 bytes in B+Tree nodes, tuple IDs). Enough for any learning project. |
| `u64` | Effectively unlimited | Future-proof but doubles pointer sizes everywhere — B+Tree fan-out drops, tuple IDs grow. |

**PureDB choice:**

**Why it matters:** Shows up in B+Tree child pointers, tuple IDs, free space maps, WAL records. Changing the size means changing every on-disk structure.

---

### Page Layout Strategy

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Slotted pages | Postgres, most OLTP systems | Slot array + tuple data. Supports variable-length rows. Tuples can be moved within the page without changing external references (slot indirection). |
| Log-structured | RocksDB, LevelDB, Cassandra | Append-only writes, periodic compaction. Great write throughput but reads require merging. Fundamentally different architecture. |
| PAX (hybrid columnar) | Vertica, some analytics systems | Column groups within a page. Better cache behavior for scans. More complex layout. |

**PureDB choice:**

**Why it matters:** Determines how tuples are stored, found, moved, and compacted. The most fundamental on-disk structure.

---

### Free Space Tracking

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Free Space Map (FSM) | Postgres | Separate structure tracking approximate free space per page. O(1) to find a page with space. Extra bookkeeping. |
| Linear scan | Simple implementations | Scan pages until you find one with room. Simple but O(N) for inserts. |
| Free list | Some systems | Linked list of pages with free space. Fast but fragile (list corruption = lost pages). |

**PureDB choice:**

**Why it matters:** Determines insert performance. A bad choice here means inserts get slower as the table grows.

---

## Tuple Format (Phase 2)

### Tuple ID (Record ID) Format

| Option | Size | Trade-off |
|--------|------|-----------|
| `(u32 PageId, u16 SlotIndex)` | 6 bytes | Compact. 65K slots per page is far more than you'd ever fit in 8KB. |
| `(u32 PageId, u32 SlotIndex)` | 8 bytes | Aligned to 8 bytes (nicer for some architectures). Wastes 2 bytes per reference. |
| `u64` logical row ID | 8 bytes | No physical location encoded — requires a lookup index. More flexible but slower. |

**PureDB choice:**

**Why it matters:** Every index entry, every join intermediate, every WAL record that references a row uses this. It's the "pointer" type of your database.

---

### Null Representation

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Null bitmap | Postgres, MySQL | One bit per column at the start of the tuple. Space-efficient. Doesn't consume valid values. |
| Sentinel values | Some embedded DBs | Use a special value per type (e.g., `i32::MIN` = null). No bitmap overhead. But you lose a valid value from the domain, and each type needs its own sentinel. |
| Optional wrapper | ORM-style | Each field wrapped in Option-like structure. Wastes space for the tag byte per field. |

**PureDB choice:**

**Why it matters:** Affects tuple serialization, deserialization, comparison operators, and index key encoding.

---

### Null Bitmap Position

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Before all fields | Postgres | Can skip reading null fields entirely during deserialization — know which are null before reading any data. |
| After fixed-length fields | Some systems | Fixed fields are at known offsets without consulting the bitmap. Bitmap is at a known offset too. |

**PureDB choice:**

**Why it matters:** Determines how you parse a tuple. Bitmap-first is simpler for deserialization.

---

### String Encoding

| Option | Used by | Trade-off |
|--------|---------|-----------|
| UTF-8 | Postgres, SQLite | Variable-width. Rust strings are already UTF-8. Can't store arbitrary binary data. |
| Latin-1 / ASCII | Older systems | Fixed 1 byte per char. Simple but limited character set. |
| UTF-16 | SQL Server | Fixed 2 bytes for BMP characters. Wastes space for ASCII-heavy data. |

**PureDB choice:**

**Why it matters:** Affects storage size, comparison semantics, and whether you need a separate BYTEA/BLOB type.

---

### Variable-Length Field Storage

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Inline with offset array | Postgres (for small values) | Offset array at tuple start points into trailing data region. Fast access. Tuple size limited to page size. |
| TOAST (out-of-line) | Postgres (for large values) | Values exceeding a threshold are stored in a separate overflow table. Keeps main tuples compact. Adds complexity. |
| Pointer to overflow page | Oracle | Similar to TOAST but at the page level. |

**PureDB choice:**

**Why it matters:** Determines max row size and whether large TEXT/BLOB values bloat the heap.

---

## Indexing (Phase 3)

### Duplicate Key Handling in B+Tree

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Allow duplicates in leaves | Postgres | Simpler insertion. Leaf can have many entries with the same key. Range scan naturally returns all matches. |
| Append tuple ID to key | Some systems | Makes every key unique. Simplifies split logic. Index is slightly larger. |
| Overflow pages for duplicates | Older systems | Leaf entry points to a chain of overflow pages. Complex and slow for high-cardinality duplicates. |

**PureDB choice:**

**Why it matters:** Affects B+Tree insertion, deletion, and search logic. Hard to change once implemented.

---

### B+Tree Key Format

| Option | Trade-off |
|--------|-----------|
| Fixed-size keys only | Simple node layout, easy binary search. Can't index VARCHAR columns without padding. |
| Variable-size keys with prefix | Flexible but node layout is more complex. Need offset arrays within nodes (like slotted pages). |
| Prefix truncation in internal nodes | Internal nodes only need enough of the key to route — can truncate, saving space and increasing fan-out. More complex but significant space savings. |

**PureDB choice:**

**Why it matters:** Determines B+Tree fan-out (keys per node), which directly affects tree height and lookup performance.

---

### Concurrent B+Tree Protocol

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Latch crabbing | Textbook | Acquire child latch before releasing parent (if safe). Simple, correct. Can bottleneck on root. |
| Optimistic latch coupling | Modern systems | Take shared latches down the tree, only exclusive at the leaf. Restart if a split happens. Higher throughput for read-heavy workloads. |
| Blink-tree | Postgres | Right-link pointers on every node. Allows concurrent splits without holding parent latches. More complex but very concurrent. |

**PureDB choice:**

**Why it matters:** Determines how much concurrency the index supports. Latch crabbing is fine for learning; Blink-tree is what production systems use.

---

## Execution Engine (Phase 4)

### Query Processing Model

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Volcano (iterator / pull) | Postgres, most traditional DBs | One tuple at a time. `next()` pulls from children. Simple. Poor cache locality. High per-tuple overhead from virtual calls. |
| Materialization (push) | Some embedded systems | Each operator processes entire input, materializes output. Simple but high memory usage for large intermediates. |
| Vectorized | DuckDB, Velox, ClickHouse | Processes batches of tuples (vectors of ~1024). Best of both worlds — low per-tuple overhead, good cache locality. More complex to implement. |

**PureDB choice:**

**Why it matters:** Affects every operator implementation. Switching from Volcano to vectorized later is essentially a rewrite of the execution layer.

---

### Expression Representation

| Option | Trade-off |
|--------|-----------|
| Closures / `Box<dyn Fn>` | Easy to write in Rust. Can't serialize, inspect, or optimize. Fine for hardcoded plans. |
| Expression AST enum | Serializable, inspectable, optimizable (constant folding, etc). More boilerplate. Required once you have a planner. |

**PureDB choice:**

**Why it matters:** Closures work for Phase 4 (hardcoded plans) but you'll need an AST for Phase 5 (planner). Decide whether to start with closures and migrate, or go straight to AST.

---

### Join Ordering

| Option | Trade-off |
|--------|-----------|
| Left-deep trees only | Simpler planner. Inner side is always a base table (can be an index scan). Used by most rule-based planners. |
| Bushy trees | More plan options, potentially better plans. Exponentially more options to search. Needs a cost-based optimizer to be useful. |

**PureDB choice:**

**Why it matters:** Constrains the planner's search space. Left-deep is sufficient until you have statistics and a cost model.

---

## Concurrency Control (Phase 6)

### Concurrency Control Scheme

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Two-Phase Locking (2PL) | MySQL/InnoDB (for serializable) | Locks rows. Readers block writers and vice versa. Simple correctness argument. Can deadlock. |
| MVCC | Postgres, Oracle, MySQL (default) | Readers never block writers. Multiple versions of each row. More complex but much higher throughput. |
| Optimistic (OCC) | VoltDB, some in-memory DBs | No locks during execution — validate at commit time. Great for low-contention workloads. Lots of aborts under contention. |

**PureDB choice:**

**Why it matters:** The most fundamental architectural decision for concurrent access. Affects tuple format (MVCC needs version info), read paths, write paths, and recovery.

---

### MVCC Version Storage Scheme

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Append-only (new versions in heap) | Postgres | New version is a new tuple. Old version stays in the same table. Simple. Table bloats without VACUUM. |
| Delta storage (undo log) | MySQL/InnoDB, Oracle | Main table always has the latest version. Undo log stores diffs to reconstruct old versions. Compact main table. More complex reads of old versions. |
| Separate version store | SQL Server (tempdb) | Versions stored in a dedicated area. Main table stays clean. Extra I/O for version access. |

**PureDB choice:**

**Why it matters:** Affects tuple layout (needs xmin/xmax or version pointer), table scan performance (bloat vs. clean), and whether you need a garbage collector (VACUUM).

---

### Lock Granularity

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Row-level | Postgres, MySQL/InnoDB | Maximum concurrency. More lock overhead (one lock entry per locked row). |
| Page-level | Older systems | Less lock overhead. More contention (locking an entire page blocks unrelated rows). |
| Table-level | SQLite (in WAL mode, one writer at a time) | Simplest. Serializes all writes to a table. |
| Intention locks (hierarchical) | Most production systems | Table-level intention lock + row-level actual lock. Allows table-wide operations (DDL) to check for conflicts without scanning every row lock. |

**PureDB choice:**

**Why it matters:** Determines maximum concurrency and lock manager complexity.

---

### Deadlock Handling

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Wait-for graph + detection | Postgres | Build a graph of who-waits-for-whom, run cycle detection. Accurate. Overhead of maintaining the graph. |
| Timeout | Simple systems | If a lock wait exceeds N ms, abort. Simple but either too aggressive (false aborts) or too slow (long waits). |
| Wait-die / Wound-wait | Some distributed systems | Prevention based on transaction age. No graph needed. More unnecessary aborts than detection. |

**PureDB choice:**

**Why it matters:** Determines how the system recovers from deadlocks. Detection is more precise; prevention is simpler.

---

## Recovery (Phase 7)

### WAL Record Type

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Physical (before/after page images) | ARIES, Postgres | Log the exact bytes changed. Redo/undo is mechanical. Larger log records. |
| Logical (operation log) | Some systems | Log the operation ("insert row X into table Y"). Smaller records. Redo must re-execute logic — fragile if code changes. |
| Physiological (physical to a page, logical within) | Postgres (mostly) | Log "on page P, do operation X". Best of both — compact and idempotent. The standard modern approach. |

**PureDB choice:**

**Why it matters:** Determines WAL record size, redo complexity, and whether recovery depends on application logic.

---

### Buffer Pool Flush Policy

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Steal / No-Force | Postgres, most systems | Dirty pages can be flushed before commit (steal). Pages don't have to be flushed at commit (no-force). Requires undo (for stolen uncommitted pages) and redo (for committed but unflushed pages). Maximum flexibility. |
| No-Steal / Force | Simple systems, some in-memory DBs | Never flush uncommitted data. Always flush at commit. No undo or redo needed. Simple but terrible performance (commit must wait for all I/O). |

**PureDB choice:**

**Why it matters:** Determines whether you need undo, redo, both, or neither in your recovery algorithm. Steal/No-Force requires both (ARIES), but gives the best performance.

---

### Checkpoint Strategy

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Sharp checkpoint | Simple systems | Halt all transactions, flush all dirty pages, write checkpoint record. Simple but causes a pause. |
| Fuzzy checkpoint | Postgres, most production systems | Snapshot the dirty page table and active transaction table without halting. Some dirty pages may not be flushed yet — redo handles them. No pause. |

**PureDB choice:**

**Why it matters:** Sharp checkpoints are simpler but cause latency spikes. Fuzzy checkpoints are what real systems use.

---

## Network Layer (Phase 8)

### Wire Protocol

| Option | Trade-off |
|--------|-----------|
| Postgres protocol | Can use `psql`, `pgcli`, any libpq driver, ORMs, connection pools out of the box. Well-documented. More work to implement correctly. |
| Custom protocol | Simpler to build. But you need to write your own client too. No ecosystem support. |
| MySQL protocol | Similar ecosystem benefits to Postgres. Less well-documented. |

**PureDB choice:**

**Why it matters:** Determines whether existing tools (psql, drivers, ORMs) work with PureDB or you need custom clients.

---

### Connection Model

| Option | Used by | Trade-off |
|--------|---------|-----------|
| Thread-per-connection | Postgres | Simple. Each connection gets a dedicated OS thread. Doesn't scale past ~1000 connections. |
| Async (event loop) | Node.js-style, some proxies | Single thread handles many connections. Scales to 10K+ connections. Harder to write (async/await). |
| Thread pool | MySQL, most modern systems | Fixed pool of worker threads. Connections are multiplexed onto workers. Good balance. |

**PureDB choice:**

**Why it matters:** Determines how many concurrent connections the database can handle and how the execution engine interacts with I/O.

---

## How to Use This Document

1. Before starting a phase, read through the decisions for that phase
2. Research the trade-offs (the phase README has resources)
3. Fill in the "PureDB choice" with your decision and a brief reason
4. If you change your mind later, update the choice and note why — that's part of the learning
