# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

PureDB is a multi-threaded relational database built from scratch in Rust as a learning project. All code is hand-written by the user; Claude assists with planning and documentation only unless explicitly asked to write code.

## Build Commands

```bash
cargo build                        # build entire workspace
cargo test                         # run all tests across all crates
cargo test -p storage              # run tests for a single crate
cargo test -p storage test_name    # run a single test
cargo clippy --workspace           # lint all crates
```

## Architecture

Cargo workspace with a binary entrypoint (`crates/puredb`) and library crates for each subsystem:

- `storage` — disk manager, page layout, buffer pool
- `catalog` — tuple serialization, data types, system catalog
- `access` — B+Tree and other index structures
- `execution` — volcano/iterator model query operators
- `sql` — lexer, parser, binder, planner
- `concurrency` — locks, MVCC, transaction manager
- `recovery` — WAL, ARIES, checkpointing
- `network` — TCP listener, wire protocol

Dependencies flow downward: `network` → `sql` → `execution` → `access` → `catalog` → `storage`. `concurrency` and `recovery` are cross-cutting (used by storage and above).

## Planning Docs

- `ARCHITECTURE.md` — subsystem map and descriptions
- `ROADMAP.md` — build phases and milestones
- `phases/<NN>-<name>/` — detailed task breakdowns per phase
