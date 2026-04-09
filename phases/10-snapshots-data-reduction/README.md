# Phase 10: Snapshots & Data Reduction (deep)

Copy-on-write snapshots and inline data reduction at the storage layer. These are the features that differentiate a toy storage engine from one that demonstrates real storage-systems thinking.

**Milestone:** Instant point-in-time snapshots via page-level CoW. Inline LZ4 compression with measurable space savings. Block-level deduplication with reference counting.

## Resources (whole phase)

- **ZFS source code & documentation** — ZFS pioneered CoW snapshots, inline compression, and dedup in a single storage stack. The design docs are the canonical reference.
- **Btrfs wiki** — another CoW filesystem with snapshots and compression. Good for comparing design tradeoffs with ZFS.
- **"File Systems: Fundamentals and Design" (lectures by Remzi Arpaci-Dusseau)** — covers CoW and journaling file systems.
- **"Database Internals" by Alex Petrov** — Ch. 7 covers log-structured storage which shares CoW principles.

## Sub-phases

### 10.1 — Copy-on-Write Page Management
Extend the CoW allocation model from Phase 1.7 to support snapshots. If Phase 1.7 was implemented correctly (page mapping table with reference counting, no in-place overwrites), this sub-phase is mostly wiring — not a rewrite.

- On page modification: allocate a new physical page, write the modified data there, update the page mapping table to point to the new location
- The old physical page remains untouched (still referenced by any existing snapshots)
- Page mapping table: logical PageId → physical disk offset, version
- Reference counting on physical pages: a page is reclaimable when no snapshot or active state references it
- Integrate with the buffer pool: dirty page flush now means "write to new location + update mapping" instead of "write in place"

**Test:** Modify a page 10 times. Verify 10 distinct physical pages were written. Verify the mapping table points to the latest version. Verify old versions are still readable by physical offset.

**Resources:**
- ZFS block pointer design — each block pointer includes physical address, checksum, compression info
- Btrfs CoW B-tree — how Btrfs applies CoW to its metadata tree
- "The Design and Implementation of a Log-Structured File System" — CoW shares the "never overwrite" principle

### 10.2 — Snapshot Create & Restore
Leverage CoW to create instant, zero-copy snapshots.

- `create_snapshot(name) -> SnapshotId` — freeze the current page mapping table (just copy or ref-count the mapping, not the data)
- `list_snapshots() -> Vec<SnapshotInfo>` — name, creation time, size estimate
- `read_snapshot(SnapshotId, PageId) -> Page` — read a page as it existed at snapshot time by following the frozen mapping
- `delete_snapshot(SnapshotId)` — decrement reference counts on physical pages; reclaim pages whose refcount drops to zero
- `restore_snapshot(SnapshotId)` — replace the active mapping with the snapshot's mapping (effectively a rollback)
- Snapshot creation should be O(1) or O(mapping table size), not O(data size)

**Test:** Insert 1000 rows. Create snapshot S1. Insert 500 more rows, update 200 existing rows. Create snapshot S2. Verify S1 reads show original 1000 rows. Verify S2 reads show 1500 rows with updates. Delete S1, verify its unique pages are reclaimed but pages shared with S2 are not.

**Resources:**
- ZFS `zfs snapshot` internals — how ZFS implements O(1) snapshot creation
- LVM thin provisioning snapshots — similar CoW mechanics at the block device layer
- WAFL (NetApp) — Write Anywhere File Layout, a CoW-based storage architecture

### 10.3 — Block-Level Deduplication
Identify and eliminate duplicate page contents using content-addressable storage.

- Compute a hash (SHA-256 or xxHash) of each page's content before writing
- Dedup table: maps content hash → physical page offset + reference count
- On write: if the hash already exists in the dedup table, increment the refcount and point the mapping to the existing physical page instead of writing a new one
- On delete/overwrite: decrement the refcount; reclaim physical page when it reaches zero
- Track dedup ratio (logical pages / physical pages) as a metric
- Handle hash collisions: either verify byte-for-byte on match (safe) or accept the astronomically small risk (xxHash/SHA-256)

**Test:** Write 1000 pages where 400 are duplicates (100 unique patterns × 4 copies each). Verify only ~700 physical pages are written (600 unique + 100 unique patterns for the 400 dupes). Delete some logical pages, verify refcounts decrement correctly and pages are reclaimed when refcount hits zero.

**Resources:**
- ZFS dedup design — content-addressable blocks, dedup tables (DDT), memory overhead tradeoffs
- "A Study of Practical Deduplication" (Meyer & Bolosky, 2012) — real-world dedup ratios and design tradeoffs
- xxHash — extremely fast non-cryptographic hash, suitable for dedup (collision-resistant enough in practice)
- Rust: `sha2` crate for SHA-256, `xxhash-rust` for xxHash

### 10.4 — Compression Integration
If inline compression wasn't already implemented in Phase 1.9, implement it here. If it was, this sub-phase focuses on integration with CoW and dedup.

- Compress pages before dedup hashing (compress-then-dedup, since compressed representations of identical data may differ — alternatively, dedup-then-compress for better dedup ratios at the cost of more hashing)
- Decide on ordering: dedup raw → compress physical pages (better dedup) vs compress → dedup compressed (simpler)
- Track combined data reduction ratio: (logical size) / (physical size after compression + dedup)
- Benchmark: measure end-to-end write throughput with compression + dedup enabled vs disabled

**Test:** Write 1000 pages of compressible, partially-duplicate data. Measure combined reduction ratio. Verify all data reads back correctly after round-tripping through compress + dedup.

**Resources:**
- ZFS data reduction pipeline — compress → checksum → dedup (ZFS order)
- LZ4 frame format — understand block vs frame compression for page-level use
