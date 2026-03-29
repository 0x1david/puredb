# Phase 7: Recovery

Ensuring committed data survives crashes using write-ahead logging and the ARIES recovery algorithm.

**Milestone:** Can crash at any point, restart, and guarantee that committed transactions are durable and uncommitted transactions are rolled back.

## Resources (whole phase)

- **CMU 15-445 (Andy Pavlo)** — Lectures 19–20 cover recovery and ARIES end-to-end. These are the best starting point — the ARIES paper alone is brutal without this context.
  - Lecture 19: Database Recovery — WAL protocol, steal/no-force, redo vs undo
  - Lecture 20: ARIES — the three-pass algorithm, CLRs, fuzzy checkpoints
  - YouTube playlist: search "CMU 15-445 Fall 2023"
- **"Database Internals" by Alex Petrov** — Chapters 11–13 cover write-ahead logging, recovery techniques, and log-structured approaches. Fills in details the lectures skim.
- **"ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollbacks Using Write-Ahead Logging" (Mohan et al., 1992)** — The original paper. Read it after watching the CMU lectures, not before. Focus on Sections 1–9; the rest covers extensions.
- **Postgres source code** — `src/backend/access/transam/xlog.c` (WAL core), `src/backend/access/transam/xlogrecovery.c` (recovery), `src/backend/postmaster/checkpointer.c` (checkpointing). Dense but demonstrates real production choices.

## Sub-phases

### 7.1 — WAL Infrastructure

Define the log record format and build an append-only log file. No integration with the rest of the system yet — just the ability to serialize, write, and read log entries.

- Log record fields: LSN (monotonically increasing), txn_id, record type (begin, update, commit, abort, checkpoint, CLR), before-image, after-image
- Append-only log writer: serialize records and flush to a log file
- Log reader/iterator: scan forward from any LSN
- LSN type: a simple `u64` representing the byte offset into the log file

**Test:** Write 100 log records, close the file, reopen, iterate from the start, verify all records are intact and in order.

**Resources:**
- CMU 15-445 Lecture 19 (Database Recovery) — WAL record structure, log sequence numbers
- Petrov Ch. 11 — write-ahead log structure, log record anatomy
- Rust: `BufWriter` with explicit `flush()` for durability, `serde` or hand-rolled serialization for records — keep the format simple (length-prefixed or fixed-size)

### 7.2 — WAL Integration

Wire the WAL into the buffer pool and transaction operations. The key invariant: a dirty page cannot be flushed to disk until the log records describing its changes have been flushed first (the WAL protocol).

- Before any page modification, write a WAL record with before/after images
- Track `page_lsn` on each page (the LSN of the last log record that modified it)
- Track `flushed_lsn` on the log manager (how far the log has been durably written)
- Buffer pool flush check: assert `page_lsn <= flushed_lsn` before writing a dirty page to disk
- On commit: force all log records for the transaction to disk before acknowledging

**Test:** Modify pages through the buffer pool, verify WAL records appear on disk before the pages do. Simulate a crash (kill without flush), recover, confirm the WAL contains the expected records.

**Resources:**
- CMU 15-445 Lecture 19 — steal/no-force policies, WAL protocol, group commit
- Petrov Ch. 11–12 — WAL integration with buffer management
- Postgres: `src/backend/storage/buffer/bufmgr.c` — look at `FlushBuffer` for the page_lsn vs flushed_lsn check

### 7.3 — ARIES Redo

The first pass of recovery. On startup after a crash, replay the log forward from the last checkpoint to restore the database to its exact pre-crash state (including changes from uncommitted transactions).

- Build the analysis pass: scan the log from the last checkpoint to reconstruct the dirty page table (DPT) and active transaction table (ATT)
- Build the redo pass: scan forward from the smallest `recLSN` in the DPT, reapply log records where `page_lsn < record_lsn`
- Redo is unconditional — reapply even if the transaction later aborted (undo handles that)
- After redo, every page is in the exact state it was in before the crash

**Test:** Begin 3 transactions, commit 2, crash before the third commits. Recover, verify the two committed transactions' data is present and correct.

**Resources:**
- CMU 15-445 Lecture 20 (ARIES) — analysis and redo passes explained step by step
- Petrov Ch. 12 — redo recovery, idempotent replay
- ARIES paper Sections 7–8 — redo protocol details (read after watching the lecture)

### 7.4 — ARIES Undo

The second pass of recovery. Roll back any transactions that were active at crash time using the before-images stored in the log.

- Walk backward through the log for each uncommitted transaction
- Restore before-images to undo uncommitted changes
- Write CLRs (compensation log records) for each undo action — CLRs have a `undo_next_lsn` pointer to skip already-undone work
- CLRs are redo-only: if we crash during recovery, the redo pass replays CLRs and the undo pass skips already-compensated records
- After undo, write an abort record for each rolled-back transaction

**Test:** Start 5 transactions with interleaved writes, commit some, crash, recover, verify committed data is present and uncommitted data is fully rolled back. Crash again during recovery, recover again, verify correctness (tests CLR logic).

**Resources:**
- CMU 15-445 Lecture 20 — undo pass, CLR structure, the `undo_next_lsn` trick
- Petrov Ch. 12 — undo recovery, compensation records
- ARIES paper Section 9 — undo protocol and nested rollbacks
- The CLR chain is the hardest part of ARIES — draw it out on paper before coding

### 7.5 — Checkpointing

Periodic fuzzy checkpoints to bound the amount of log that must be replayed during recovery.

- Fuzzy checkpoint: snapshot the dirty page table and active transaction table to the log without halting the system
- Write a `begin_checkpoint` record, then the DPT/ATT data, then an `end_checkpoint` record
- Store the LSN of the last successful `begin_checkpoint` in a known location (master record / checkpoint meta file)
- Recovery starts analysis from this LSN instead of scanning the entire log
- Trigger checkpoints periodically (by time or by log bytes written)

**Test:** Write a large volume of transactions, take a checkpoint, write more, crash. Verify recovery only replays from the checkpoint forward. Measure recovery time with and without checkpoints.

**Resources:**
- CMU 15-445 Lecture 20 — fuzzy checkpoints vs sharp checkpoints
- Petrov Ch. 12 — checkpoint strategies, master record
- Postgres: `src/backend/postmaster/checkpointer.c` — real checkpoint scheduling and execution
- Keep the master record simple: a small file with a single LSN, atomically overwritten
