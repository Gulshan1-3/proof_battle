#!/usr/bin/env python3
"""
High-Throughput Lean 4 Compiler Worker and Cache Manager (Stage 10).

Provides:
- In-memory verification hash caching for idempotent subgoals.
- Pre-warmed execution harness reducing sub-lemma checks to sub-100ms on cache hits.
- Subprocess timeout sandboxing with set_option warningAsError true.
"""

import os
import sys
import re
import time
import json
import hashlib
import tempfile
import subprocess
import signal
import psutil
from pathlib import Path
from typing import Dict, Any, Tuple, Optional, List

LEAN_DIR = Path(__file__).resolve().parent.parent.parent / "lean"


def verify_in_lean4(raw_code: str, tactic: str, timeout_sec: float = 10.0) -> Tuple[bool, float, str]:
    """
    Compiles candidate tactic sequence through Lean 4 compiler kernel.
    Process-based fallback used when Lean REPL client is offline.
    """
    for _ in range(40):
        try:
            if psutil.virtual_memory().available >= 2.0 * 1024 * 1024 * 1024:
                break
        except Exception:
            break
        time.sleep(0.2)

    code_mod = raw_code
    if "set_option maxHeartbeats" in code_mod:
        code_mod = re.sub(r"set_option\s+maxHeartbeats\s+\d+", "set_option maxHeartbeats 60000", code_mod)
    else:
        if "import Mathlib" in code_mod:
            code_mod = code_mod.replace("import Mathlib", "import Mathlib\nset_option maxHeartbeats 60000", 1)
        else:
            code_mod = "set_option maxHeartbeats 60000\n" + code_mod

    if "import Mathlib" in code_mod:
        code_mod = code_mod.replace("import Mathlib", "import Mathlib\n\nset_option warningAsError true\nset_option linter.unusedVariables false", 1)
    else:
        code_mod = "set_option warningAsError true\nset_option linter.unusedVariables false\n" + code_mod

    tactic_indented = "\n  ".join([l.strip() for l in tactic.strip().split("\n") if l.strip()])
    code_mod = code_mod.replace(":= by sorry", f":= by\n  {tactic_indented}").replace(":= by\n  sorry", f":= by\n  {tactic_indented}")
    if ":= by" not in code_mod:
        code_mod = code_mod.replace(":= sorry", f":= by\n  {tactic_indented}")

    with tempfile.NamedTemporaryFile(suffix=".lean", mode="w", encoding="utf-8", delete=False) as f:
        f.write(code_mod)
        fname = f.name

    p = None
    try:
        t0 = time.perf_counter()
        p = subprocess.Popen(
            ["lake", "env", "lean", "-j", "1", fname],
            cwd=str(LEAN_DIR),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            start_new_session=True
        )
        stdout, stderr = p.communicate(timeout=timeout_sec)
        elapsed_ms = (time.perf_counter() - t0) * 1000.0
        out_combined = (stderr or "") + "\n" + (stdout or "")
        success = (p.returncode == 0) and ("sorry" not in out_combined) and ("error:" not in out_combined)
        err = out_combined.strip() if not success else ""
        return success, elapsed_ms, err
    except subprocess.TimeoutExpired:
        if p is not None:
            try:
                os.killpg(p.pid, signal.SIGKILL)
            except (ProcessLookupError, OSError):
                pass
            try:
                p.communicate()
            except Exception:
                pass
        return False, timeout_sec * 1000.0, "Execution timed out"
    except Exception as e:
        if p is not None:
            try:
                os.killpg(p.pid, signal.SIGKILL)
            except (ProcessLookupError, OSError):
                pass
        return False, 0.0, str(e)
    finally:
        if p is not None and p.poll() is None:
            try:
                os.killpg(p.pid, signal.SIGKILL)
            except (ProcessLookupError, OSError):
                pass
            try:
                p.wait(timeout=0.5)
            except Exception:
                pass
        try:
            os.remove(fname)
        except OSError:
            pass


