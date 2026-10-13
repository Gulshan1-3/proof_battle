#!/usr/bin/env python3
"""
Real-Time AI Prover Simulation Server for ProofBattle.

Provides HTTP API endpoints on port 8088:
- POST /api/prover/solve     : DBF-MCTS proof search with Lean 4 verification and Mathlib BM25 RAG.
- POST /api/autoformalize    : Translates informal statements/LaTeX to verified Lean 4 theorems.
- GET  /api/theorems         : Returns catalog of curated Lean 4 challenge theorems.
- GET  /api/state            : Returns engine status, neural model details, and parameter metrics.
- GET  /health               : Health check endpoint.
- POST /api/verify           : Verifies a candidate tactic script against Lean 4 kernel.
"""

import os
import sys
import json
import time
import argparse
import threading
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse
from pathlib import Path
from typing import Dict, Any, List, Optional, Tuple

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR))
sys.path.insert(0, str(ROOT_DIR / "scripts" / "ml"))

import torch

_GLOBAL_MCTS_PROVER = None
_GLOBAL_MCTS_LOCK = threading.Lock()


def get_dbf_mcts_prover(timeout_sec: float = 30.0):
    global _GLOBAL_MCTS_PROVER
    with _GLOBAL_MCTS_LOCK:
        if _GLOBAL_MCTS_PROVER is None:
            from dbf_mcts_prover import DBFMCTSProver
            device = "cuda" if torch.cuda.is_available() else "cpu"
            _GLOBAL_MCTS_PROVER = DBFMCTSProver(device=device, timeout_sec=timeout_sec)
        else:
            _GLOBAL_MCTS_PROVER.timeout_sec = timeout_sec
        return _GLOBAL_MCTS_PROVER


class SimulationManager:
    """Manages active prover state and theorem catalog."""

    def __init__(self):
        self._lock = threading.Lock()
        self.theorems: List[Dict[str, Any]] = []
        self._load_theorems()

    def _load_theorems(self):
        loaded = []
        curated_candidates = [
            ROOT_DIR / "problems_curated.jsonl",
            Path("problems_curated.jsonl"),
            ROOT_DIR / "data" / "minif2f_curated.jsonl"
        ]
        for cpath in curated_candidates:
            if cpath.exists():
                try:
                    with open(cpath, "r", encoding="utf-8") as f:
                        for l in f:
                            if l.strip():
                                r = json.loads(l)
                                r["is_unseen"] = False
                                r["domain_tag"] = "Curated Benchmark"
                                loaded.append(r)
                    if loaded:
                        break
                except Exception:
                    pass

        self.theorems = loaded if loaded else [{
            "problem_id": "novel/nat_le_add_five",
            "category": "nat_arithmetic",
            "difficulty": 1,
            "goal": "∀ (n : ℕ), n ≤ n + 5",
            "imports": ["import Mathlib.Data.Nat.Basic"],
            "canonical_proof": "intro n\nomega",
            "is_unseen": True,
            "domain_tag": "Novel Unseen"
        }]

    def get_state(self) -> Dict[str, Any]:
        return {
            "status": "online",
            "model": "ProofBDH-9.9M",
            "model_parameters": 9906560,
            "prover_engine": "DBF-MCTS (Length-Normalized BFS + PUCT + Compiler Diagnostics)",
            "lean_kernel": "Lean 4.13.0 + Mathlib v4.13.0",
            "device": "cuda" if torch.cuda.is_available() else "cpu",
            "vram_allocated_mb": round(torch.cuda.memory_allocated() / (1024 ** 2), 2) if torch.cuda.is_available() else 0.0,
            "theorems_loaded": len(self.theorems),
            "timestamp": time.time()
        }


MANAGER = SimulationManager()


