# Neuro-Symbolic Lean 4 Autoformalization Engine

The autoformalization subsystem in [`scripts/ml/autoformalizer.py`](file:///home/gulshansharma/proof_battle/scripts/ml/autoformalizer.py) translates informal mathematical statements, LaTeX equations, and contest problems into verified, machine-checked Lean 4 theorem declarations.

---

## 1. The Autoformalization Challenge

Translating human mathematics to Lean 4 requires more than surface NLP text translation:
1. **Type Domain Assignment**: In informal text, variables like $x, y$ often omit explicit domains. Lean 4 requires explicit binders (e.g., `(x y : ℝ)` or `(n : ℕ)`).
2. **Implicit Hypotheses**: Informal statements often assume side conditions (e.g., $x > 0$ for logarithms, $a + b + c = 0$) that must be captured as named hypotheses `(h : ...)`.
3. **Formal Elaboration Soundness**: Any generated theorem must be syntactically valid and pass the Lean 4 kernel's type checker without errors.

---

## 2. Pipeline Stages

```text
    Informal Math / LaTeX:
    "For positive reals a and b, (a + b)^2 - 4ab = (a - b)^2"
                              |
                              v
    +-------------------------------------------------------+
    |  Stage 1: Domain & Variable Extraction                |
    |  Detects variables (a, b) and type annotations (ℝ)    |
    |  Result binders: (a b : ℝ)                            |
    +---------------------------+---------------------------+
                                |
                                v
    +-------------------------------------------------------+
    |  Stage 2: Canonical Pattern Recognition & LaTeX Map   |
    |  Maps LaTeX \frac, exponents, and operator heads      |
    |  Normalizes to Lean 4 syntax:                         |
    |  (a + b)^2 - 4 * a * b = (a - b)^2                   |
    +---------------------------+---------------------------+
                                |
                                v
    +-------------------------------------------------------+
    |  Stage 3: Kernel Elaboration Verification             |
    |  Checks syntax via LeanREPLClient:                    |
    |  theorem target (a b : ℝ) : ... := by sorry          |
    +---------------------------+---------------------------+
                                |
                                v
    Formal Lean 4 Challenge Output:
    Verified Theorem Ready for ProofBattle Match Arena
```

---

## 3. Supported Mathematical Domains

The autoformalizer recognizes standard problem templates across four primary domains:
- **Algebra**: Quadratic identities, binomial expansions, AM-GM inequalities, sums of cubes, logarithmic and exponential identities.
- **Number Theory**: Divisibility properties, prime characteristics, modular arithmetic relations, successor/Peano inductions.
- **Geometry**: Pythagorean relations, metric distances, triangle inequalities.
- **Analysis**: Convex combinations, Cauchy-Schwarz instances, polynomial bounds.

---

## 4. HTTP API Endpoint

The autoformalization engine is exposed via the simulation server on port 8088:

- **Endpoint**: `POST /api/autoformalize`
- **Request Body**:
  ```json
  {
    "statement": "For real numbers a and b, (a + b)^2 - 4 * a * b = (a - b)^2"
  }
  ```
- **Response**:
  ```json
  {
    "formalized": true,
    "theorem_name": "auto_theorem_1",
    "binders": "(a b : ℝ)",
    "goal": "(a + b)^2 - 4 * a * b = (a - b)^2",
    "full_declaration": "theorem auto_theorem_1 (a b : ℝ) : (a + b)^2 - 4 * a * b = (a - b)^2 := by sorry",
    "verified_by_kernel": true,
    "elapsed_ms": 28.4
  }
  ```

---

## 5. Code References

- Engine Implementation: [`scripts/ml/autoformalizer.py`](file:///home/gulshansharma/proof_battle/scripts/ml/autoformalizer.py)
- Server Integration: [`scripts/ml/simulation_server.py`](file:///home/gulshansharma/proof_battle/scripts/ml/simulation_server.py)
