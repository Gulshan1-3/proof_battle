# ProofBDH: Baby Dragon Hatchling Neural Architecture

`ProofBDH` is a custom 9.9M-parameter neural sequence model specifically engineered for formal theorem proving in Lean 4. It implements the biological post-Transformer principles introduced by Pathway (2025) in *"The Dragon Hatchling: The Missing Link between the Transformer and Models of the Brain"*.

---

## 1. Architectural Highlights

Traditional Transformer decoders suffer from two core limitations when applied to formal theorem proving:
1. **$O(T)$ Memory Growth (KV-Cache Explosion)**: As proofs grow longer with multiple subgoals, keeping past key-value states exhausts GPU/CPU memory during parallel search (especially across 512+ concurrent arenas).
2. **Dense, Polysemantic Representations**: Softmax attention spreads probability mass across irrelevant tokens, hindering clean symbolic selection of Lean 4 tactics.

ProofBDH addresses both issues through four distinct mechanisms:

### A. Fast-Weight Synaptic Plasticity
Instead of caching all historical keys and values, ProofBDH maintains an internal recurrent synaptic state matrix $S_t \in \mathbb{R}^{d_k \times d_v}$ per attention head that updates in real time using a Hebbian outer-product rule:

$$S_t = \lambda S_{t-1} + \eta \, (k_t \otimes v_t)$$

where:
- $\lambda \in (0, 1)$ is the synaptic decay factor (retention rate).
- $\eta = 0.1$ is the synaptic plasticity rate (learning rate of fast weights).
- $k_t \otimes v_t = k_t v_t^\top$ is the rank-1 outer product binding queries to values.

Query tokens retrieve working memory via direct linear projection through the synaptic matrix:

$$y_t = \frac{1}{\sqrt{d_k}} \cdot q_t S_t$$

### B. Constant $O(1)$ Inference Memory
Because $S_t$ has fixed dimension $(d_k \times d_v) = (40 \times 40)$, each attention head requires only $1,600$ floats (6.4 KB in float32). Across 8 heads and 8 layers, the total recurrent working memory footprint is strictly **409.6 KB**, regardless of whether the proof sequence is 10 tokens or 10,000 tokens long.

### C. Multi-Scale Synaptic Retention Heads
To capture both local syntactic transitions (e.g., token separators, brackets) and permanent theorem goal context, each of the 8 attention heads operates at a different exponential timescale:

$$\lambda_h = 1 - 2^{-(4 + h)}, \quad h \in \{0, 1, \dots, 7\}$$

| Head Index | Decay $\lambda_h$ | Effective Token Half-Life $\tau = \frac{1}{1 - \lambda_h}$ | Functional Role |
| :---: | :---: | :---: | :--- |
| **Head 0** | $0.9375$ | 16 tokens | Local syntax, tactic punctuation, binders |
| **Head 1** | $0.96875$ | 32 tokens | Term structure, subexpression boundaries |
| **Head 2** | $0.984375$ | 64 tokens | Intermediate hypotheses |
| **Head 3** | $0.9921875$ | 128 tokens | Subgoal context |
| **Head 4** | $0.99609375$ | 256 tokens | Theorem premise hypotheses |
| **Head 5** | $0.998046875$ | 512 tokens | Long-range premise association |
| **Head 6** | $0.9990234375$ | 1024 tokens | Global module context |
| **Head 7** | $0.9995117$ | 2048 tokens | Permanent theorem goal & imports memory |

### D. Non-Negative Sparse Activations ($\text{ReLU}^2$)
Softmax attention is replaced with biological, non-negative neuron activations using $\text{ReLU}^2$:

$$q = \left[\max(0, x_q)\right]^2, \quad k = \left[\max(0, x_k)\right]^2$$

This enforces monosemantic mathematical representations: inactive neurons produce strict zeros, eliminating noisy cross-talk between unrelated mathematical symbols.

---

## 2. Configuration & Parameter Budget

ProofBDH is configured under a strict **sub-10,000,000 parameter budget**:

```python
@dataclass
class BDHConfig:
    vocab_size: int = 1500          # Lean 4 domain-specific mathematical BPE vocab
    dim: int = 320                  # Hidden state dimension
    n_layers: int = 8               # Number of BDH blocks
    n_heads: int = 8                # Multi-head synaptic heads (head_dim = 40)
    hidden_dim: int = 800           # SwiGLU FFN intermediate dimension
    max_seq_len: int = 512          # Rotary Positional Encoding (RoPE) window
    synaptic_decay: float = 0.95    # Base decay parameter
    plasticity_rate: float = 0.1    # Fast synaptic learning rate (eta)
    dropout: float = 0.05           # Regularization dropout
    norm_eps: float = 1e-5          # RMSNorm epsilon
```

### Parameter Breakdown

| Component | Dimensions / Formula | Parameter Count |
| :--- | :--- | :--- |
| **Token Embeddings** | `vocab_size * dim` ($1,500 \times 320$) | 480,000 |
| **Synaptic Attention (8 layers)** | $8 \times [4 \times (320 \times 320)]$ ($W_q, W_k, W_v, W_o$) | 3,276,800 |
| **RMSNorms** | $8 \times [2 \times 320 + 40] + 320$ | 5,760 |
| **SwiGLU FFN (8 layers)** | $8 \times [3 \times (320 \times 800)]$ ($W_1, W_2, W_3$) | 6,144,000 |
| **Language Model Head** | Tied with token embeddings | 0 (tied) |
| **Total Model Parameters** | | **9,906,560** |

This parameter count was formally verified in `data/clean_minif2f_eval/stage51_audit_report.json`:
```json
{
  "parameter_audit": {
    "passed": true,
    "parameter_count": 9906560,
    "message": "Passed: 9,906,560 <= 10,000,000"
  }
}
```

---

## 3. Training & Inference Execution Modes

ProofBDH supports dual execution modes implemented in [`scripts/ml/proof_bdh.py`](file:///home/gulshansharma/proof_battle/scripts/ml/proof_bdh.py):

### 1. Vectorized Parallel Mode (Training)
When training on full sequences, token interactions are computed in parallel using a causal decay tensor mask:

$$D_{ij} = \lambda^{|i - j|} \cdot \mathbf{1}_{i \ge j}$$

This allows standard backpropagation through time (BPTT) using GPU matrix multiplications without running an explicit sequential loop.

### 2. Recurrent $O(1)$ Mode (Inference & MCTS Search)
During autoregressive generation and tree search, tokens are generated one at a time. The synaptic state $S_t$ is updated in $O(1)$ time per step. No historical KV-cache is allocated or resized.

---

## 4. Code References

- Implementation: [`scripts/ml/proof_bdh.py`](file:///home/gulshansharma/proof_battle/scripts/ml/proof_bdh.py)
- Tokenizer: [`scripts/ml/proof_tokenizer.py`](file:///home/gulshansharma/proof_battle/scripts/ml/proof_tokenizer.py)
- Search Integration: [`scripts/ml/dbf_mcts_prover.py`](file:///home/gulshansharma/proof_battle/scripts/ml/dbf_mcts_prover.py)
