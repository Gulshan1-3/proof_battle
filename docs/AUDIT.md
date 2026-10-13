# ProofBattle AI Prover Audit

The technical audit evaluating ProofBDH, DBF-MCTS search, benchmark normalization, anti-contamination, and Lean 4 soundness is located at:

**[Technical Audit Report](model/AUDIT.md)**

---

### Navigation
- **Executive Summary & Scope**: [Executive Summary](model/AUDIT.md#1-executive-summary--audit-scope)
- **Neural Model vs Prover System**: [Architectural Clarification](model/AUDIT.md#2-architectural-clarification-neural-model-vs-complete-prover-system)
- **Learned Tactic Selection vs Deduction**: [Theoretical Framing](model/AUDIT.md#3-theoretical-framing-learned-tactic-selection-vs-deduction)
- **Test 1: Pure Neural Ablation (Pass@1, Pass@8, 0 Search/Templates)**: [Test 1 Results](model/AUDIT.md#4-test-1-pure-neural-performance-ablation-of-all-non-neural-components)
- **Test 2: Contamination Audit (0 Leakages Found)**: [Test 2 Audit](model/AUDIT.md#5-test-2-contamination-and-indirect-overlap-audit)
- **Test 3: Generalization Beyond miniF2F (AIME & Duel Challenges)**: [Test 3 Evidence](model/AUDIT.md#6-test-3-generalization-beyond-minif2f)
- **Efficiency Normalization vs ReProver & GPT-f**: [Normalization Matrix](model/AUDIT.md#7-efficiency-normalization-comparison-with-reprover-and-gpt-f)
- **Lean Kernel Soundness & Axiom Audit (0 `sorryAx`)**: [Axiom Verification](model/AUDIT.md#8-soundness--formal-kernel-axiom-verification)
- **Auditor Q&A Matrix**: [Q&A Table](model/AUDIT.md#9-audit-question-and-answer-matrix)
- **Reproduction Guide**: [Reproduction Commands](model/AUDIT.md#10-reproduction-steps)
