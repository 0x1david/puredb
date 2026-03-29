# PureDB

A multi-threaded relational database built from scratch in Rust.

The goal is to learn database internals — storage engines, query execution, concurrency control, crash recovery — by implementing them. Inspired by systems like Postgres, SQLite, and Oracle.

## Project Structure

```
ARCHITECTURE.md          # Subsystem map and descriptions
ROADMAP.md               # Build phases and milestones
phases/                  # Detailed task breakdowns per phase
  01-storage-engine/
  02-tuple-layout-catalog/
  03-access-methods/
  04-execution-engine/
  05-sql-parser-planner/
  06-concurrency-control/
  07-recovery/
  08-network-layer/
```

## Approach

All code is hand-written. Planning docs (`ARCHITECTURE.md`, `ROADMAP.md`) were bootstrapped with Claude and may be revised as the project evolves.
