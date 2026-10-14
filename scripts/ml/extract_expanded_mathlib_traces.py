#!/usr/bin/env python3
"""
Stage 57: High-Throughput Mathlib 4 Trace Miner & Dataset Expander.
"""
import os, sys, json, random
from pathlib import Path
from typing import List, Dict, Any, Set, Tuple

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
DATA_DIR = ROOT_DIR / "data"
MATHLIB_DIR = ROOT_DIR / "lean" / "Mathlib"
TRACES_DIR = DATA_DIR / "clean_mathlib_traces"
TRACES_DIR.mkdir(parents=True, exist_ok=True)

MINIF2F_FILE = DATA_DIR / "minif2f_full_488.jsonl"
LEMMAS_FILE = DATA_DIR / "mathlib_competition_lemmas.jsonl"
OUT_EXPANDED_FILE = TRACES_DIR / "stage57_expanded_train.jsonl"
MANIFEST_FILE = TRACES_DIR / "stage57_manifest.json"

def load_minif2f_quarantine() -> Tuple[Set[str], Set[str]]:
    quarantined_names = set()
    quarantined_goals = set()
    if MINIF2F_FILE.exists():
        with open(MINIF2F_FILE, "r", encoding="utf-8") as f:
            for line in f:
                if not line.strip(): continue
                try:
                    d = json.loads(line)
                    pid = d.get("problem_id", "")
                    name = pid.split("/")[-1].lower().strip()
                    if name: quarantined_names.add(name)
                    goal = d.get("goal", "").strip().lower()
                    if goal: quarantined_goals.add(goal)
                except Exception: pass
    print(f"Quarantine Loaded: {len(quarantined_names)} names, {len(quarantined_goals)} goals.")
    return quarantined_names, quarantined_goals

def mine_mathlib_files(quarantined_names: Set[str], quarantined_goals: Set[str], max_traces: int = 25000) -> List[Dict[str, Any]]:
    files = list(MATHLIB_DIR.glob("**/*.lean"))
    random.seed(42)
    random.shuffle(files)
    traces = []
    seen = set()
    print(f"Scanning {len(files)} Mathlib 4 source files...")

    for f_idx, file_path in enumerate(files, 1):
        if len(traces) >= max_traces: break
        try:
            with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
                lines = f.readlines()
            i = 0
            while i < len(lines):
                line = lines[i].strip()
                is_thm = line.startswith("theorem ") or line.startswith("lemma ")
                if (is_thm or line.startswith("protected theorem ") or line.startswith("protected lemma ")) and ":=" in line:
                    parts = line.split(":=")
                    head = parts[0].strip()
                    tail = parts[1].strip()
                    tokens = head.split()
                    name = tokens[1] if len(tokens) > 1 else ""
                    clean_name = name.split("(")[0].split("{")[0].split(":")[0].strip()
                    if clean_name.lower() in quarantined_names:
                        i += 1
                        continue
                    if ":" in head:
                        subparts = head.split(":", 1)
                        decl_part = subparts[0].strip()
                        goal_part = subparts[1].strip()
                        binders = " ".join(decl_part.split()[2:]).strip()
                    else:
                        binders = ""
                        goal_part = ""
                    if not goal_part or len(goal_part) > 250 or goal_part.lower() in quarantined_goals:
                        i += 1
                        continue
                    tacs = []
                    if "by" in tail:
                        after_by = tail.split("by", 1)[1].strip()
                        if after_by and not after_by.startswith("--"):
                            tacs.append(after_by)
                        for j in range(i + 1, min(i + 8, len(lines))):
                            next_line = lines[j]
                            stripped = next_line.strip()
                            if not stripped or stripped.startswith("--") or stripped.startswith("/-"): continue
                            if stripped.startswith("theorem ") or stripped.startswith("lemma ") or stripped.startswith("def ") or stripped.startswith("instance "): break
                            if next_line.startswith("  ") or next_line.startswith("\t"):
                                tacs.append(stripped)
                            else:
                                break
                    elif tail and not tail.startswith("by") and len(tail) < 150:
                        tacs.append(f"exact {tail}")
                    if tacs:
                        proof_body = "\n".join(tacs[:3]).strip()
                        if proof_body and len(proof_body) < 250:
                            if binders:
                                text = f"<hyp> {binders} </hyp> <goal> {goal_part} <plan> proof </plan><tactic> {proof_body} <qed>"
                            else:
                                text = f"<goal> {goal_part} <plan> proof </plan><tactic> {proof_body} <qed>"
                            if text not in seen:
                                seen.add(text)
                                traces.append({
                                    "type": "mathlib_file_proof",
                                    "name": clean_name,
                                    "module": str(file_path.relative_to(ROOT_DIR)),
                                    "text": text,
                                    "num_steps": len(tacs[:3])
                                })
                i += 1
        except Exception:
            continue
    print(f"Extracted {len(traces):,} high-quality proof traces from Mathlib source files.")
    return traces

