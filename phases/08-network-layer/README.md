# Phase 8: Network Layer (minimal)

External clients connect over TCP and run SQL. Exists to demo the system, not as a goal in itself. Postgres wire protocol compatibility is not a target.

**Milestone:** A basic client can connect, send SQL, and receive results over TCP.

## Resources

- Rust: `std::net::TcpListener`, `TcpStream`, `std::thread::spawn`

## Sub-phases

### 8.1 — TCP Listener

Accept connections and spawn a handler per connection.

- Bind to `127.0.0.1:5433`, accept in a loop, spawn a thread per connection
- Each handler owns its `TcpStream`
- Clean shutdown via signal

**Test:** Start the listener, connect from a test, send bytes, verify response.

### 8.2 — Simple Wire Protocol

A custom (non-Postgres) text-based protocol. Keep it dead simple.

- Client sends a SQL string terminated by newline
- Server parses, executes, and sends back results as text lines
- Error responses include a message
- `QUIT` command to disconnect

**Test:** Connect, send `SELECT 1;`, verify the result comes back. Send a bad query, verify an error response.
