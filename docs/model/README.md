# ProofBattle AI Prover: Neural Architecture & Reasoning System

The ProofBattle AI Prover subsystem (`scripts/ml/`) provides an automated formal theorem-proving engine capable of generating and formally verifying machine-checked proofs in Lean 4.

The system is engineered to operate under strict resource constraints ($\le 10,000,000$ parameters, sub-30ms per-candidate verification latency, and low resident memory) while competing on standard formal reasoning benchmarks such as miniF2F and high-throughput real-time battle simulations.

---

## High-Level Subsystem Architecture

```text
               +-------------------------------------------------------------+
               |              Informal or Formal Theorem Input               |
               |       (Natural Language / LaTeX / Lean 4 Statement)         |
               +------------------------------+------------------------------+
                                              |
                                              v
                              +-------------------------------+
                              |    Autoformalization Engine   |
                              |      (autoformalizer.py)      |
                              +---------------+---------------+
                                              |
                                              | Normalized Lean 4 Goal Statement
                                              v
      +-------------------------------------------------------------------------------+
      |                 Diagnostic Best-First MCTS Prover (dbf_mcts_prover.py)        |
      |                                                                               |
      |   +-----------------------+                    +--------------------------+   |
      |   |   ProofBDH (9.9M)     |                    |  BM25 Premise Retriever  |   |
      |   | Synaptic Hebbian Net  |                    |  102,278 Mathlib Lemmas  |   |
      |   |   (proof_bdh.py)      |                    |  (premise_retriever.py)  |   |
      |   +-----------+-----------+                    +------------+-------------+   |
      |               |                                             |                 |
      |               | Length-Normalized Priors                    | Top-k Premises  |
      |               +----------------------+----------------------+                 |
      |                                      |                                        |
      |                                      v                                        |
      |                       +------------------------------+                        |
      |                       |    PUCT Exploration Tree     |                        |
      |                       +--------------+---------------+                        |
      +--------------------------------------|----------------------------------------+
                                             | Candidate Tactic Script
                                             v
                             +-------------------------------+
                             |    Persistent Lean 4 REPL     |
                             |      (lean_repl_client.py)    |
                             +---------------+---------------+
                                             |
                     +-----------------------+-----------------------+
                     |                                               |
              [Proof Accepted]                                [Compiler Error]
                     |                                               |
                     v                                               v
        +-------------------------+                     +-------------------------+
        | Formal Victory Verdict  |                     |  In-Tree Self-Repair    |
        |  100% Soundness Verified|                     |  Conditioned on Stderr  |
        +-------------------------+                     +-------------------------+
```

---

## Core Components

| Component | Module | Description |
| :--- | :--- | :--- |
| **ProofBDH Architecture** | [`scripts/ml/proof_bdh.py`](file:///home/gulshansharma/proof_battle/scripts/ml/proof_bdh.py) | 9,906,560-parameter post-Transformer neural model with Hebbian fast-weight synaptic plasticity ($O(1)$ memory). |
| **DBF-MCTS Prover** | [`scripts/ml/dbf_mcts_prover.py`](file:///home/gulshansharma/proof_battle/scripts/ml/dbf_mcts_prover.py) | Diagnostic Best-First Monte Carlo Tree Search unifying PUCT exploration, length-normalized BFS priors, and compiler diagnostic self-repair. |
| **BM25 Premise Retriever** | [`scripts/ml/premise_retriever.py`](file:///home/gulshansharma/proof_battle/scripts/ml/premise_retriever.py) | Parameter-free inverted index indexing 102,278 Mathlib competition lemmas with sub-5ms query latency. |
| **Lean 4 REPL Client** | [`scripts/ml/lean_repl_client.py`](file:///home/gulshansharma/proof_battle/scripts/ml/lean_repl_client.py) | Subprocess client maintaining Mathlib resident in RAM for sub-30ms headless proof verification. |
| **Autoformalizer** | [`scripts/ml/autoformalizer.py`](file:///home/gulshansharma/proof_battle/scripts/ml/autoformalizer.py) | Neuro-symbolic parser translating LaTeX and natural language statements into formal Lean 4 declarations. |
| **Simulation Server** | [`scripts/ml/simulation_server.py`](file:///home/gulshansharma/proof_battle/scripts/ml/simulation_server.py) | Multi-threaded HTTP server on port 8088 serving automated solver bots and solo practice matches for 512+ concurrent arenas. |
| **Evaluation Suite** | [`scripts/ml/eval_dbf_mcts_minif2f.py`](file:///home/gulshansharma/proof_battle/scripts/ml/eval_dbf_mcts_minif2f.py) | Evaluation harness for miniF2F benchmarks with pass@k measurement and memory watchdogs. |
| **Integrity Auditor** | [`scripts/ml/audit_solutions_integrity.py`](file:///home/gulshansharma/proof_battle/scripts/ml/audit_solutions_integrity.py) | Verifies parameter limits, zero `sorryAx` axioms, and zero contamination leaks. |

---

## Detailed Specifications

- [ProofBDH Architecture Deep-Dive](PROOF_BDH.md)
- [Diagnostic Best-First MCTS & PUCT Search](DBF_MCTS.md)
- [BM25 Mathlib Premise Retriever](PREMISE_RETRIEVAL.md)
- [Persistent Lean 4 REPL & Subprocess Sandbox](LEAN_REPL_PIPELINE.md)
- [Autoformalization Engine](AUTOFORMALIZATION.md)
- [Benchmark Results & Integrity Audits](BENCHMARKS_AND_EVAL.md)
- [Simulation Server & HTTP API](SIMULATION_SERVER.md)
