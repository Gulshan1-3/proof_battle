#!/usr/bin/env python3
"""
Stage 65: Learned Adaptive Monte Carlo Proof Search (LAMP) Controller.

Implements the Learned Allocation Controller for Persistent Multi-Branch Forests:
1. Feature Extraction (z_k in R^8):
   - z_1: Normalized search depth reached (d / d_max)
   - z_2: Visit share (N_k / sum N)
   - z_3: Empirical Q-value / proof progress reward
   - z_4: Context growth (number of introduced hypotheses / step count)
   - z_5: Subgoal reduction metric (1 / (1 + remaining_subgoals))
   - z_6: Cost expenditure ratio (cost_k / budget_max)
   - z_7: Root policy prior P(a_k | s_0)
   - z_8: Terminal / diagnostic penalty (0 for active, 1 for exhausted/failed)
2. Marginal Utility Model:
   Computes utility U_k balancing progress indicators against cost and failure penalties.
3. Exploration-Exploitation Scheduling:
   Combines marginal utility U_k with PUCT-style exploration bonus:
   A_k = U_k + c_lamp * sqrt(ln(sum N + 1) / (N_k + 1))
4. Dynamic Allocation:
   Selects the highest-utility branch k* and allocates the next compute increment delta.
"""

import math
import numpy as np
from dataclasses import dataclass
from typing import Dict, Any, List, Optional, Tuple


@dataclass
class BranchFeatureVector:
    """8-dimensional state representation of a search branch."""
    depth_norm: float        # d / d_max
    visit_share: float       # N_k / sum N
    q_value: float           # Empirical value
    context_growth: float    # Trajectory length / context expansion
    subgoal_metric: float    # 1 / (1 + remaining_goals)
    cost_ratio: float        # consumed_time / budget_max
    root_prior: float        # P(a_k | s_0)
    diagnostic_penalty: float  # 1.0 if exhausted/failed, 0.0 otherwise

    def to_numpy(self) -> np.ndarray:
        return np.array([
            self.depth_norm,
            self.visit_share,
            self.q_value,
            self.context_growth,
            self.subgoal_metric,
            self.cost_ratio,
            self.root_prior,
            self.diagnostic_penalty
        ], dtype=np.float32)


class BranchFeatureExtractor:
    """Extracts normalized feature vectors from branch states."""

    @staticmethod
    def extract(
        branch: Any,
        total_visits: int,
        global_budget_ms: float,
        max_depth: int = 8
    ) -> BranchFeatureVector:
        d_norm = min(1.0, branch.max_depth_reached / max(1, max_depth))
        v_share = branch.visits / max(1, total_visits)
        q_val = branch.q_value
        c_growth = min(1.0, len(branch.action_trajectory) / 10.0)
        n_goals = len(branch.remaining_subgoals) if branch.remaining_subgoals else 1
        sub_metric = 1.0 / (1.0 + n_goals)
        cost_rat = min(1.0, branch.total_cost_ms / max(1.0, global_budget_ms))
        prior = branch.prior
        diag_pen = 1.0 if branch.is_exhausted else 0.0

        return BranchFeatureVector(
            depth_norm=d_norm,
            visit_share=v_share,
            q_value=q_val,
            context_growth=c_growth,
            subgoal_metric=sub_metric,
            cost_ratio=cost_rat,
            root_prior=prior,
            diagnostic_penalty=diag_pen
        )


class LearnedAllocationController:
    """
    Learned Allocation Controller (LAMP).
    Predicts marginal utility of allocating compute chunks to branches and schedules effort.
    """

    def __init__(
        self,
        c_lamp: float = 1.25,
        weights: Optional[List[float]] = None,
        temperature: float = 0.5
    ):
        self.c_lamp = c_lamp
        self.temperature = temperature
        # Weights for [depth, visit_share, q_value, context_growth, subgoal_metric, cost_ratio, root_prior, diag_penalty]
        if weights is None:
            self.weights = np.array([
                0.20,   # depth progression
                -0.10,  # mild penalty on over-visited branches
                0.40,   # strong preference for high Q-value
                0.25,   # reward context enrichment (introduced lemmas)
                0.35,   # reward reducing subgoals
                -0.15,  # penalty on expensive branches
                0.20,   # respect initial policy prior
                -1.00   # heavy penalty on exhausted/dead branches
            ], dtype=np.float32)
        else:
            self.weights = np.array(weights, dtype=np.float32)

    def compute_marginal_utility(self, features: BranchFeatureVector) -> float:
        """Computes scalar marginal utility U_k = w^T z_k."""
        z = features.to_numpy()
        return float(np.dot(self.weights, z))

    def select_next_branch(
        self,
        branches: List[Any],
        global_budget_ms: float,
        step_chunk_delta: int = 4
    ) -> Tuple[int, int]:
        """
        Selects the most promising branch k* and the recommended simulation chunk delta.
        Returns: (branch_index, allocated_simulations)
        """
        active_branches = [b for b in branches if not b.is_exhausted and not b.is_solved]
        if not active_branches:
            return 0, 0

        total_visits = sum(b.visits for b in branches)
        scores = []

        for b in branches:
            if b.is_exhausted or b.is_solved:
                scores.append(-1e9)
                continue

            feats = BranchFeatureExtractor.extract(b, total_visits, global_budget_ms)
            u_k = self.compute_marginal_utility(feats)

            # Exploration bonus (PUCT-style)
            exploration = self.c_lamp * math.sqrt(math.log(total_visits + 1) / (b.visits + 1))
            a_k = u_k + exploration
            scores.append(a_k)

        # Select highest scoring branch
        best_idx = int(np.argmax(scores))

        # Dynamic simulation allocation chunk:
        # High-scoring branches receive larger chunk delta; lower receive minimum chunk
        best_score = scores[best_idx]
        mean_active_score = float(np.mean([s for s in scores if s > -1e8]))
        if best_score > mean_active_score + 0.2:
            allocated_delta = max(2, int(step_chunk_delta * 1.5))
        elif best_score < mean_active_score - 0.2:
            allocated_delta = max(2, int(step_chunk_delta * 0.75))
        else:
            allocated_delta = step_chunk_delta

        return best_idx, allocated_delta
