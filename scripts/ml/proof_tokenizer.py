#!/usr/bin/env python3
"""
Custom Formal Mathematics and Lean 4 Tokenizer (ProofTokenizer).

Designed specifically for Lean 4 theorem proving syntax, Unicode mathematical symbols,
and structured decomposition tags without third-party dependencies.
"""

import json
import re
from pathlib import Path
from typing import List, Dict, Optional, Union

SPECIAL_TOKENS = [
    "<pad>",
    "<unk>",
    "<bos>",
    "<eos>",
    "<goal>",
    "<hyp>",
    "<plan>",
    "</plan>",
    "<tactic>",
    "<qed>",
    "<err>",
]

CORE_LEAN_KEYWORDS = [
    "theorem", "lemma", "def", "example", "by", "intro", "intros", "exact",
    "apply", "rw", "simp", "ring", "linarith", "nlinarith", "omega",
    "positivity", "norm_num", "aesop", "tauto", "constructor", "induction",
    "cases", "rcases", "obtain", "have", "show", "calc", "refine", "congr",
    "ext", "fun", "let", "if", "then", "else", "match", "with", "open",
    "import", "variable", "namespace", "section", "end", "set_option",
    "attribute", "linear_combination", "subst", "subst_vars", "ring_nf",
    "norm_cast", "push_cast", "refl", "rfl", "contradiction", "trivial",
    "decide", "split", "left", "right", "Mathlib", "Real", "Nat", "Int",
    "Complex", "Finset", "Set", "List", "Prop", "Type", "Sort", "Sort*",
]

CORE_MATH_SYMBOLS = [
    "∀", "∃", "→", "↔", "∧", "∨", "¬", "≤", "≥", "≠", "ℕ", "ℤ", "ℚ", "ℝ", "ℂ",
    "∈", "∉", "⊆", "∪", "∩", "∑", "∏", "·", "⟨", "⟩", "∣", "∤", "√", "π", "⊥", "⊤",
    "h₀", "h₁", "h₂", "h₃", "h₄", "h₅", "h₆", "h₇", "h₈", "h₉",
    ":=", ":", ";", ",", "(", ")", "[", "]", "{", "}", "+", "-", "*", "/", "^",
    "=", "<", ">", "|", "!", "?", "\\", ".", "'", "\"", "\n", "\t", " "
]

COMMON_SUBWORDS = [
    "add", "mul", "sub", "div", "pow", "le", "lt", "ge", "gt", "eq", "ne",
    "dvd", "gcd", "mod", "card", "sum", "sqrt", "log", "zero", "one", "two",
    "succ", "comm", "assoc", "distrib", "cancel", "inv", "neg", "pos", "nonneg",
    "abs", "min", "max", "Icc", "Ico", "Ioc", "Ioo", "univ", "empty",
    "Finset.Icc", "Finset.sum", "Real.sqrt", "Real.log", "Nat.succ", "Nat.dvd",
    "Nat.add", "Nat.mul", "Nat.sub", "Int.ofNat", "Complex.I", "Complex.I_sq",
    "conjunction", "inequality", "equality", "linear", "quadratic", "polynomial",
]


