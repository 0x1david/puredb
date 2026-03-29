# Phase 3: Access Methods

Index structures that speed up lookups beyond sequential scan.

**Milestone:** Can create a B+Tree index on a table column, use it for point lookups and range scans, and handle concurrent readers and writers safely.

## Resources (whole phase)

- **CMU 15-445 (Andy Pavlo)** — Lectures 7–8 cover tree indexes and B+Trees in detail.
  - Lecture 7: Tree Indexes I — B+Tree structure, search, insertions
  - Lecture 8: Tree Indexes II — deletions, concurrency control (latch crabbing)
  - YouTube playlist: search "CMU 15-445 Fall 2023"
  - Course projects (Bustub): Project 2 is "build a B+Tree index"
- **"Database Internals" by Alex Petrov** — The most thorough treatment of on-disk B-Trees you'll find.
  - Ch. 2: B-Tree Basics — structure, invariants, search/insert/delete algorithms
  - Ch. 4: Implementing B-Trees — page layout for nodes, overflow pages, on-disk considerations
  - Ch. 6: B-Tree Variants — concurrency, optimistic locking, Blink-trees
- **Postgres source code** — `src/backend/access/nbtree/` (B-Tree implementation), `src/include/access/nbtree.h` (node format). This is a production B+Tree — good for checking your design against reality.

## Sub-phases

### 3.1 — B+Tree Node Format

Define the binary layout for B+Tree nodes, stored in pages from the buffer pool. Two node types:

- Internal node: header (node type, key count, level) + array of keys + array of child `PageId` pointers (one more pointer than keys)
- Leaf node: header + array of keys + array of record IDs (`PageId` + slot index), plus a `next_leaf` pointer for range scans
- Serialize/deserialize nodes to/from `[u8; PAGE_SIZE]` — reuse the page abstraction from Phase 1
- Key type: start with fixed-size (e.g., `i64`), parameterize later

**Test:** Create internal and leaf nodes, fill them with keys, serialize to a page, deserialize back, verify all fields round-trip.

**Resources:**
- CMU 15-445 Lecture 7 — covers node layout, difference between internal and leaf nodes
- Petrov Ch. 4 — detailed on-disk node format, cell layout, pointer management
- Rust: `byteorder` crate or manual `from_le_bytes` / `to_le_bytes` for encoding — same approach as Phase 1 page layout

### 3.2 — B+Tree Search & Point Lookup

Traverse the tree from root to leaf, then binary search within the leaf to find a key.

- `search(key) -> Option<RecordId>` — follow internal node pointers downward, binary search at each level
- Store the root `PageId` in a metadata page or header
- All page access goes through the buffer pool — fetch, read, unpin
- Handle the empty-tree case

**Test:** Build a small tree by hand (manually construct and write nodes to pages), then search for existing and missing keys.

**Resources:**
- CMU 15-445 Lecture 7 — B+Tree search walkthrough
- Petrov Ch. 2 — search algorithm, key comparison, pointer traversal
- Rust: `slice::binary_search()` or `partition_point()` — built-in and correct

### 3.3 — B+Tree Insertion & Splits

Insert keys into the tree, splitting nodes when they overflow.

- `insert(key, record_id)` — find the correct leaf, insert in sorted order
- Leaf split: when a leaf is full, allocate a new leaf, redistribute keys, push the middle key up to the parent
- Internal split: same idea — redistribute keys/pointers, push a key up
- Root split: allocate a new root, increasing tree height by one
- Handle duplicate keys (decide: allow or reject)

**Test:** Insert 1000 sequential and 1000 random keys, verify every key is findable via search. Check that tree height grows as expected.

**Resources:**
- CMU 15-445 Lecture 7 — insertion algorithm, split animations
- Petrov Ch. 2, 4 — split strategies, handling parent propagation
- Postgres: `_bt_insertonpg()` in `nbtinsert.c` — real insertion logic with split handling

### 3.4 — B+Tree Deletion & Merges

Delete keys and maintain B+Tree invariants (minimum occupancy).

- `delete(key)` — find the leaf, remove the key
- Underflow handling: if a node drops below minimum occupancy, try to redistribute with a sibling
- If redistribution fails, merge two siblings and remove a key from the parent
- Recursive merge: a parent underflow triggers the same logic upward
- Lazy deletion alternative: mark as deleted, defer merge — simpler, good enough for many real systems

**Test:** Insert 1000 keys, delete 800 in random order, verify remaining 200 are intact. Check that tree height shrinks when appropriate.

**Resources:**
- CMU 15-445 Lecture 8 — deletion, merge, redistribution
- Petrov Ch. 2 — delete algorithm, rebalancing strategies
- Note: many production databases (including Postgres) use lazy deletion. Implement full merge first for the learning, then consider lazy as an optimization.

### 3.5 — B+Tree Concurrent Access

Make the B+Tree safe for multiple threads using latch crabbing (lock coupling).

- Basic latch crabbing: acquire latch on child before releasing latch on parent — but only release parent if child is "safe" (won't split/merge)
- Read path: acquire shared latches going down, release parent once child is latched
- Write path: acquire exclusive latches going down, release ancestors once a safe node is found
- Optimistic approach: assume most inserts won't cause splits — take shared latches on the way down, upgrade to exclusive only at the leaf, restart if a split is needed

**Test:** Spawn N reader threads and M writer threads doing concurrent inserts, deletes, and lookups. Assert no lost keys and no corrupted nodes.

**Resources:**
- CMU 15-445 Lecture 8 — latch crabbing explained step by step
- Petrov Ch. 6 — lock coupling, Blink-trees, optimistic concurrency
- Rust: `RwLock` per node/page. The buffer pool already handles frame-level latching — coordinate with that, don't double-lock.

### 3.6 — Index Integration

**Deferred to Phase 5.5.** Integrating the B+Tree into SQL (parsing `CREATE INDEX`, planner index scan selection, catalog metadata) requires the parser, planner, and execution engine from Phases 4–5. The work is covered in Phase 5.5 (Basic Optimizer) and the `CREATE INDEX` DDL in Phase 5.2.

What remains in this phase is the access-layer API: the B+Tree exposes `search`, `insert`, `delete`, and a range-scan iterator. Phase 5 wires those into the query pipeline.