def mine_lemma_premise_traces(quarantined_names: Set[str], quarantined_goals: Set[str], max_lemmas: int = 15000) -> List[Dict[str, Any]]:
    if not LEMMAS_FILE.exists(): return []
    traces = []
    seen = set()
    with open(LEMS := LEMMAS_FILE, "r", encoding="utf-8") as f:
        for line in f:
            if len(traces) >= max_lemmas: break
            if not line.strip(): continue
            try:
                d = json.loads(line)
                name = d.get("name", "")
                raw_name = d.get("raw_name", "")
                sig = d.get("signature", "")
                if name.lower() in quarantined_names or raw_name.lower() in quarantined_names: continue
                if ":" not in sig or len(sig) > 250: continue
                parts = sig.split(":", 1)
                binders = parts[0].strip()
                goal = parts[1].strip()
                if not goal or goal.lower() in quarantined_goals: continue
                if ":=" in goal: goal = goal.split(":=")[0].strip()
                if "/-" in goal: goal = goal.split("/-")[0].strip()
                if binders:
                    text = f"<hyp> {binders} </hyp> <goal> {goal} <plan> proof </plan><tactic> exact {name} <qed>"
                else:
                    text = f"<goal> {goal} <plan> proof </plan><tactic> exact {name} <qed>"
                if text not in seen:
                    seen.add(text)
                    traces.append({"type": "premise_application", "name": name, "text": text, "num_steps": 1})
            except Exception: continue
    print(f"Extracted {len(traces):,} lemma premise application traces.")
    return traces

def build_stage57_expanded_dataset():
    quarantined_names, quarantined_goals = load_minif2f_quarantine()
    existing_records = []
    existing_files = [TRACES_DIR / "multistep_train_traces.jsonl", TRACES_DIR / "stage51_unified_train.jsonl"]
    seen_texts = set()
    for p in existing_files:
        if p.exists():
            with open(p, "r", encoding="utf-8") as f:
                for line in f:
                    if not line.strip(): continue
                    try:
                        d = json.loads(line)
                        text = d.get("text") or d.get("formatted_trace") or ""
                        if text and text not in seen_texts:
                            seen_texts.add(text)
                            existing_records.append(d)
                    except Exception: pass
    print(f"Loaded {len(existing_records):,} existing curated traces.")
    file_traces = mine_mathlib_files(quarantined_names, quarantined_goals, max_traces=20000)
    lemma_traces = mine_lemma_premise_traces(quarantined_names, quarantined_goals, max_lemmas=12000)
    combined = list(existing_records)
    for r in file_traces:
        if r["text"] not in seen_texts:
            seen_texts.add(r["text"])
            combined.append(r)
    for r in lemma_traces:
        if r["text"] not in seen_texts:
            seen_texts.add(r["text"])
            combined.append(r)
    random.seed(42)
    random.shuffle(combined)
    print(f"Total Combined Expanded Dataset: {len(combined):,} traces (from {len(existing_records):,} base).")
    with open(OUT_EXPANDED_FILE, "w", encoding="utf-8") as f:
        for r in combined: f.write(json.dumps(r) + "\n")
    manifest = {
        "dataset_name": "stage57_expanded_train",
        "total_traces": len(combined),
        "existing_base_traces": len(existing_records),
        "mined_mathlib_file_traces": len(file_traces),
        "mined_lemma_premise_traces": len(lemma_traces),
        "quarantined_minif2f_names": len(quarantined_names),
        "quarantined_minif2f_goals": len(quarantined_goals),
        "output_file": str(OUT_EXPANDED_FILE.relative_to(ROOT_DIR))
    }
    with open(MANIFEST_FILE, "w", encoding="utf-8") as f: json.dump(manifest, f, indent=2)
    print(f"Saved expanded dataset to {OUT_EXPANDED_FILE.name}")

if __name__ == "__main__":
    build_stage57_expanded_dataset()