class ProofTokenizer:
    """Specialized Tokenizer for formal Lean 4 statements and proofs."""

    def __init__(self, vocab_path: Optional[str] = None):
        self.vocab: Dict[str, int] = {}
        self.id_to_token: Dict[int, str] = {}
        self.pattern: Optional[re.Pattern] = None

        if vocab_path and Path(vocab_path).exists():
            self.load(vocab_path)
        else:
            self._build_default_vocab()

    def _build_default_vocab(self):
        """Builds default formal mathematics vocabulary."""
        idx = 0
        for tok in SPECIAL_TOKENS:
            if tok not in self.vocab:
                self.vocab[tok] = idx
                idx += 1

        for kw in CORE_LEAN_KEYWORDS:
            if kw not in self.vocab:
                self.vocab[kw] = idx
                idx += 1

        for sym in CORE_MATH_SYMBOLS:
            if sym not in self.vocab:
                self.vocab[sym] = idx
                idx += 1

        for sw in COMMON_SUBWORDS:
            if sw not in self.vocab:
                self.vocab[sw] = idx
                idx += 1

        # Add single ASCII characters (0-127) as fallback
        for c in range(128):
            ch = chr(c)
            if ch not in self.vocab:
                self.vocab[ch] = idx
                idx += 1

        # Common Latin / Greek letters and digits
        alphabet = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_αβγδεθλμπστω"
        for ch in alphabet:
            if ch not in self.vocab:
                self.vocab[ch] = idx
                idx += 1

        self.id_to_token = {v: k for k, v in self.vocab.items()}
        self._compile_regex()

    def _compile_regex(self):
        """Compiles regex matching longest tokens first."""
        sorted_tokens = sorted(
            [t for t in self.vocab.keys() if t not in ["<unk>", "<pad>"]],
            key=lambda x: len(x),
            reverse=True
        )
        escaped = [re.escape(t) for t in sorted_tokens]
        # Match tokens, or any single character
        self.pattern = re.compile("|".join(escaped) + "|.", re.DOTALL)

    @property
    def vocab_size(self) -> int:
        return len(self.vocab)

    @property
    def pad_id(self) -> int:
        return self.vocab["<pad>"]

    @property
    def unk_id(self) -> int:
        return self.vocab["<unk>"]

    @property
    def bos_id(self) -> int:
        return self.vocab["<bos>"]

    @property
    def eos_id(self) -> int:
        return self.vocab["<eos>"]

    def encode(self, text: str, add_bos: bool = True, add_eos: bool = True) -> List[int]:
        """Encodes string to list of token IDs."""
        if not self.pattern:
            self._compile_regex()

        tokens = []
        if add_bos:
            tokens.append(self.bos_id)

        matches = self.pattern.findall(text)
        for m in matches:
            tokens.append(self.vocab.get(m, self.unk_id))

        if add_eos:
            tokens.append(self.eos_id)

        return tokens

    def decode(self, ids: List[int], skip_special: bool = False) -> str:
        """Decodes token IDs back to string."""
        chars = []
        for i in ids:
            tok = self.id_to_token.get(i, "")
            if skip_special and tok in SPECIAL_TOKENS:
                continue
            chars.append(tok)
        return "".join(chars)

    def save(self, path: str):
        """Saves vocabulary to disk."""
        p = Path(path)
        p.parent.mkdir(parents=True, exist_ok=True)
        with open(p, "w", encoding="utf-8") as f:
            json.dump({
                "vocab": self.vocab,
                "vocab_size": len(self.vocab)
            }, f, ensure_ascii=False, indent=2)

    def load(self, path: str):
        """Loads vocabulary from disk."""
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
            self.vocab = data["vocab"]
            self.id_to_token = {int(v): k for k, v in self.vocab.items()}
        self._compile_regex()


if __name__ == "__main__":
    tokenizer = ProofTokenizer()
    test_str = "<goal> ∀ n : ℕ, n + 0 = n <hyp> h₀ : 0 < n <plan> Induction on n </plan><tactic> induction n with | zero => rfl | succ n ih => simp [ih] <qed>"
    enc = tokenizer.encode(test_str)
    dec = tokenizer.decode(enc)
    print("Vocab size:", tokenizer.vocab_size)
    print("Tokens count:", len(enc))
    print("Encoded IDs:", enc[:20], "...")
    print("Decoded match:", dec == f"<bos>{test_str}<eos>")
    out_dir = Path(__file__).resolve().parent.parent.parent / "data/native_prover"
    tokenizer.save(str(out_dir / "vocab.json"))
    print("Saved vocab to:", out_dir / "vocab.json")
