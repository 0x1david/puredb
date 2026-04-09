# Phase 5: SQL Parser & Planner (minimal)

Turns SQL text into executable query plans. Kept minimal — enough to demo the system end-to-end, not a goal in itself.

**Milestone:** Can parse basic SQL (SELECT, INSERT, CREATE TABLE), validate against the catalog, and produce a naive physical plan.

## Resources

- **"Crafting Interpreters" by Robert Nystrom** — Chapters 4–8 cover scanning and parsing technique. Free online at craftinginterpreters.com.
- **`sqlparser-rs` crate** — A production SQL parser in Rust. Read for inspiration, but we're building from scratch for learning.

## Sub-phases

### 5.1 — Lexer

Converts raw SQL into tokens.

- `Token` enum: keywords (`SELECT`, `FROM`, `WHERE`, `INSERT`, `CREATE`, `TABLE`, `INTO`, `VALUES`, `AND`, `OR`), identifiers, literals, operators, punctuation
- Case-insensitive keywords
- Skip whitespace and comments

**Test:** Lex `SELECT name FROM users WHERE age >= 18;` and assert the token sequence.

### 5.2 — Parser

Recursive descent parser producing a typed AST.

- Parse `SELECT [columns] FROM [table] WHERE [expr]`
- Parse `INSERT INTO [table] (cols) VALUES (vals)`
- Parse `CREATE TABLE [name] (col type, ...)`
- Error messages with source position

**Test:** Parse a SELECT with a WHERE clause, verify AST structure. Test malformed SQL produces useful errors.

### 5.3 — Binder & Naive Planner

Resolve names against the catalog and produce a physical plan. No optimizer.

- Resolve table/column names, type-check expressions
- `SELECT ... FROM t WHERE pred` → `Projection(Filter(SeqScan(t)))`
- `INSERT INTO t VALUES (...)` → `Insert(t, values)`

**Test:** Bind and plan a SELECT, verify the plan tree structure.
