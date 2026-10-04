# ProofBattle Monitoring & Alerting PromQL Queries

This document contains standard PromQL queries for dashboards (such as Grafana) and alerts evaluating metrics exposed at `GET /metrics`.

---

## 1. Verification Latency & Performance

### p95 Verification Latency (Seconds)
Evaluates the 95th percentile execution latency of the Lean 4 judge across all completed proofs over a 5-minute rolling window:
```promql
histogram_quantile(0.95, sum(rate(proofbattle_lean_duration_seconds_bucket[5m])) by (le))
```

### p99 Verification Latency (Seconds)
Evaluates the 99th percentile execution latency to identify tail compiler bottlenecks:
```promql
histogram_quantile(0.99, sum(rate(proofbattle_lean_duration_seconds_bucket[5m])) by (le))
```

### Average Queue Wait Time (Seconds)
Measures the time verification jobs spend waiting for an available worker permit:
```promql
rate(proofbattle_lean_queue_wait_seconds_sum[5m]) / rate(proofbattle_lean_queue_wait_seconds_count[5m])
```

---

## 2. Verification Outcomes & Soundness

### Verification Throughput (Total Verifications per Minute)
```promql
sum(rate(proofbattle_lean_verifications_total[1m])) * 60
```

### Acceptance Ratio (Percentage of Submissions Accepted)
```promql
sum(rate(proofbattle_lean_verifications_total{verdict="Accepted"}[5m]))
/
sum(rate(proofbattle_lean_verifications_total[5m])) * 100
```

### Rejections by Reason (Rate per Minute)
Breaks down failures into specific rejection categories (LeaningFailed, UsesSorry, TimedOut, TooLarge, RejectedByFilter, Busy):
```promql
sum(rate(proofbattle_lean_verifications_total{verdict="Rejected"}[5m])) by (reason) * 60
```

### Sandbox Rejections by Failure Type
```promql
sum(rate(proofbattle_sandbox_rejected_total[5m])) by (reason) * 60
```

---

## 3. Queue Capacity & Load Shedding

### Queue Drop Rate (Drops per Minute by Lane)
Identifies when background check tasks are dropped or when the submit queue is saturated:
```promql
sum(rate(proofbattle_queue_dropped_total[1m])) by (lane) * 60
```

### Rate Limiting Rate (Events per Minute)
Identifies when players exceed their submission or check quotas:
```promql
rate(proofbattle_rate_limited_total[5m]) * 60
```

---

## 4. Game Activity & Connections

### Active Games in Progress
```promql
proofbattle_games_in_progress
```

### Connected WebSocket Clients
```promql
proofbattle_ws_connections
```

### Authenticated Players Online
```promql
proofbattle_players_online
```

### Games Completed per Minute
```promql
rate(proofbattle_rooms_total[5m]) * 60
```

---

## 5. Recommended Alert Rules

### Critical: Sandbox Image Missing on Ready Probe
Alert fires when the readiness probe fails because sandboxing is enabled but the Docker image is absent:
```promql
probe_success{job="proofbattle_readyz"} == 0
```

### Warning: High Verification Latency
Alert fires when p95 verification exceeds 10 seconds for 5 consecutive minutes:
```promql
histogram_quantile(0.95, sum(rate(proofbattle_lean_duration_seconds_bucket[5m])) by (le)) > 10
```

### Warning: Elevated Rate Limiting
Alert fires when rate limit violations exceed 20 events per minute:
```promql
rate(proofbattle_rate_limited_total[5m]) * 60 > 20
```
