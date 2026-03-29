# Phase 8: Network Layer

How external clients connect over TCP and run SQL against PureDB.

**Milestone:** A standard Postgres client (`psql`, `pgcli`, any libpq-based driver) can connect, send SQL, and receive results.

## Resources (whole phase)

- **Postgres wire protocol documentation** — The authoritative reference. Covers every message type, the startup handshake, and the query lifecycle.
  - Frontend/Backend Protocol: https://www.postgresql.org/docs/current/protocol.html
  - Message Formats: https://www.postgresql.org/docs/current/protocol-message-formats.html
  - Message Flow: https://www.postgresql.org/docs/current/protocol-flow.html
- **Postgres source code** — `src/backend/libpq/` (connection handling, auth), `src/backend/tcop/postgres.c` (main query dispatch loop), `src/backend/utils/adt/` (output functions for data types)
- **tokio** — https://tokio.rs/tokio/tutorial — async runtime for the eventual production-grade version. Start synchronous, migrate later.

## Sub-phases

### 8.1 — TCP Listener

Accept TCP connections and spawn a handler per connection. Start with the standard library; async is an optimization, not a prerequisite.

- Bind to a configurable `host:port` (default `127.0.0.1:5433`)
- Accept connections in a loop, spawn a `std::thread` per connection
- Each handler owns its `TcpStream` and reads/writes bytes directly
- Clean shutdown: listen for a signal or poison pill to stop accepting

**Test:** Start the listener, connect with a raw `TcpStream` from a test, send bytes, verify the handler receives them and responds.

**Resources:**
- Rust: `std::net::TcpListener`, `TcpStream`, `std::thread::spawn` — all you need for v1
- tokio upgrade path: replace `std::net` with `tokio::net`, `std::thread` with `tokio::spawn`, add `async`/`await` — the structure stays identical
- Postgres: `src/backend/libpq/pqcomm.c` — how Postgres accepts connections

### 8.2 — Wire Protocol

Implement enough of the Postgres simple query protocol that `psql` can connect and send queries. This is the heart of the phase.

- Parse the **StartupMessage** (protocol version, `user`/`database` params) and reply with **AuthenticationOk** + **ReadyForQuery**
- Handle **Query** messages: extract the SQL string, pass it to the execution engine
- Send **RowDescription** (column names, types, type OIDs) before result rows
- Send **DataRow** messages for each tuple
- Send **CommandComplete** (e.g., `SELECT 3`) after the last row
- Send **ErrorResponse** with severity, code, and message when something fails
- Handle **Terminate** to close cleanly

**Test:** Start PureDB, connect with `psql`, run `SELECT 1;`, verify the result comes back correctly. Also test a bad query and confirm `psql` shows the error without disconnecting.

**Resources:**
- Protocol message formats: https://www.postgresql.org/docs/current/protocol-message-formats.html — every byte-level detail is here
- Simple query flow: https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-SIMPLE-QUERY
- Rust: `byteorder` crate or `i32::from_be_bytes()` / `to_be_bytes()` — the wire protocol is big-endian throughout
- Postgres type OIDs: `SELECT oid, typname FROM pg_type;` — you'll need OIDs for RowDescription (e.g., `int4` = 23, `text` = 25, `bool` = 16)

### 8.3 — Session State

Each connection needs its own transaction context. Track whether the client is in a transaction block and handle the control-flow SQL.

- Per-connection `Session` struct holding current transaction state (`Idle`, `InTransaction`, `Failed`)
- `BEGIN` transitions from `Idle` → `InTransaction`
- `COMMIT` and `ROLLBACK` transition back to `Idle` (wiring to the actual transaction engine from Phase 6)
- `ReadyForQuery` message includes a status byte: `I` (idle), `T` (in transaction), `E` (failed transaction) — must reflect real state
- Queries inside a `Failed` transaction are rejected until `ROLLBACK`

**Test:** Connect, run `BEGIN; INSERT ...; SELECT ...; COMMIT;` — verify the `ReadyForQuery` status byte changes correctly at each step. Force an error mid-transaction and confirm subsequent queries are rejected until `ROLLBACK`.

**Resources:**
- ReadyForQuery status indicators: https://www.postgresql.org/docs/current/protocol-message-formats.html (search "ReadyForQuery")
- Postgres: `src/backend/tcop/postgres.c` — `PostgresMain()` is the per-connection event loop; `src/backend/access/transam/xact.c` — transaction state machine
- Postgres transaction states: https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-EXT-QUERY — describes how transaction state affects message flow

### 8.4 — Result Serialization

Encode query engine output into the wire protocol's binary/text format. Handle type conversion and error formatting.

- Convert each column value to its Postgres text representation (integers as decimal strings, booleans as `t`/`f`, NULLs as `-1` length)
- Populate **RowDescription** fields: column name, table OID (0 if not applicable), type OID, type size, type modifier, format code (0 = text)
- Format **ErrorResponse** fields: severity (`ERROR`, `FATAL`), SQLSTATE code (e.g., `42601` for syntax error, `42P01` for undefined table), message, optional detail/hint
- Handle the difference between `SELECT` (returns rows), `INSERT`/`UPDATE`/`DELETE` (returns affected row count), and `CREATE`/`DROP` (returns command tag only)

**Test:** Run queries returning various types (integers, strings, booleans, NULLs), capture the raw bytes sent to the client, verify they match the expected wire format. Test that error responses include correct SQLSTATE codes.

**Resources:**
- Data type OIDs and text output formats: https://www.postgresql.org/docs/current/datatype.html
- SQLSTATE error codes: https://www.postgresql.org/docs/current/errcodes-appendix.html — pick codes from here for your error responses
- Postgres: `src/backend/utils/adt/int8.c`, `varchar.c`, `bool.c` — see how Postgres formats each type for the wire
- Rust: implement a `ToWireFormat` trait on your internal types to keep serialization clean
