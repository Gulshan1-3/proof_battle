# High-Throughput Lean 4 REPL & Subprocess Verification

The verification layer in [`scripts/ml/lean_repl_client.py`](../../scripts/ml/lean_repl_client.py) bridges the neural prover and the formal Lean 4 kernel, reducing candidate verification latency from seconds to milliseconds.

---

## 1. The Cold-Start Bottleneck

Lean 4 and its mathematics library (Mathlib) contain hundreds of thousands of formal definitions and compiled object files (`.olean`). 

- **Cold Execution (`lake env lean`)**: Spawning a new Lean process per candidate script forces the OS to re-map several gigabytes of compiled `.olean` files into memory. Measured cold latency is **3.7 seconds per candidate**.
- **Impact on Search**: In an MCTS budget of 64 simulations across 244 MiniF2F problems, cold verification would require over 15 hours of pure I/O overhead.

---

## 2. Persistent Resident REPL Architecture

The `LeanREPLClient` spawns a single persistent background process using the Lean 4 REPL binary (`lean/repl/.lake/build/bin/repl`):

```text
+-------------------------------------------------------------------------+
|                        Python Prover Runtime                            |
|                                                                         |
|  +--------------------+         JSON IPC Pipe       +----------------+  |
|  | DBF-MCTS / ProofBDH| --------------------------> | Thread Lock    |  |
|  +--------------------+                             +--------+-------+  |
+--------------------------------------------------------------|----------+
                                                               |
                                            Stdin / Stdout     | Sub-30ms
                                                               v
+-------------------------------------------------------------------------+
|                  Lean 4 Background Process (Resident)                  |
|                                                                         |
|  +-------------------------------------------------------------------+  |
|  | Mathlib Resident Environment (Preloaded in RAM, ~1.2 GiB RSS)     |  |
|  +-------------------------------------------------------------------+  |
|  | Root Environment ID: 0                                            |  |
|  | Reusable State Tree: zero disk reads on candidate verification    |  |
|  +-------------------------------------------------------------------+  |
+-------------------------------------------------------------------------+
```

### Measured Performance
- **Cold Process Startup**: 3,700 ms
- **Persistent REPL Verification**: **15 – 35 ms** per candidate script
- **Throughput Acceleration**: **100x – 250x faster**

---

## 3. Subprocess Safety & Concurrency Safeguards

Because theorem proving code may contain infinite loops (e.g., non-terminating simplification rules), the REPL client enforces strict process controls:

1. **Thread-Safe Serialization (`RLock`)**:
   A re-entrant lock ensures parallel Python threads do not interleave commands across the REPL stdin/stdout pipe.
2. **Process Group Isolation (`os.setsid`)**:
   The REPL is spawned in its own process group. If a candidate script hangs past the 10-second timeout, `os.killpg(proc.pid, signal.SIGKILL)` terminates the process and any spawned child threads completely, preventing orphan zombie processes.
3. **Automated Respawn & Environment Reset**:
   If the REPL process crashes, panics, or times out, the client automatically re-spawns a fresh worker and re-initializes the root Mathlib environment.
4. **Memory Watchdog**:
   RSS memory is continuously monitored using `psutil`. If total process tree memory exceeds 4.2 GiB, garbage collection is triggered and excess memory is reclaimed.

---

## 4. Code References

- REPL Client: [`scripts/ml/lean_repl_client.py`](../../scripts/ml/lean_repl_client.py)
- Standalone Worker Fallback: [`scripts/ml/lean_worker.py`](../../scripts/ml/lean_worker.py)
- REPL Upstream Directory: `lean/repl/`