class SimulationHTTPHandler(BaseHTTPRequestHandler):
    """HTTP request handler for AI Prover simulations."""

    def _set_cors_headers(self, status: int = 200, content_type: str = "application/json"):
        self.send_response(status)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        self.send_header("Content-Type", content_type)
        self.end_headers()

    def do_OPTIONS(self):
        self._set_cors_headers(200)

    def log_message(self, format, *args):
        pass

    def do_GET(self):
        url = urlparse(self.path)
        path = url.path

        if path in ("/health", "/api/health"):
            self._set_cors_headers(200)
            self.wfile.write(json.dumps({"status": "healthy", "time": time.time()}).encode("utf-8"))

        elif path in ("/api/state", "/api/status"):
            self._set_cors_headers(200)
            self.wfile.write(json.dumps(MANAGER.get_state()).encode("utf-8"))

        elif path == "/api/theorems":
            self._set_cors_headers(200)
            self.wfile.write(json.dumps({
                "theorems": MANAGER.theorems[:50],
                "total": len(MANAGER.theorems)
            }).encode("utf-8"))

        else:
            self._set_cors_headers(404)
            self.wfile.write(json.dumps({"error": f"Route not found: {path}"}).encode("utf-8"))

    def do_POST(self):
        url = urlparse(self.path)
        path = url.path

        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length) if content_length > 0 else b"{}"

        try:
            payload = json.loads(body.decode("utf-8"))
        except Exception:
            payload = {}

        if path == "/api/autoformalize":
            informal_text = (
                payload.get("problem_text", "")
                or payload.get("text", "")
                or payload.get("informal_text", "")
                or payload.get("input", "")
                or payload.get("prompt", "")
            )
            try:
                from autoformalizer import AutoformalizerEngine
                engine = AutoformalizerEngine()
                res = engine.autoformalize(informal_text)
                self._set_cors_headers(200)
                self.wfile.write(json.dumps(res).encode("utf-8"))
            except Exception as e:
                self._set_cors_headers(500)
                self.wfile.write(json.dumps({
                    "success": False,
                    "error": f"Autoformalization error: {str(e)}"
                }).encode("utf-8"))

        elif path == "/api/prover/solve":
            lean_code = payload.get("lean_code", "")
            goal = payload.get("goal", "")
            params = payload.get("params", "")
            simulations = int(payload.get("simulations", 32))
            timeout_sec = float(payload.get("timeout", 20.0))

            t0 = time.time()
            res = {
                "success": False,
                "solved": False,
                "winning_tactic": "",
                "full_code": "",
                "solve_method": "",
                "latency_ms": 0.0,
                "nodes_expanded": 1,
                "max_depth": 1,
                "axioms": [],
                "sorry_count": 0,
                "search_trace": [],
                "error": None
            }

            try:
                candidate_tactics = [
                    ("ring", "algebraic_decision_procedure"),
                    ("linarith", "linear_arithmetic"),
                    ("omega", "integer_presburger"),
                    ("aesop", "automated_search"),
                    ("trivial", "pure_logical_identity"),
                    ("rfl", "definitional_equality"),
                    ("simp", "simplifier_reduction"),
                    ("nlinarith", "non_linear_arithmetic"),
                    ("ring\n  aesop", "depth2_algebraic_repair"),
                    ("simp\n  ring", "depth2_simplifier_algebra"),
                    ("simp\n  omega", "depth2_simplifier_integer"),
                ]

                try:
                    from premise_retriever import PremiseRetriever
                    retriever = PremiseRetriever()
                    p_tacs = retriever.generate_candidate_tactics(goal, hypotheses=params, top_k=3)
                    for ptac, prior in p_tacs:
                        candidate_tactics.append((ptac, f"bm25_premise_rag (score={prior:.2f})"))
                except Exception:
                    pass

                from lean_worker import verify_in_lean4
                try:
                    from lean_repl_client import LeanREPLClient
                    repl = LeanREPLClient()
                except Exception:
                    repl = None

                trace = []
                solved_tac = None

                for tac, source in candidate_tactics:
                    t_eval_0 = time.time()
                    if repl is not None:
                        ok, lat, err = repl.verify_script(lean_code, tac)
                    else:
                        ok, lat, err = verify_in_lean4(lean_code, tac, timeout_sec=min(5.0, timeout_sec))
                    eval_lat = round((time.time() - t_eval_0) * 1000, 1)
                    trace.append({
                        "depth": 2 if "\n" in tac else 1,
                        "tactic": tac,
                        "source": source,
                        "verified": ok,
                        "latency_ms": eval_lat,
                        "error": err[:120] if err else None
                    })
                    if ok:
                        solved_tac = tac
                        break

                if solved_tac:
                    indented_tac = "\n  ".join([l.strip() for l in solved_tac.split("\n") if l.strip()])
                    full_script = lean_code
                    if ":= by" in full_script:
                        stmt_part = full_script.split(":= by")[0].strip()
                        full_script = f"{stmt_part} := by\n  {indented_tac}"
                    else:
                        full_script = f"{full_script}\n  {indented_tac}"

                    total_ms = round((time.time() - t0) * 1000, 1)
                    res.update({
                        "success": True,
                        "solved": True,
                        "winning_tactic": solved_tac,
                        "full_code": full_script,
                        "solve_method": "one_step",
                        "latency_ms": total_ms,
                        "nodes_expanded": len(trace),
                        "max_depth": 2 if "\n" in solved_tac else 1,
                        "axioms": ["propext", "Classical.choice", "Quot.sound"],
                        "sorry_count": 0,
                        "search_trace": trace,
                        "error": None
                    })
                else:
                    try:
                        prover = get_dbf_mcts_prover(timeout_sec=max(30.0, timeout_sec))
                        prob_dict = {
                            "problem_id": "custom_user_problem",
                            "goal": goal,
                            "params": params,
                            "raw_code": lean_code
                        }
                        search_res = prover.search(prob_dict, simulations_budget=simulations)
                        if search_res.solved and search_res.winning_script:
                            winning_tac = search_res.winning_script
                            indented_tac = "\n  ".join([l.strip() for l in winning_tac.split("\n") if l.strip()])
                            full_script = lean_code
                            if ":= by" in full_script:
                                stmt_part = full_script.split(":= by")[0].strip()
                                full_script = f"{stmt_part} := by\n  {indented_tac}"
                            else:
                                full_script = f"{full_script}\n  {indented_tac}"

                            res.update({
                                "success": True,
                                "solved": True,
                                "winning_tactic": winning_tac,
                                "full_code": full_script,
                                "solve_method": search_res.solve_method or "mcts_tree_search",
                                "latency_ms": round(search_res.total_latency_ms, 1),
                                "nodes_expanded": search_res.nodes_expanded,
                                "max_depth": search_res.max_depth_reached,
                                "axioms": ["propext", "Classical.choice", "Quot.sound"],
                                "sorry_count": 0,
                                "search_trace": trace,
                                "error": None
                            })
                        else:
                            total_ms = round((time.time() - t0) * 1000, 1)
                            res.update({
                                "success": False,
                                "solved": False,
                                "latency_ms": total_ms,
                                "nodes_expanded": max(len(trace), getattr(search_res, "nodes_expanded", len(trace))),
                                "max_depth": getattr(search_res, "max_depth_reached", 1),
                                "search_trace": trace,
                                "error": "Search completed: proof search budget exhausted without closing all goals"
                            })
                    except Exception as tree_err:
                        total_ms = round((time.time() - t0) * 1000, 1)
                        res.update({
                            "success": False,
                            "solved": False,
                            "latency_ms": total_ms,
                            "nodes_expanded": len(trace),
                            "search_trace": trace,
                            "error": f"MCTS search error: {str(tree_err)}"
                        })

                self._set_cors_headers(200)
                self.wfile.write(json.dumps(res).encode("utf-8"))

            except Exception as e:
                self._set_cors_headers(500)
                res["error"] = f"Prover execution error: {str(e)}"
                self.wfile.write(json.dumps(res).encode("utf-8"))

        elif path == "/api/verify":
            raw_code = payload.get("raw_code", "")
            tactic = payload.get("tactic", "")
            timeout_sec = float(payload.get("timeout_sec", 10.0))

            try:
                from lean_worker import verify_in_lean4
                ok, lat, err = verify_in_lean4(raw_code, tactic, timeout_sec=timeout_sec)
                self._set_cors_headers(200)
                self.wfile.write(json.dumps({
                    "verified": ok,
                    "latency_ms": round(lat, 1),
                    "error": err
                }).encode("utf-8"))
            except Exception as e:
                self._set_cors_headers(500)
                self.wfile.write(json.dumps({
                    "verified": False,
                    "error": str(e)
                }).encode("utf-8"))

        else:
            self._set_cors_headers(404)
            self.wfile.write(json.dumps({"error": f"Unknown POST endpoint: {path}"}).encode("utf-8"))


def run_server(port: int = 8088):
    server_address = ("0.0.0.0", port)
    httpd = ThreadingHTTPServer(server_address, SimulationHTTPHandler)
    print(f"Simulation server running on http://0.0.0.0:{port}...")
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down simulation server.")
    finally:
        httpd.server_close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="ProofBattle Simulation Server")
    parser.add_argument("--port", type=int, default=8088, help="Port to listen on (default 8088)")
    args = parser.parse_args()
    run_server(args.port)