class LeanWorker:
    def __init__(self, lean_dir: Optional[Path] = None, max_heartbeats: int = 200000, timeout_sec: float = 12.0):
        self.lean_dir = lean_dir or LEAN_DIR
        self.max_heartbeats = max_heartbeats
        self.timeout_sec = timeout_sec
        self.cache: Dict[str, Dict[str, Any]] = {}
        self.total_checks = 0
        self.cache_hits = 0
        self.total_lean_ms = 0.0

    def _compute_hash(self, goal: str, tactic: str, imports: List[str]) -> str:
        payload = f"{','.join(sorted(imports))}|{goal.strip()}|{tactic.strip()}"
        return hashlib.sha256(payload.encode("utf-8")).hexdigest()

    def format_lean_file(self, goal: str, tactic: str, imports: List[str], thm_name: str = "hierarchical_goal") -> str:
        imports_str = "\n".join(imports)
        proof_lines = "\n  ".join(tactic.strip().split("\n"))
        return (
            f"{imports_str}\n\n"
            f"set_option warningAsError true\n"
            f"set_option maxHeartbeats {self.max_heartbeats}\n\n"
            f"theorem {thm_name} : {goal.strip()} := by\n  {proof_lines}\n"
        )

    def verify(self, goal: str, tactic: str, imports: List[str], thm_name: str = "hierarchical_goal") -> Tuple[bool, float, str, bool]:
        """
        Verifies a proof candidate against Lean 4.
        Returns: (success, latency_ms, error_message, is_cache_hit)
        """
        self.total_checks += 1
        h = self._compute_hash(goal, tactic, imports)
        if h in self.cache:
            self.cache_hits += 1
            entry = self.cache[h]
            return entry["success"], entry["latency_ms"], entry["error"], True

        code = self.format_lean_file(goal, tactic, imports, thm_name)
        with tempfile.NamedTemporaryFile(suffix=".lean", mode="w", encoding="utf-8", delete=False) as f:
            f.write(code)
            tmp_path = f.name

        p = None
        try:
            t0 = time.time()
            p = subprocess.Popen(
                ["lake", "env", "lean", tmp_path],
                cwd=str(self.lean_dir),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                start_new_session=True
            )
            stdout, stderr = p.communicate(timeout=self.timeout_sec)
            elapsed_ms = (time.time() - t0) * 1000.0
            self.total_lean_ms += elapsed_ms
            success = p.returncode == 0
            err_output = (stderr or "") + "\n" + (stdout or "")
            err = err_output.strip() if not success else ""

            self.cache[h] = {
                "success": success,
                "latency_ms": round(elapsed_ms, 2),
                "error": err
            }
            return success, elapsed_ms, err, False
        except subprocess.TimeoutExpired:
            if p is not None:
                try:
                    os.killpg(p.pid, signal.SIGKILL)
                except (ProcessLookupError, OSError):
                    pass
                try:
                    p.communicate()
                except Exception:
                    pass
            return False, self.timeout_sec * 1000.0, "Execution timed out", False
        except Exception as e:
            if p is not None:
                try:
                    os.killpg(p.pid, signal.SIGKILL)
                except (ProcessLookupError, OSError):
                    pass
            return False, 0.0, str(e), False
        finally:
            if p is not None and p.poll() is None:
                try:
                    os.killpg(p.pid, signal.SIGKILL)
                except (ProcessLookupError, OSError):
                    pass
                try:
                    p.wait(timeout=0.5)
                except Exception:
                    pass
            try:
                os.remove(tmp_path)
            except OSError:
                pass

    def get_stats(self) -> Dict[str, Any]:
        return {
            "total_checks": self.total_checks,
            "cache_hits": self.cache_hits,
            "cache_hit_rate": round((self.cache_hits / self.total_checks * 100.0), 1) if self.total_checks > 0 else 0.0,
            "avg_latency_ms": round((self.total_lean_ms / max(1, self.total_checks - self.cache_hits)), 2)
        }

def test_worker():
    print("Testing LeanWorker acceleration harness...")
    worker = LeanWorker()
    goal = "∀ (x y : ℝ), x + y = y + x"
    tactic = "intro x y\nring"
    imports = ["import Mathlib.Data.Real.Basic", "import Mathlib.Tactic.Ring"]

    # Cold check
    ok1, lat1, err1, hit1 = worker.verify(goal, tactic, imports)
    print(f"Run 1 (Cold): success={ok1}, lat={lat1:.1f}ms, cache_hit={hit1}")
    assert ok1, f"Run 1 failed: {err1}"

    # Warm cache check
    ok2, lat2, err2, hit2 = worker.verify(goal, tactic, imports)
    print(f"Run 2 (Cached): success={ok2}, lat={lat2:.1f}ms, cache_hit={hit2}")
    assert ok2 and hit2, "Run 2 cache hit assertion failed"

    stats = worker.get_stats()
    print("Worker Stats:", json.dumps(stats, indent=2))
    print("LeanWorker passed all verification checks.")

if __name__ == "__main__":
    if "--test" in sys.argv:
        test_worker()
    else:
        print("LeanWorker module loaded. Use --test to run harness.")
