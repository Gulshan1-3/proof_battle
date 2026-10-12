# Diagnostic Best-First Monte Carlo Tree Search (DBF-MCTS)

`DBF-MCTS` is the automated proof search engine implemented in [`scripts/ml/dbf_mcts_prover.py`](file:///home/gulshansharma/proof_battle/scripts/ml/dbf_mcts_prover.py). It unifies best-first tree search, length-normalized policy priors, and closed-loop Lean 4 compiler error repair.

---

## 1. Search Formulation & Core Equations

Traditional MCTS relies on random rollouts or simple value heads. In formal mathematics, evaluating an unfinished proof state with rollouts is computationally prohibitive and produces noisy signals. DBF-MCTS replaces rollouts with **guided policy expansion** and **compiler diagnostic feedback**.

### A. Length-Normalized BFS Priors
Standard autoregressive language models penalize longer token sequences exponentially, creating an artificial bias toward trivial tactics (`rfl`, `done`) over compound tactics (`simp [Nat.add_comm, Nat.mul_zero]`). 

To eliminate sequence length bias, candidate tactics are scored using length-normalized log-probabilities (following BFS-Prover, ByteDance Seed, ACL 2025):

$$P_{\text{BFS}}(a \mid s) = \exp\left( \frac{1}{|a|^\alpha} \sum_{t=1}^{|a|} \log P(\text{token}_t \mid s, \text{token}_{<t}) \right)$$

where:
- $|a|$ is the number of tokens generated for candidate tactic $a$.
- $\alpha = 0.8$ is the length-normalization temperature parameter.

### B. PUCT Node Selection
At each iteration, the search descends the tree from the root node to a leaf using the Predictor Upper Confidence bound for Trees (PUCT):

$$a^* = \arg\max_a \left[ Q(s, a) + U(s, a) \right]$$

$$U(s, a) = c_{\text{puct}} \cdot P_{\text{BFS}}(a \mid s) \cdot \frac{\sqrt{N(s)}}{1 + N(s, a)}$$

where:
- $Q(s, a) = \frac{W(s, a)}{N(s, a)}$ is the empirical mean value of the branch.
- $N(s)$ is the visit count of parent state $s$.
- $N(s, a)$ is the visit count of action $a$.
- $c_{\text{puct}} = 1.414$ balances exploitation of high-confidence neural predictions with exploration across sister branches.

---

## 2. In-Tree Compiler Diagnostic Self-Repair

When a candidate tactic fails in Lean 4, most theorem provers simply discard the action with a reward of $0$. In contrast, DBF-MCTS inspects the Lean compiler's diagnostic output:

```text
               +--------------------------------------+
               | Leaf Node Evaluation in Lean 4 Kernel|
               +-------------------+------------------+
                                   |
                  +----------------+----------------+
                  |                                 |
           [Proof Accepted]                  [Compiler Error]
                  |                                 |
           Value = 1.0                              v
           Terminates Search            +-----------------------+
                                        | Classify Error Type   |
                                        +-----------+-----------+
                                                    |
                         +--------------------------+--------------------------+
                         |                                                     |
             "unsolved goals"                                          "type mismatch"
                         |                                                     |
             Value = 0.70 (Valid Partial Step)                         Value = 0.15
             Clear error, extract subgoals                             Spawn Diagnostic Repair Node
             Descend tree deeper                                       Prompt ProofBDH with error trace
```

### Self-Repair Prompting
When Lean returns a type mismatch or syntax error, the prover generates corrective candidates using interactive diagnostic prompting:

```text
<goal> {current_goal} <attempt> {failed_tactic} <err> {sanitized_error} <plan> repair </plan><tactic>
```

ProofBDH conditions directly on the compiler's error message to autoregressively synthesize a replacement tactic (e.g., swapping `ring` for `linarith`, or supplying the missing argument). In empirical MiniF2F test runs, this mechanism successfully rescued multiple failed proofs that would have otherwise terminated as search failures.

---

## 3. Search Workflow & Lifecycle

1. **Step 0: Fast-Path One-Step Evaluation**
   Before building a deep tree, the top $K$ neural candidates (greedy, context-conditioned, and stochastic nucleus samples) are evaluated immediately against Lean 4. 
   - Over **93.9% of all solved MiniF2F test theorems (62 / 66)** close in Step 0 without requiring search.
2. **Step 1: Multi-Step Tree Search**
   If Step 0 does not close the proof:
   - Select leaf via PUCT.
   - If partial progress is detected ("unsolved goals"), extract remaining subgoals and continue expansion.
   - If error is repairable, spawn diagnostic repair nodes.
   - Propagate terminal value ($1.0$ for closed proof, $0.70$ for valid progress) back up the tree.
3. **Thermal Pacing**
   Between iterations, a short $5\text{ms}$ thermal pause prevents CPU frequency throttling during heavy 512+ simulation sweeps.

---

## 4. Code References

- Core MCTS Implementation: [`scripts/ml/dbf_mcts_prover.py`](file:///home/gulshansharma/proof_battle/scripts/ml/dbf_mcts_prover.py)
- Benchmark Runner: [`scripts/ml/eval_dbf_mcts_minif2f.py`](file:///home/gulshansharma/proof_battle/scripts/ml/eval_dbf_mcts_minif2f.py)
- Lean Evaluation Seam: [`scripts/ml/lean_repl_client.py`](file:///home/gulshansharma/proof_battle/scripts/ml/lean_repl_client.py)
