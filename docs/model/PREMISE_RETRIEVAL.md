# BM25 Mathlib Premise Retriever

The premise retrieval subsystem in [`scripts/ml/premise_retriever.py`](file:///home/gulshansharma/proof_battle/scripts/ml/premise_retriever.py) provides sub-5ms semantic retrieval over formal mathematical lemmas in Lean 4's Mathlib library without adding neural parameters.

---

## 1. Design Principles & Zero-Parameter Constraint

Modern dense retrieval models (e.g., DPR, Contriever) introduce 100M+ neural parameters. For ProofBattle, adding external neural weights would violate the strict **$\le 10,000,000$ total parameter budget**.

The BM25 premise retriever achieves competitive retrieval precision using an optimized, domain-specific inverted index:
- **0 Neural Parameters**: Strictly 0 weights added to the model footprint.
- **102,278 Mathlib Lemmas Indexed**: Covers algebra, analysis, number theory, geometry, and combinatorics.
- **Sub-5ms Query Latency**: In-memory inverted index serialized with `pickle` cache for sub-100ms startup.

---

## 2. Math-Aware Tokenization & Unicode Normalization

Standard NLP tokenizers fail on Lean 4 because mathematical notation relies heavily on Unicode symbols, dot-namespaces, and implicit operators.

The premise retriever applies custom math preprocessing:

### A. Unicode Normalization
Lean 4 Unicode operators are mapped to normalized ASCII tokens:

```python
UNICODE_MAP = {
    "≤": " le ",   "≥": " ge ",   "≠": " ne ",
    "√": " sqrt ", "∑": " sum ",  "∏": " prod ",
    "∈": " mem ",  "⊆": " subset ","∧": " and ",
    "∨": " or ",   "¬": " not ",  "→": " imp ",
    "↔": " iff ",  "∀": " forall ","∃": " exists ",
    "ℕ": " Nat ",  "ℤ": " Int ",  "ℝ": " Real ",
    "ℂ": " Complex "
}
```

### B. Dot-Namespace Splitting
Mathlib lemma names encode taxonomy via dot notation (e.g., `Real.sqrt_le_sqrt_iff`). The tokenizer decomposes namespaces into both their component parts (`Real`, `sqrt`, `le`, `iff`) and full identifiers so queries match generic and qualified terms.

---

## 3. Semantic & Type-Aware Reranking

Raw BM25 term frequency often ranks irrelevant lemmas highly if they share common tokens. The retriever applies a multi-factor reranking pass:

1. **Metaprogramming Penalty ($\times 0.10$)**: Suppresses compiler internals containing `.Meta.` or `Elab`.
2. **Derivative Guard ($\times 0.15$)**: Penalizes calculus derivatives unless the theorem explicitly involves calculus.
3. **Domain & Type Alignment ($\times 1.40 - \times 1.50$)**: Boosts lemmas matching the target variable domain (`Real`, `Nat`, `Int`, `Complex`, `Rat`).
4. **Relational Target Alignment ($\times 1.30 - \times 1.50$)**: Boosts equalities (`=`), inequalities (`≤`, `<`), or divisibility (`∣`) based on the goal's top-level relation.
5. **Operator Head Matching ($\times 1.80 - \times 3.0$)**: Multiplies score when target operators like `log`, `sqrt`, or `abs` co-occur (e.g., `Real.log_mul`).

---

## 4. Integration with ProofBDH & MCTS

Retrieved premises are used in two ways:
1. **Context-Conditioned Neural Prompting**: The top-2 retrieved lemma signatures are injected directly into ProofBDH's generation prompt:
   ```text
   <lemmas> {lemma_1}: {sig_1} ; {lemma_2}: {sig_2} </lemmas> <goal> {goal} <plan> proof </plan><tactic>
   ```
   ProofBDH conditions on this context to autoregressively synthesize tactics referencing the lemmas.
2. **Candidate Generation**: Proposes candidate actions for tree exploration.

---

## 5. Code References

- Implementation: [`scripts/ml/premise_retriever.py`](file:///home/gulshansharma/proof_battle/scripts/ml/premise_retriever.py)
- Competition Lemma Dataset: `data/mathlib_competition_lemmas.jsonl`
- Precomputed Index Cache: `data/native_prover/bm25_premise_index.pkl`
