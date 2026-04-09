# Phase 9: Replication & Consensus (deep)

Ship the WAL to replicas, elect leaders, and handle failures. This is the layer that turns a single-node storage engine into a fault-tolerant distributed system.

**Milestone:** Leader-follower replication with automatic failover via simplified Raft. Can lose the leader, elect a new one, and continue serving reads from a follower with no data loss for committed transactions.

## Resources (whole phase)

- **"In Search of an Understandable Consensus Algorithm" (Ongaro & Ousterhout, 2014)** — The Raft paper. Read this first. It's written to be understood, unlike Paxos.
- **raft.github.io** — Raft visualization, reference implementation links, and the TLA+ spec.
- **"Designing Data-Intensive Applications" by Martin Kleppmann** — Chapters 5 (Replication), 8 (Distributed System Troubles), 9 (Consistency and Consensus). The best high-level treatment.
- **MIT 6.824 (Distributed Systems)** — Labs 2 and 3 implement Raft. Lecture videos and lab descriptions are publicly available.
- **etcd/raft source** (Go) — a production Raft implementation. Clean, well-commented, good reference for state machine design.

## Sub-phases

### 9.1 — WAL Shipping (Leader → Follower)
Stream log records from the leader to followers over TCP. No leader election yet — the leader is hard-coded.

- Leader appends WAL records as usual, then sends them to connected followers
- Follower receives records, writes them to its own WAL, and replays them against its storage engine
- Track replication lag: the difference between the leader's latest LSN and the follower's applied LSN
- Handle follower disconnect and reconnect — resume from the follower's last applied LSN

**Test:** Start a leader and one follower. Insert 1000 rows on the leader. Verify the follower has all 1000 rows. Kill the follower, insert 100 more rows, restart the follower, verify it catches up.

**Resources:**
- Kleppmann Ch. 5 — leader-based replication, replication lag, failover
- Postgres: WAL shipping / streaming replication docs — the same idea at production scale
- Rust: `std::net::TcpStream` for the replication connection. Serialize WAL records with the same format used for disk.

### 9.2 — Raft Leader Election
Implement the leader election portion of Raft. Nodes can be leader, follower, or candidate.

- Each node has a persistent `current_term` and `voted_for`
- Followers become candidates after an election timeout (randomized, 150–300ms)
- Candidates request votes from all peers via `RequestVote` RPC
- A candidate wins if it receives a majority of votes
- Leaders send periodic heartbeats to prevent new elections
- Handle split votes, term confusion, and stale leaders

**Test:** Start a 3-node cluster, verify a leader is elected within a few seconds. Kill the leader, verify a new leader is elected. Restart the old leader, verify it steps down to follower.

**Resources:**
- Raft paper §5.1–5.2 — leader election, terms, election safety
- raft.github.io visualization — step through elections visually
- etcd/raft `raft.go` — `becomeCandidate()`, `becomeLeader()`, `becomeFollower()`

### 9.3 — Raft Log Replication
Replace the ad-hoc WAL shipping from 9.1 with Raft's log replication protocol, which guarantees committed entries are durable on a majority.

- Leader appends entries to its log, sends `AppendEntries` RPC to followers
- Followers append entries and respond with success/failure
- Leader commits an entry once a majority has replicated it
- Handle log inconsistencies: if a follower's log diverges, the leader sends earlier entries to bring it back in sync (nextIndex/matchIndex tracking)
- Client writes only succeed after the entry is committed (majority-replicated)

**Test:** 3-node cluster. Write 100 entries. Verify all three nodes have identical logs. Kill one follower, write 50 more, bring it back, verify it catches up. Kill the leader mid-write, verify the new leader has all committed entries.

**Resources:**
- Raft paper §5.3–5.4 — log replication, safety argument
- etcd/raft — `stepLeader()`, `handleAppendEntries()`
- MIT 6.824 Lab 2B — log replication lab with test cases

### 9.4 — Failure Detection & Automatic Failover
Tie it all together: detect leader failure and automatically failover without manual intervention.

- Heartbeat mechanism: leader sends heartbeats every N ms, followers expect them within a timeout
- On timeout: follower triggers election (already implemented in 9.2)
- After new leader is elected, it must commit a no-op entry to establish its authority
- Clients redirect to the new leader (or discover it via any node)
- Test network partitions: a minority partition should not elect a leader

**Test:** 5-node cluster. Kill the leader. Verify a new leader is elected and the cluster resumes accepting writes within 2× election timeout. Partition 2 nodes from 3 — the majority partition elects a leader, the minority does not. Heal the partition, verify the cluster converges.

**Resources:**
- Raft paper §5.4.1 — election restriction (leader completeness)
- Kleppmann Ch. 8 — unreliable networks, failure detection
- Jepsen test methodology — how to think about testing distributed systems under failure
