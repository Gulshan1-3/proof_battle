# Real-Time AI Prover Simulation Server

The simulation server in [`scripts/ml/simulation_server.py`](file:///home/gulshansharma/proof_battle/scripts/ml/simulation_server.py) provides high-concurrency HTTP API services on port 8088 to power automated solver bots, solo practice arenas, and batch theorem evaluation.

---

## 1. Concurrency Architecture & 512+ Simulation Scaling

To support 512+ concurrent automated proof battles without crashing the host machine or exhausting RAM:
- **Thread Pool Concurrency**: Utilizes `ThreadingHTTPServer` to process incoming HTTP requests across independent threads.
- **Singleton Prover with Lock**: A global `_GLOBAL_MCTS_PROVER` instance is shared across simulation threads, with generation operations guarded by `threading.Lock()` to prevent GPU/CPU memory thrashing.
- **Constant Memory Footprint**: Because ProofBDH maintains $O(1)$ synaptic memory states (409 KB per instance) rather than growing KV-caches, hundreds of parallel duel sessions can evaluate candidates concurrently within a 1.2 GiB RAM budget.

---

## 2. HTTP API Reference

The server listens on `http://0.0.0.0:8088`.

### `GET /health`
Verifies server liveness.
- **Response**:
  ```json
  {
    "status": "healthy",
    "timestamp": 1728864000.0,
    "version": "1.0.0"
  }
  ```

### `GET /api/state`
Returns engine status, neural model metadata, and parameter limits.
- **Response**:
  ```json
  {
    "engine": "DBF-MCTS Prover",
    "model": "ProofBDH",
    "parameters": 9906560,
    "max_parameters_allowed": 10000000,
    "parameter_audit_passed": true,
    "device": "cpu",
    "total_theorems": 128,
    "active_simulations": 0
  }
  ```

### `GET /api/theorems`
Returns the catalog of curated formal Lean 4 challenge problems.
- **Query Parameters**:
  - `category` (optional): Filter by mathematical domain (e.g., `algebra`, `number_theory`).
  - `difficulty` (optional): Filter by difficulty level (1 to 5).
- **Response**: Array of problem objects containing `id`, `goal`, `imports`, `difficulty`, and `category`.

### `POST /api/prover/solve`
Executes an automated proof search for a given theorem.
- **Request Body**:
  ```json
  {
    "problem_id": "test_amc12_1",
    "goal": "(a + b)^2 - 4 * a * b = (a - b)^2",
    "params": "(a b : ℝ)",
    "simulations_budget": 16,
    "timeout_sec": 15.0
  }
  ```
- **Response**:
  ```json
  {
    "solved": true,
    "winning_script": "ring",
    "solve_method": "one_step",
    "nodes_expanded": 8,
    "elapsed_ms": 32.5,
    "diagnostic_repairs_attempted": 0
  }
  ```

### `POST /api/autoformalize`
Translates informal math or LaTeX into a verified Lean 4 statement.
- **Request Body**:
  ```json
  {
    "statement": "For all integers n, n + 0 = n"
  }
  ```
- **Response**:
  ```json
  {
    "formalized": true,
    "binders": "(n : ℤ)",
    "goal": "n + 0 = n",
    "verified_by_kernel": true
  }
  ```

### `POST /api/verify`
Verifies a specific tactic script candidate directly against the Lean 4 kernel.
- **Request Body**:
  ```json
  {
    "raw_code": "theorem test (n : ℕ) : n + 0 = n := by",
    "tactic_script": "simp"
  }
  ```
- **Response**:
  ```json
  {
    "verified": true,
    "elapsed_ms": 18.2,
    "compiler_error": null
  }
  ```

---

## 3. Operations & Startup

To launch the simulation server:
```bash
python3 scripts/ml/simulation_server.py --port 8088
```

To run in the background as a service daemon:
```bash
nohup python3 scripts/ml/simulation_server.py --port 8088 > /tmp/sim_server.log 2>&1 &
```

---

## 4. Code References

- Server Implementation: [`scripts/ml/simulation_server.py`](file:///home/gulshansharma/proof_battle/scripts/ml/simulation_server.py)
- Prover Engine: [`scripts/ml/dbf_mcts_prover.py`](file:///home/gulshansharma/proof_battle/scripts/ml/dbf_mcts_prover.py)
