#!/usr/bin/env python3
"""
Phase 1 Problem Extractor for ProofBattle.
Parses local Mathlib Lean files (lean/Mathlib/**/*.lean) and extracts candidate
problems with verified fine-grained module imports, goals, categories, heuristic difficulty,
and canonical proof bodies.
"""

import os
import re
import sys
import json
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
LEAN_DIR = REPO_ROOT / "lean"
MATHLIB_DIR = LEAN_DIR / "Mathlib"
OUTPUT_FILE = REPO_ROOT / "problems_import.jsonl"

PROP_OPERATORS = [
    "=", "≠", "≤", "<", "≥", ">", "↔", "∧", "∨", "¬", "∀", "∃", "→", "∈", "⊆", "∣"
]

ALLOWED_HEADS = {
    "intro", "intros", "intro1", "intro2", "rename", "rename_i", "obtain", "rcases", "rintro",
    "match_cases", "match", "use", "exact", "exact_mod_cast", "refine", "apply", "constructor",
    "constructors", "cases", "case_tac", "on_left", "on_right", "rfl", "assumption",
    "assumption_mod_cast", "trivial", "trivial_mod_cast", "simp", "simpa", "simp_all",
    "simp_arith", "simp_rw", "dsimp", "norm_num", "norm_num1", "norm_cast", "push_cast",
    "decide", "ring", "ring_nf", "ring_exp", "ring!", "field_simp", "linarith", "nlinarith",
    "abel", "positivity", "omega", "grind", "aesop", "haveI", "have", "show", "suffices",
    "subst", "substs", "induction", "induction'", "inductionOn", "cases'", "rw", "rw'", "erw",
    "nth_rw", "left", "right", "guard_hyp", "guard_target", "unfold", "delta", "change",
    "convert", "unfold_using", "simp_where_generalize", "done", "trivial_1", "focus",
    "all_goals", "any_goals", "repeat", "try", "first", "solve", "ext", "ext1", "lift",
    "split_ifs", "split", "congr", "congr'", "gcongr", "revert", "clear", "symm", "trans",
    "tauto", "fin_cases", "interval_cases", "choose", "contrapose", "exfalso", "by_contra",
    "by_cases", "generalize", "apply_fun"
}

FORBIDDEN_TOKENS = [
    "sorry", "sorryAx", "admit", "stop", "native_decide", "unsafe", "extern",
    "#eval", "#check", "#print", "#exit", "#load", "#help", "#guard_msgs",
    "IO.println", "IO.getEnv", "System.Process", "@["
]

def load_available_oleans() -> set[str]:
    available = set()
    for root, _, files in os.walk(LEAN_DIR / ".lake"):
        for f in files:
            if f.endswith(".olean"):
                p = Path(root) / f
                parts = p.parts
                if "lib" in parts and "lean" in parts:
                    idx = len(parts) - 1 - list(reversed(parts)).index("lean")
                    mod = ".".join(parts[idx+1:]).replace(".olean", "")
                    available.add(mod)
    return available

def determine_category(file_path: Path) -> str:
    path_str = str(file_path)
    if "Mathlib/Data/Nat" in path_str or "Algebra/Ring/Nat" in path_str:
        return "nat_arithmetic"
    elif "Mathlib/Logic" in path_str:
        return "logic"
    elif "Mathlib/Algebra/Order" in path_str or "Mathlib/Order" in path_str:
        return "order"
    elif "Mathlib/Data/Finset" in path_str or "Mathlib/Data/Set" in path_str:
        return "finset"
    elif "Mathlib/Analysis" in path_str:
        return "analysis"
    elif "Mathlib/Algebra" in path_str:
        return "algebra"
    elif "Mathlib/Topology" in path_str:
        return "topology"
    else:
        return "general"

