# Phase 5: SQL Parser & Planner

Turns SQL text into executable query plans that drive the Phase 4 operators.

**Milestone:** Can parse a SQL string, validate it against the catalog, and produce a physical query plan that the execution engine runs correctly.

## Resources (whole phase)

- **CMU 15-445 (Andy Pavlo)** — Lectures 12–15 cover query processing and optimization end-to-end.
  - Lecture 12: Query Processing I — overview of parsing, binding, planning
  - Lecture 13: Query Processing II — plan representations, physical vs logical operators
  - Lecture 14: Query Planning & Optimization I — heuristics, rule-based rewriting
  - Lecture 15: Query Planning & Optimization II — cost-based optimization (read for context, we only do rule-based here)
  - YouTube playlist: search "CMU 15-445 Fall 2023"
- **"Crafting Interpreters" by Robert Nystrom** — Chapters 4–8 cover scanning and parsing technique. SQL is simpler than a general-purpose language, but the mechanics (lexer, recursive descent parser, AST) are identical. Free online at craftinginterpreters.com.
- **`sqlparser-rs` crate** — A production SQL parser in Rust. Read the source for inspiration on token types and AST node design, but we're building from scratch for learning.
- **Postgres source code** — `src/backend/parser/` (lexer, grammar, parse analysis), `src/backend/optimizer/` (planner, path selection). The grammar is yacc-based and complex, but the analyzer and planner logic are instructive.

## Sub-phases

### 5.1 — Lexer/Tokenizer

Converts a raw SQL string into a flat sequence of tokens. No structure yet — just classification.

- Define a `Token` enum: keywords (`SELECT`, `FROM`, `WHERE`, `INSERT`, `UPDATE`, `DELETE`, `CREATE`, `TABLE`, `INDEX`, `INTO`, `VALUES`, `SET`, `AND`, `OR`, `NOT`, ...), identifiers, integer/float/string literals, operators (`=`, `<>`, `<`, `>`, `<=`, `>=`, `+`, `-`, `*`, `/`), punctuation (`,`, `(`, `)`, `;`)
- Handle case insensitivity for keywords (`select` == `SELECT`)
- Track source position (line, column) on each token for error reporting
- Skip whitespace, handle single-line comments (`--`)

**Test:** Lex `SELECT name, age FROM users WHERE age >= 18;` and assert the exact token sequence. Test edge cases: string literals with escapes, numeric literals, unknown characters producing error tokens.

**Resources:**
- Crafting Interpreters Ch. 4 (Scanning) — the lexer chapter. Directly applicable; just swap Lox keywords for SQL keywords.
- `sqlparser-rs` `tokenizer.rs` — see how they define `Token` and handle SQL-specific quirks like quoted identifiers.
- Rust: `std::iter::Peekable<Chars>` is all you need for the scanner loop. `logos` crate exists but hand-rolling is more educational.

### 5.2 — Parser

Transforms the token stream into a typed AST. Recursive descent, no parser generators.

- Define AST nodes: `Statement` (Select, Insert, Update, Delete, CreateTable, CreateIndex), `Expr` (Column, Literal, BinaryOp, UnaryOp), `SelectItem`, `TableRef`, `ColumnDef`, etc.
- Parse `SELECT [columns] FROM [table] WHERE [expr]` with `AND`/`OR` precedence
- Parse `INSERT INTO [table] (cols) VALUES (vals)`
- Parse `UPDATE [table] SET col = expr WHERE [expr]`
- Parse `DELETE FROM [table] WHERE [expr]`
- Parse `CREATE TABLE [name] (col type, ...)` with basic types (INTEGER, TEXT, BOOLEAN, FLOAT)
- Parse `CREATE INDEX [name] ON [table] (col)`
- Produce clear error messages with source position on parse failures

**Test:** Parse a multi-column SELECT with a WHERE clause containing AND/OR, verify the AST structure. Test each statement type. Test malformed SQL produces a useful error pointing at the right token.

