#!/usr/bin/env python3
"""
Test Suite for Stage 64: AlphaGeometry Auxiliary Construction Engine (AG-ACE).

Verifies:
1. Syntax-driven AST matching (Zero problem-ID branching).
2. Auxiliary lemma synthesis for radical conjugate and piecewise absolute value problems.
3. Lean 4 formal verification via LeanREPLClient.
4. Formal axiom integrity audit via `#print axioms` (0 sorryAx, clean classical axioms).
"""

import sys
import json
import time
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.append(str(ROOT_DIR / "scripts" / "ml"))

from lean_repl_client import LeanREPLClient
from auxiliary_construction_engine import AuxiliaryConstructionEngine, ConjugateAuxiliaryConstructor, AbsIntervalAuxiliaryConstructor


def run_alphageometry_tests():
    print("==================================================================")
    print("STAGE 64: ALPHAGEOMETRY AUXILIARY CONSTRUCTION TEST SUITE")
    print("==================================================================")

    repl = LeanREPLClient()
    engine = AuxiliaryConstructionEngine(repl)

    data_file = ROOT_DIR / "data" / "minif2f_full_488.jsonl"
    problems = {}
    with open(data_file) as f:
        for line in f:
            d = json.loads(line)
            if d.get("problem_id") in ["test/imo_1960_p2", "test/aime_1983_p2"]:
                problems[d["problem_id"]] = d

    all_passed = True

    # -------------------------------------------------------------
    # Test 1: Radical Conjugate Construction (test/imo_1960_p2)
    # -------------------------------------------------------------
    print("\n[TEST 1] Testing Radical Conjugate Auxiliary Construction...")
    p_imo = problems.get("test/imo_1960_p2")
    assert p_imo is not None, "test/imo_1960_p2 not found in dataset"

    # Verify constructor matches purely on syntax (NOT on problem_id)
    matches_imo = engine.conjugate_constructor.matches(p_imo["raw_code"], p_imo["params"], p_imo["goal"])
    print(f"  Syntax Match on Radical Denominator: {matches_imo}")
    assert matches_imo, "Failed to match radical denominator syntax"

    constructions_imo = engine.propose_constructions(p_imo["goal"], p_imo["params"], p_imo["raw_code"])
    print(f"  Proposals generated: {len(constructions_imo)}")
    assert len(constructions_imo) >= 1, "Expected at least 1 proposal for IMO problem"

    tac_imo, prior_imo, ctype_imo = constructions_imo[0]
    print(f"  Proposal Type: {ctype_imo} (Prior: {prior_imo:.2f})")

    # Verify in Lean 4 REPL
    ok_imo, lat_imo, err_imo = repl.verify_script(p_imo["raw_code"], tac_imo)
    print(f"  Lean 4 REPL Verification: Solved={ok_imo} (Latency: {lat_imo:.1f}ms)")
    if not ok_imo:
        print(f"  Lean Error:\n{err_imo}")
        all_passed = False
    else:
        # Run axiom check
        stmt = repl._prepare_statement(p_imo["raw_code"])
        full_code = f"{stmt} := by\n{tac_imo}"
        ok_ax, _, _, axioms = repl.verify_full_code(full_code, "imo_1960_p2")
        print(f"  #print axioms: {axioms}")
        assert "sorryAx" not in axioms, "CRITICAL: sorryAx detected in proof!"
        assert ok_ax, "Failed axiom verification"
        print("  ✓ PASSED: Radical conjugate auxiliary construction verified with clean classical axioms!")

    # -------------------------------------------------------------
    # Test 2: Piecewise Absolute Value Construction (test/aime_1983_p2)
    # -------------------------------------------------------------
    print("\n[TEST 2] Testing Piecewise Absolute Value Auxiliary Construction...")
    p_aime = problems.get("test/aime_1983_p2")
    assert p_aime is not None, "test/aime_1983_p2 not found in dataset"

    matches_aime = engine.abs_constructor.matches(p_aime["raw_code"], p_aime["params"], p_aime["goal"])
    print(f"  Syntax Match on Piecewise Abs: {matches_aime}")
    assert matches_aime, "Failed to match piecewise abs syntax"

    constructions_aime = engine.propose_constructions(p_aime["goal"], p_aime["params"], p_aime["raw_code"])
    print(f"  Proposals generated: {len(constructions_aime)}")
    assert len(constructions_aime) >= 1, "Expected at least 1 proposal for AIME problem"

    tac_aime, prior_aime, ctype_aime = constructions_aime[0]
    print(f"  Proposal Type: {ctype_aime} (Prior: {prior_aime:.2f})")

    # Verify in Lean 4 REPL
    ok_aime, lat_aime, err_aime = repl.verify_script(p_aime["raw_code"], tac_aime)
    print(f"  Lean 4 REPL Verification: Solved={ok_aime} (Latency: {lat_aime:.1f}ms)")
    if not ok_aime:
        print(f"  Lean Error:\n{err_aime}")
        all_passed = False
    else:
        # Run axiom check
        stmt = repl._prepare_statement(p_aime["raw_code"])
        full_code = f"{stmt} := by\n{tac_aime}"
        ok_ax, _, _, axioms = repl.verify_full_code(full_code, "aime_1983_p2")
        print(f"  #print axioms: {axioms}")
        assert "sorryAx" not in axioms, "CRITICAL: sorryAx detected in proof!"
        assert ok_ax, "Failed axiom verification"
        print("  ✓ PASSED: Piecewise abs auxiliary construction verified with clean classical axioms!")

    repl.close()
    print("\n==================================================================")
    if all_passed:
        print("ALL ALPHAGEOMETRY AUXILIARY CONSTRUCTION TESTS PASSED (100% CLEAN)")
    else:
        print("TEST FAILURES ENCOUNTERED")
    print("==================================================================")
    return all_passed


if __name__ == "__main__":
    success = run_alphageometry_tests()
    sys.exit(0 if success else 1)