def estimate_difficulty(goal: str, proof: str) -> int:
    proof_lower = proof.lower()
    line_count = len([l for l in proof.splitlines() if l.strip()])
    
    if any(k in proof_lower for k in ["measure", "integral", "hausdorff", "compact", "uniform_continuous"]):
        return min(10, 7 + (line_count // 3))
    if "induction" in proof_lower or "obtain" in proof_lower or "rcases" in proof_lower:
        return min(6, 4 + (line_count // 2))
    if "linarith" in proof_lower or "norm_num" in proof_lower or "aesop" in proof_lower:
        return 4 if line_count > 2 else 3
    if "ring" in proof_lower or "omega" in proof_lower:
        return 2 if line_count == 1 else 3
    if "simp" in proof_lower or "rfl" in proof_lower or "decide" in proof_lower:
        return 1 if line_count == 1 else 2
    return min(10, max(1, 2 + line_count))

def clean_binders(params: str) -> str:
    # Replace {a b : T} with (a b : T)
    p = params.replace("{", "(").replace("}", ")")
    # Clean universe/sort annotations
    p = re.sub(r'Sort\s*\*?', 'Type', p)
    p = re.sub(r'Type\s*\*?', 'Type', p)
    p = re.sub(r'Type\s+[u-z0-9_\']+', 'Type', p)
    # Remove leading/trailing spaces
    return " ".join(p.split())

def is_proof_allowed(proof_body: str) -> bool:
    for forbidden in FORBIDDEN_TOKENS:
        if forbidden in proof_body:
            return False
    
    # Check first tactic head
    lines = [l.strip() for l in proof_body.splitlines() if l.strip()]
    if not lines:
        return False
    first_line = lines[0]
    first_token = re.split(r'[\s;(\[{⟨·|]', first_line)[0]
    if first_token and first_token not in ALLOWED_HEADS:
        return False
    return True

def extract_from_file(file_path: Path, available_modules: set[str]):
    try:
        content = file_path.read_text(encoding="utf-8", errors="ignore")
    except Exception:
        return []

    rel_path = file_path.relative_to(LEAN_DIR)
    module_name = str(rel_path).replace("/", ".").replace(".lean", "")

    # Require module itself to have a built olean
    if module_name not in available_modules:
        return []

    # Fine-grained imports
    imports = [f"import {module_name}"]
    for line in content.splitlines():
        trimmed = line.strip()
        if trimmed.startswith("import ") and not trimmed == "import Mathlib":
            imp_module = trimmed[7:].strip()
            if imp_module in available_modules and trimmed not in imports:
                imports.append(trimmed)
                if len(imports) >= 4:
                    break

    category = determine_category(file_path)
    records = []

    lines = content.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        # Match theorem or lemma line
        m = re.match(r'^(?:@\[[^\]]+\]\s+)?(?:protected\s+)?(?:theorem|lemma)\s+([a-zA-Z0-9_\'.]+)\s*(.*)', line)
        if m and ":=" in line and "by" in line:
            name = m.group(1)
            rest = line[m.end(1):].strip()
            
            if ":=" in rest:
                parts = rest.split(":=", 1)
                sig = parts[0].strip()
                after_assign = parts[1].strip()
                
                if ":" in sig:
                    sig_parts = sig.rsplit(":", 1)
                    params = sig_parts[0].strip()
                    raw_goal = sig_parts[1].strip()
                else:
                    raw_goal = sig.strip()
                    params = ""

                # Collect proof body lines
                proof_lines = []
                if after_assign.startswith("by"):
                    inline_proof = after_assign[2:].strip()
                    if inline_proof:
                        proof_lines.append(inline_proof)
                    
                    j = i + 1
                    while j < len(lines):
                        next_line = lines[j]
                        if not next_line.strip():
                            proof_lines.append("")
                            j += 1
                            continue
                        if next_line.startswith("  ") or next_line.startswith("\t"):
                            proof_lines.append(next_line.strip())
                            j += 1
                        else:
                            break
                    i = j - 1

                proof_body = "\n".join(proof_lines).strip()
                
                # Check validity
                if raw_goal and any(op in raw_goal for op in PROP_OPERATORS) and proof_body:
                    if is_proof_allowed(proof_body):
                        # Format goal
                        if params:
                            clean_p = clean_binders(params)
                            goal = f"∀ {clean_p}, {raw_goal}"
                        else:
                            goal = raw_goal

                        # Clean whitespace
                        goal = " ".join(goal.split())
                        # Avoid complex binders that break Lean parsing
                        if "(" in goal and ")" in goal or not params:
                            if len(goal) < 300 and len(proof_body) < 1000 and len(proof_body.splitlines()) < 60:
                                slug_name = name.replace(".", "_")
                                slug = f"{category}/{slug_name}".lower()
                                diff = estimate_difficulty(goal, proof_body)
                                statement = f"theorem goal : {goal} := by"
                                
                                hint = None
                                if "simp" in proof_body:
                                    hint = "try simp"
                                elif "omega" in proof_body:
                                    hint = "try omega"
                                elif "ring" in proof_body:
                                    hint = "try ring"
                                elif "linarith" in proof_body:
                                    hint = "try linarith"

                                records.append({
                                    "slug": slug,
                                    "goal": goal,
                                    "imports": imports,
                                    "statement": statement,
                                    "canonical_proof": proof_body,
                                    "difficulty": max(1, min(10, diff)),
                                    "category": category,
                                    "tactic_hint": hint,
                                    "source_theorem": name,
                                    "source_url": None,
                                })
        i += 1

    return records

def main():
    print(f"Scanning available olean modules from {LEAN_DIR / '.lake'}...")
    available_modules = load_available_oleans()
    print(f"Found {len(available_modules)} pre-built oleans.")

    print(f"Traversing Mathlib directory: {MATHLIB_DIR}...")
    if not MATHLIB_DIR.exists():
        print(f"Error: Mathlib directory not found at {MATHLIB_DIR}")
        sys.exit(1)

    all_records = []
    seen_slugs = set()

    for root, _, files in os.walk(MATHLIB_DIR):
        for f in sorted(files):
            if f.endswith(".lean"):
                lean_file = Path(root) / f
                records = extract_from_file(lean_file, available_modules)
                for r in records:
                    if r["slug"] not in seen_slugs:
                        seen_slugs.add(r["slug"])
                        all_records.append(r)

    print(f"Extracted {len(all_records)} candidate problems from local Mathlib source.")
    
    # Sort deterministically
    all_records.sort(key=lambda r: (r["category"], r["difficulty"], r["slug"]))

    with open(OUTPUT_FILE, "w", encoding="utf-8") as out:
        for r in all_records:
            out.write(json.dumps(r, ensure_ascii=False) + "\n")

    print(f"Wrote {len(all_records)} candidates to {OUTPUT_FILE}")

if __name__ == "__main__":
    main()