**Resources:**
- Crafting Interpreters Ch. 6 (Parsing Expressions) — Pratt parsing / recursive descent for operator precedence. The `WHERE` clause is just expression parsing.
- Crafting Interpreters Ch. 8 (Statements and State) — parsing statement-level grammar.
- `sqlparser-rs` `parser.rs` — see `parse_select`, `parse_insert` etc. for real SQL grammar decisions.
- Rust: represent AST with enums + `Box` for recursive nodes. Derive `Debug, PartialEq` on everything for easy test assertions.

### 5.3 — Binder/Analyzer

Resolves names and validates semantics. Bridges the parsed AST (which is just syntax) to meaningful operations against actual tables and columns.

- Resolve table names against the catalog — does the table exist?
- Resolve column names — does this column exist in this table? Handle ambiguity if we later add joins.
- Type check expressions — can't compare an INTEGER to a TEXT, `+` requires numeric operands
- Validate INSERT column count matches VALUES count
- Produce a "bound" or "annotated" AST where every column reference carries its table, column index, and type
- Collect clear semantic errors: "table 'foo' does not exist", "column 'bar' not found in table 'users'"

**Test:** Bind a SELECT against a catalog with known tables. Verify column references are resolved to the correct table/column index. Test that referencing a nonexistent table or column produces the expected error.

**Resources:**
- CMU 15-445 Lecture 12 — covers the binding/name resolution step in the query processing pipeline
- Postgres: `src/backend/parser/analyze.c` — `transformStmt()` is where Postgres does this work
- Rust: the bound AST can be a separate type from the parsed AST (cleaner) or the same type with `Option` fields filled in (simpler). Separate types are worth the boilerplate.

### 5.4 — Naive Planner

Converts the bound AST into a physical query plan — a tree of Phase 4 operators. No optimization; just produce something correct.

- `SELECT ... FROM t WHERE pred` becomes `Filter(pred, SeqScan(t))`
- `SELECT col1, col2 ...` becomes `Projection([col1, col2], <child>)`
- `INSERT` becomes `Insert(table, values)`
- `UPDATE ... SET col = expr WHERE pred` becomes `Update(table, assignments, Filter(pred, SeqScan(t)))`
- `DELETE ... WHERE pred` becomes `Delete(table, Filter(pred, SeqScan(t)))`
- The plan nodes should be the same operator types from Phase 4, wired together into a tree
- Plan output is a tree that the execution engine can walk top-down (Volcano-style from Phase 4)

**Test:** Plan a SELECT with a WHERE clause, verify the plan tree is `Projection -> Filter -> SeqScan` with the correct table and predicate. Plan each statement type and assert the structure.

**Resources:**
- CMU 15-445 Lecture 13 — plan representations, logical vs physical operators
- Postgres: `src/backend/optimizer/plan/createplan.c` — how Postgres builds plan nodes (skip the optimization parts for now)
- Rust: plan nodes as an enum or trait objects. Enum is simpler and sufficient at this stage.

### 5.5 — Basic Optimizer

Rule-based rewrites on the query plan. No cost model, no statistics — just mechanical transformations that are always beneficial or can be decided with catalog metadata.

- **Predicate pushdown:** if a filter references only one table, push it below any future join node (matters more when joins exist, but set up the infrastructure now)
- **Index scan selection:** query the catalog for available indexes. If a WHERE clause filters on an indexed column with a point lookup or range, replace `SeqScan` + `Filter` with `IndexScan` (the B+Tree operator from Phase 3)
- **Projection pushdown:** only read the columns the query actually needs (reduces data flowing through the plan)
- Apply rules as plan-tree rewrites: walk the tree, match patterns, replace nodes

**Test:** Create a table with an index on column `age`. Plan `SELECT name FROM users WHERE age = 25`. Verify the optimizer replaces `Filter(SeqScan)` with `IndexScan(age = 25)` and the projection only requests `name` and `age`. Without the index, verify it stays as `Filter(SeqScan)`.

**Resources:**
- CMU 15-445 Lecture 14 — heuristic/rule-based optimization, predicate pushdown, projection pushdown
- CMU 15-445 Lecture 15 — cost-based optimization (read for awareness, not implementing here)
- Postgres: `src/backend/optimizer/path/` — `allpaths.c`, `indxpath.c` for index path selection logic
- Rust: tree rewriting is natural with recursive functions over your plan enum. A `fn optimize(plan: Plan) -> Plan` that pattern-matches and recurses.
