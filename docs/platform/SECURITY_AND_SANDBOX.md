# ProofBattle Anti-Cheat System Design

> **Author Perspective**: Security engineer background from chess.com anti-cheat systems, applied to formal proof verification competitive platform.
>
> **Classification**: Internal Security Document - Confidential

---

## Table of Contents

1. [Threat Model Overview](#1-threat-model-overview)
2. [Cheat Vector Taxonomy](#2-cheat-vector-taxonomy)
3. [Detection Methodologies](#3-detection-methodologies)
4. [Enforcement & Ban Policy](#4-enforcement--ban-policy)
5. [Implementation Roadmap](#5-implementation-roadmap)

---

## 1. Threat Model Overview

### 1.1 Adversary Profiles

| Profile | Motivation | Technical Capability | Resource Level |
|---------|------------|----------------------|----------------|
| **Casual Boosters** | Higher rating for prestige | Low (copy-paste from AI) | None |
| **Rating Farmers** | Sell high-rated accounts | Medium (scripts, multiple accounts) | Low |
| **Professional Cheaters** | Tournament prizes, betting | High (custom tooling, ML) | High |
| **Academic Researchers** | Test system limits | Very High (novel attacks) | Academic |

### 1.2 Attack Surface Analysis

```
┌─────────────────────────────────────────────────────────────┐
│                    PROOFBATTLE ATTACK SURFACE               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  CLIENT SIDE              NETWORK              SERVER SIDE  │
│  ┌─────────────┐         ┌──────────┐       ┌───────────┐  │
│  │ Browser     │◄───────►│ WebSocket│◄──────►│ Verification│  │
│  │ - DOM access│         │ - Timing │       │ - Lean exec │  │
│  │ - DevTools  │         │ - Packet │       │ - DB writes │  │
│  │ - Extensions│         │ - Replay │       │ - Rating   │  │
│  └─────────────┘         └──────────┘       └───────────┘  │
│       │                        │                   │        │
│       ▼                        ▼                   ▼        │
│  - AI integration         - Lag switching    - Race cond.  │
│  - Proof databases        - Packet replay    - SQL inject  │
│  - Automation scripts     - MITM proxies     - Logic bugs  │
│  - Account sharing                              │          │
│                                                     │        │
│  EXTERNAL SERVICES                                   │        │
│  ┌─────────────┐                                    │        │
│  │ LLM APIs    │───────────────────────────────────┘        │
│  │ (GPT, Claude)│                                            │
│  │ Proof search│                                            │
│  │ Mathlib DB  │                                            │
│  └─────────────┘                                            │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Cheat Vector Taxonomy

### 2.1 AI/LLM Assistance (HIGH SEVERITY)

#### Attack Description
Players use large language models (ChatGPT, Claude, GPT-4, etc.) to generate Lean proofs in real-time during matches.

#### Why It's Effective
- Modern LLMs can generate correct Lean tactics for simple to intermediate proofs
- Latency of API calls (~2-5s) fits within typical match time limits
- No visual indicators to opponent
- Difficult to distinguish from skilled human play

#### Specific Techniques

**Technique A: Real-time API Calls**
```javascript
// Browser extension or injected script
async function getProofFromAI(goal) {
  const response = await fetch('https://api.openai.com/v1/chat/completions', {
    method: 'POST',
    headers: { 'Authorization': 'Bearer sk-...' },
    body: JSON.stringify({
      model: 'gpt-4',
      messages: [{
        role: 'user',
        content: `Write a Lean 4 proof for: ${goal}. Return only the tactic body inside 'by ...'`
      }]
    })
  });
  return response.json().choices[0].message.content;
}
```

**Technique B: Local LLM Inference**
- Run LLaMA-2, Mistral, or CodeLlama locally via Ollama/lm-studio
- Zero network footprint
- Sub-second latency for small proofs

**Technique C: Pre-computed AI Cache**
- Query AI for all problems in the database offline
- Store in local lookup table indexed by problem hash
- Near-instant lookup during matches

#### Detection Methods

**Method 1: Timing Anomaly Detection**
```
Metric: Time from problem receipt to first proof submission

Baseline distribution (human):
  - Easy problems (ELO 0-600):    μ=45s, σ=25s
  - Medium problems (ELO 600-1000): μ=120s, σ=45s
  - Hard problems (ELO 1000+):    μ=300s, σ=90s

AI-assisted distribution:
  - All difficulties:              μ=8s, σ=3s

Detection: Z-score > 3.5 for submission time
```

**Method 2: Proof Style Fingerprinting**
```rust
// Analyze submitted tactic patterns
struct ProofFingerprint {
    tactic_sequence: Vec<String>,
    indentation_style: IndentationPattern,
    variable_naming: NamingConvention,
    comment_density: f64,
    use_of_automation: bool,  // e.g., 'simp', 'omega' overuse
}

// AI-generated proofs often have:
// - No comments
// - Perfect indentation
// - Unusual tactic combinations
// - Missing 'have'/'obtain' scaffolding
```

**Method 3: Network Traffic Analysis**
- Detect outbound API calls to known LLM endpoints
- Monitor for characteristic request patterns (JSON with specific headers)
- Browser extension detection via CSP violations

**Method 4: Behavioral Consistency**
- Compare proof complexity across matches
- Sudden skill jumps without corresponding practice
- Inconsistent error patterns (AI doesn't make "human" mistakes)

#### Prevention Strategies

**Technical:**
- Implement client-side integrity checks (WebAssembly-based)
- Detect browser DevTools open state
- Monitor for injected scripts via Content Security Policy
- Rate-limit proof submissions (prevent AI retry loops)

**Design:**
- Add "live coding" requirement: must show intermediate steps
- Require tactic-by-tactic submission for higher-rated matches
- Time pressure: reduce time limits for higher ELO brackets

**Policy:**
- Flag accounts with suspicious timing for manual review
- Require video verification for tournament participation
- Implement "trusted device" program for high-rated players

---

### 2.2 Proof Database / Pre-computation (MEDIUM SEVERITY)

#### Attack Description
Players maintain databases of pre-computed proofs for all problems in the system, either scraped from official solutions or computed offline.

#### Why It's Effective
- Mathlib4 has ~70,000 theorems with existing proofs
- Natural Number Game has 100+ curated problems
- Zero latency during match
- Indistinguishable from skilled play if proofs are hand-written

#### Specific Techniques

**Technique A: Mathlib Scraping**
```bash
# Extract all proofs from Mathlib4
grep -A 20 "^theorem " mathlib4/Mathlib/**/*.lean > proof_database.txt
# Index by theorem signature
```

**Technique B: Community Solution Archives**
- Monitor Lean community forums, GitHub repos
- Scrape educational platforms (Natural Number Game solutions)
- Build hash-indexed lookup table

**Technique C: Offline Computation Farm**
- Run automated provers (auto, simp, ring) on all problems
- Store successful tactic sequences
- Update as new problems are added

#### Detection Methods

**Method 1: Submission Similarity Analysis**
```sql
-- Compare submitted proofs against known solutions
SELECT similarity_score(
    submitted_proof,
    canonical_solution,
    'tactic_sequence'
) FROM submissions
WHERE similarity_score > 0.95;

-- Flag near-identical submissions
```

**Method 2: Novelty Scoring**
```rust
fn calculate_novelty(submission: &Proof, historical: &[Proof]) -> f64 {
    let tactic_entropy = calculate_tactic_entropy(&submission.tactics);
    let sequence_uniqueness = compare_sequences(&submission, historical);
    
    // Low entropy + high similarity to existing = likely copied
    1.0 - (tactic_entropy * sequence_uniqueness)
}
```

**Method 3: Access Pattern Analysis**
- Detect if player requests same problem repeatedly in practice mode
- Monitor for suspicious problem selection patterns
- Track "first-time correct" rates

#### Prevention Strategies

**Technical:**
- Rotate problem statements (equivalent but different formulation)
- Add "problem salt" - small variations that break pre-computation
- Implement proof obfuscation (different variable names, tactic order)

**Design:**
- Procedural problem generation for higher ELO brackets
- Dynamic difficulty adjustment based on player history
- Time-limited problem availability

---

### 2.3 Collusion & Rating Manipulation (HIGH SEVERITY)

#### Attack Description
Multiple players coordinate to artificially inflate ratings through intentional wins/losses.

#### Why It's Effective
- Difficult to detect if played realistically
- Can be done across multiple accounts
- High ROI for selling boosted accounts

#### Specific Techniques

**Technique A: Win Trading**
```
Account A (ELO 1200) vs Account B (ELO 1200)
A wins → A: 1224, B: 1176
B wins → A: 1200, B: 1200
Repeat → both maintain high rating while farming match volume
```

**Technique B: Rating Farming**
```
Main account (ELO 1500) vs Farm accounts (ELO 800)
Main wins consistently → Main: 1800+
Farms create new accounts → repeat cycle
```

**Technique C: Disconnect Abuse**
```
Player sees difficult problem → intentionally disconnect
Within 30-second forfeit window → no rating penalty
Repeat until favorable problem appears
```

**Technique D: Tournament Collusion**
- Multiple players agree to throw matches
- Bracket manipulation
- Prize splitting agreements

#### Detection Methods

**Method 1: Graph Analysis of Player Network**
```python
# Build social graph of match history
G = build_match_graph(all_matches)

# Detect clusters with unusual patterns
suspicious_clusters = find_communities(G, criteria={
    'high_mutual_win_rate': >0.8,
    'low_opponent_diversity': <5 unique opponents,
    'temporal_clustering': matches within short windows
})
```

**Method 2: Statistical Anomaly Detection**
```
Metrics to monitor:
- Win rate vs expected win rate (based on ELO)
- Rating trajectory (sudden jumps)
- Opponent selection (repeated matches vs same players)
- Disconnect timing (pattern of early disconnects)
- Submission timing (consistent delays suggesting coordination)

Flag if:
- Actual win rate > expected win rate + 3σ
- Rating gain > 100 ELO in 24 hours
- >50% of matches vs same 3 opponents
```

**Method 3: Device & Network Fingerprinting**
```rust
struct PlayerFingerprint {
    browser_agent: String,
    screen_resolution: (u32, u32),
    timezone: String,
    ip_address: IpAddr,
    canvas_hash: String,
    webgl_hash: String,
    audio_context_hash: String,
}

// Detect multiple accounts from same device/network
fn detect_sybil_accounts(fingerprints: &[PlayerFingerprint]) -> Vec<AccountId> {
    fingerprints
        .iter()
        .group_by(|f| (f.ip_address, f.canvas_hash))
        .filter(|(_, group)| group.len() > 1)
        .flat_map(|(_, group)| group.map(|f| f.account_id))
        .collect()
}
```

**Method 4: Gameplay Pattern Analysis**
```rust
// Analyze in-match behavior
struct GameplayMetrics {
    time_to_first_submission: Duration,
    submission_frequency: f64,  // submissions per minute
    error_recovery_time: Duration,
    tactic_diversity: f64,
    use_of_hints: u32,
}

// Colluding accounts often have:
// - Identical timing patterns
// - Synchronized submission bursts
// - Unusual hint usage (both skip or both use)
```

#### Prevention Strategies

**Technical:**
- Implement the 30-second forfeit window (ADR-010)
- Rate-limit account creation per IP/device
- Require phone/email verification for competitive play
- Implement "trusted queue" for verified players only

**Design:**
- Hidden ELO (visible rating differs from internal)
- Rating decay for inactivity
- Matchmaking restrictions (cannot play same opponent frequently)
- Tournament qualification requirements (minimum match diversity)

**Policy:**
- Lifetime bans for confirmed win-trading
- Account linking (prevent multiple accounts per person)
- Prize payout verification (identity verification for payouts)

---

### 2.4 Automation & Scripting (MEDIUM SEVERITY)

#### Attack Description
Players use scripts to automate proof submission, match joining, or other game actions.

#### Why It's Effective
- Can achieve superhuman reaction times
- Can play 24/7 for rating farming
- Difficult to distinguish from fast human play

#### Specific Techniques

**Technique A: Auto-Submit Script**
```javascript
// Browser userscript
setInterval(() => {
  const submitButton = document.querySelector('[data-testid="submit-proof"]');
  if (submitButton && !submitButton.disabled) {
    submitButton.click();
  }
}, 100);
```

**Technique B: Match Queue Automation**
```python
# Python script with Selenium
driver = webdriver.Chrome()
while True:
    join_match(driver)
    proof = get_proof_from_database(problem)
    submit_proof(driver, proof)
    wait_for_next_match()
```

**Technique C: Keyboard Macro Recording**
- Record human solving a problem once
- Replay macro for subsequent matches
- Harder to detect than full automation

#### Detection Methods

**Method 1: Input Pattern Analysis**
```rust
struct InputMetrics {
    keystroke_intervals: Vec<Duration>,
    mouse_movement_smoothness: f64,
    typing_rhythm: HashMap<char, Duration>,
    copy_paste_frequency: f64,
}

// Automated input has:
// - Superhuman consistency (σ < 10ms)
// - No natural variation
// - Instant copy-paste patterns
// - Missing correction behaviors
```

**Method 2: CAPTCHA Integration**
- Trigger CAPTCHA on suspicious patterns
- Use invisible reCAPTCHA v3 for background scoring
- Require CAPTCHA for tournament qualification

**Method 3: Browser Integrity Checks**
```javascript
// Detect automation tools
function detectAutomation() {
    return (
        window.navigator.webdriver ||           // Selenium
        window.chrome?.runtime?.id ||          // Extensions
        window.outerWidth === 0 ||             // Headless
        !window.navigator.plugins.length       // Headless
    );
}
```

**Method 4: Behavioral Biometrics**
- Measure typing cadence unique to each player
- Build baseline profile over time
- Flag deviations from personal baseline

#### Prevention Strategies

**Technical:**
- Implement rate limits on all actions
- Add random delays to server responses (prevent timing attacks)
- Use WebAssembly for client-side logic (harder to instrument)

**Design:**
- Require human interaction elements (drag-and-drop, not just clicks)
- Add "prove you're human" challenges for high-stakes matches
- Implement skill-based CAPTCHAs (solve a simple proof puzzle)

---

### 2.5 Network Manipulation (LOW-MEDIUM SEVERITY)

#### Attack Description
Players manipulate network conditions to gain unfair advantages or disrupt opponents.

#### Why It's Effective
- Can cause opponent timeouts
- Can delay unfavorable game states
- Hard to prove intent vs poor connection

#### Specific Techniques

**Technique A: Lag Switching**
- Intentionally degrade network when losing
- Cause opponent to timeout while maintaining connection

**Technique B: Packet Replay**
- Capture valid proof submission packets
- Replay at strategic times
- Bypass timing restrictions

**Technique C: MITM Proxies**
- Intercept and modify WebSocket messages
- Potentially inject false game state

#### Detection Methods

**Method 1: Network Quality Monitoring**
```rust
struct NetworkMetrics {
    latency: Duration,
    jitter: Duration,
    packet_loss: f64,
    connection_stability: f64,
}

// Flag if:
// - Latency spikes correlate with game state changes
// - Jitter exceeds threshold (>100ms)
// - Pattern of "convenient" disconnects
```

**Method 2: Server-Side State Validation**
- All game state maintained server-side
- Client messages validated against expected state
- Reject out-of-order or impossible state transitions

**Method 3: WebSocket Message Signing**
```rust
// Sign all client messages
struct SignedMessage {
    payload: ClientMessage,
    timestamp: u64,
    signature: String,  // HMAC with session key
}

// Prevent replay attacks
fn validate_message(message: &SignedMessage, session: &Session) -> bool {
    let now = unix_timestamp();
    let age = now - message.timestamp;
    
    age < 5000 && verify_hmac(&message.payload, message.signature, &session.key)
}
```

#### Prevention Strategies

**Technical:**
- Implement WebSocket over QUIC (resistant to packet manipulation)
- Use TLS with certificate pinning
- Server-authoritative game state (ADR-004)

**Design:**
- Graceful degradation for poor connections
- Disconnect forgiveness (allow brief reconnects)
- Network quality requirements for ranked play

---

### 2.6 Account Compromise & Identity Fraud (HIGH SEVERITY)

#### Attack Description
Players compromise or create fake accounts to bypass restrictions or farm ratings.

#### Why It's Effective
- Can bypass bans
- Can create unlimited farm accounts
- Can steal high-rated accounts

#### Specific Techniques

**Technique A: Credential Stuffing**
- Use leaked passwords from other breaches
- Access existing high-rated accounts

**Technique B: Synthetic Identity Creation**
- Use temporary email services
- Use VOIP numbers for SMS verification
- Use stolen identity documents

**Technique C: Account Takeover**
- Social engineering
- Session hijacking
- XSS/token theft

#### Detection Methods

**Method 1: Anomaly-Based Authentication**
```rust
struct LoginAttempt {
    ip_address: IpAddr,
    device_fingerprint: DeviceFingerprint,
    location: GeoLocation,
    time: DateTime,
    success: bool,
}

// Flag if:
// - Login from new country within 1 hour of previous login
// - Login from new device without verification
// - Multiple failed attempts from same IP
// - Impossible travel (login from NY, then London 10 min later)
```

**Method 2: Behavioral Biometrics for Authentication**
- Typing pattern on password entry
- Mouse movement during login
- Navigation patterns post-login

**Method 3: Email/Phone Verification**
- Require email verification for competitive features
- Implement SMS verification for high-rated accounts
- Use reputable email validation services

**Method 4: Session Security**
```rust
// Secure session management
struct SecureSession {
    token: String,
    device_binding: DeviceFingerprint,
    ip_binding: IpAddr,
    expires_at: DateTime,
    refresh_token: String,
}

// Invalidate on:
// - Device change
// - IP change (with grace period)
// - Suspicious activity
```

#### Prevention Strategies

**Technical:**
- Implement MFA for accounts above ELO 1200
- Use password hashing with Argon2id
- Implement rate limiting on authentication endpoints
- Use WebAuthn for passwordless authentication

**Design:**
- Graduated security (more restrictions for higher ratings)
- "Trusted device" program
- Account recovery with strict verification

**Policy:**
- Require identity verification for tournament participation
- Ban accounts using temporary email services
- Lifetime bans for account takeover attempts

---

### 2.7 Proof Injection & Exploitation (CRITICAL SEVERITY)

#### Attack Description
Players attempt to bypass the verification system through Lean compiler exploits or protocol manipulation.

#### Why It's Effective
- Could allow instant wins without valid proofs
- Could compromise server security
- Highest impact if successful

#### Specific Techniques

**Technique A: Sorry Exploitation**
```lean
-- Before ADR-003 fix
theorem goal : ∀ n : ℕ, n + 0 = n := by
  sorry  -- Exits with code 0, wins match
```

**Technique B: Compiler Option Bypass**
```lean
-- Attempt to disable warningAsError
set_option warningAsError false in
  theorem goal : ... := by sorry
```

**Technique C: Macro-Based Injection**
```lean
-- Use Lean macros to hide malicious code
macro "steal_win" : tactic => `(tactic| sorry)
theorem goal : ... := by steal_win
```

**Technique D: Protocol Manipulation**
```javascript
// Send fake ServerMessage claiming win
socket.send(JSON.stringify({
  type: 'GameEnded',
  winner: my_player_id
}));
```

#### Detection Methods

**Method 1: Multi-Layer Verification (ADR-003)**
```rust
// Already implemented per ADR-003
fn verify_proof(code: &str) -> Verdict {
    // 1. Static analysis (block forbidden patterns)
    // 2. Inject warningAsError
    // 3. Scan stderr for 'sorry'
    // 4. Check exit code
    // 5. Enforce timeout
    // 6. Verify heartbeat limit
}
```

**Method 2: Protocol Validation**
```rust
// All server messages signed
// Client messages validated against state machine
// Reject any message that would create invalid state
```

**Method 3: Lean Sandbox Hardening**
```dockerfile
# Docker seccomp profile
{
  "defaultAction": "SCMP_ACT_ERRNO",
  "syscalls": [
    { "names": ["read", "write", "exit"], "action": "SCMP_ACT_ALLOW" },
    { "names": ["execve", "fork"], "action": "SCMP_ACT_ERRNO" }
  ]
}
```

**Method 4: Audit Logging**
```rust
// Log all verification attempts
struct VerificationAudit {
    player_id: PlayerId,
    code_hash: String,
    verdict: Verdict,
    lean_output: String,
    execution_time: Duration,
    timestamp: DateTime,
}

// Monitor for:
// - Repeated exploit attempts
// - Unusual Lean output patterns
// - Suspicious code structures
```

#### Prevention Strategies

**Technical:**
- Strict sandboxing (Docker + seccomp + network isolation)
- Input sanitization (ADR-002)
- Fail-closed verification (ADR-003)
- Regular Lean security updates

**Design:**
- Defense in depth (multiple verification layers)
- Assume breach (monitor for successful exploits)
- Rapid response team for zero-days

**Policy:**
- Lifetime bans for any exploit attempt
- Bug bounty program for responsible disclosure
- Security review of all Lean code changes

---

## 3. Detection Methodologies

### 3.1 Real-Time Detection (During Match)

**Purpose**: Catch cheaters in the act, prevent immediate damage

**Triggers:**
```rust
enum RealTimeTrigger {
    SubmissionTimeTooShort(Duration),           // < 5s for medium problem
    ImpossibleProofComplexity(u32),             // Too few tactics for difficulty
    NetworkAnomaly(NetworkMetrics),             // Lag switching detected
    AutomationSignature(InputMetrics),          // Bot-like input
    ProtocolViolation(Message),                 // Invalid message sequence
}

// Response actions
enum RealTimeResponse {
    MonitorOnly,                                 // Log for later analysis
    FlagForReview,                               // Queue for manual review
    ImmediateDisqualification,                   // End match, award loss
    TemporaryBan(Duration),                      // Short-term ban
    EmergencyShutdown,                           // Stop match, investigate
}
```

**Implementation:**
```rust
pub struct RealTimeDetector {
    player_metrics: Arc<DashMap<PlayerId, PlayerMetrics>>,
    rule_engine: RuleEngine,
    alert_queue: mpsc::Sender<SecurityAlert>,
}

impl RealTimeDetector {
    pub async fn on_proof_submission(&self, player_id: PlayerId, proof: &Proof) {
        let metrics = self.player_metrics.get(&player_id).unwrap();
        
        if let Some(trigger) = self.evaluate_submission(proof, &metrics) {
            let response = self.rule_engine.decide(&trigger);
            self.alert_queue.send(SecurityAlert {
                player_id,
                trigger,
                response,
                timestamp: now(),
            }).await;
        }
    }
}
```

### 3.2 Batch Analysis (Post-Match)

**Purpose**: Detect patterns across multiple matches, catch sophisticated cheaters

**Daily Jobs:**
```rust
pub struct BatchAnalyzer {
    db: PgPool,
    ml_model: Option<CheaterDetectionModel>,
}

impl BatchAnalyzer {
    pub async fn daily_analysis(&self) -> AnalysisReport {
        let report = AnalysisReport::new();
        
        // 1. Rating anomaly detection
        report.extend(self.detect_rating_anomalies().await);
        
        // 2. Collusion detection
        report.extend(self.detect_collusion().await);
        
        // 3. AI assistance detection
        report.extend(self.detect_ai_assistance().await);
        
        // 4. Sybil account detection
        report.extend(self.detect_sybil_accounts().await);
        
        report
    }
}
```

**ML Model Features:**
```python
features = {
    # Temporal features
    'submission_time_mean': submission_times.mean(),
    'submission_time_std': submission_times.std(),
    'submission_time_skew': submission_times.skew(),
    
    # Proof features
    'tactic_diversity': len(set(all_tactics)) / len(all_tactics),
    'proof_length_mean': proof_lengths.mean(),
    'comment_density': comment_count / total_lines,
    
    # Behavioral features
    'win_rate_vs_expected': actual_win_rate - expected_win_rate,
    'opponent_diversity': len(set(opponents)) / total_matches,
    'disconnect_timing_pattern': disconnect_pattern_score,
    
    # Network features
    'latency_mean': latencies.mean(),
    'latency_jitter': latencies.std(),
    'ip_changes': ip_change_count,
    
    # Device features
    'device_consistency': device_fingerprint_consistency,
    'browser_changes': browser_change_count,
}
```

### 3.3 Manual Review Queue

**Purpose**: Human judgment for edge cases, high-stakes decisions

**Queue Prioritization:**
```rust
pub struct ReviewQueue {
    items: BinaryHeap<ReviewItem>,
}

#[derive(PartialEq, Eq)]
pub struct ReviewItem {
    priority: Priority,
    player_id: PlayerId,
    evidence: SecurityEvidence,
    auto_action: Option<EnforcementAction>,
}

impl Ord for ReviewItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // Prioritize:
        // 1. High-rated players (tournament impact)
        // 2. Multiple trigger types (stronger evidence)
        // 3. Recent activity (prevent ongoing damage)
        // 4. Severity of suspected cheat
        self.priority.cmp(&other.priority)
    }
}
```

**Review Interface:**
```rust
struct ReviewDashboard {
    // Player profile
    player_info: PlayerProfile,
    
    // Evidence timeline
    evidence_timeline: Vec<EvidenceEvent>,
    
    // Statistical comparison
    comparison: PlayerComparison,  // vs similar-rated players
    
    // Raw data access
    match_history: Vec<Match>,
    submissions: Vec<Submission>,
    network_logs: Vec<NetworkEvent>,
    
    // Recommended action
    recommendation: EnforcementAction,
    confidence: f64,
}
```

### 3.4 False Positive Mitigation

**Challenge**: Balance security with player experience

**Strategies:**

1. **Graduated Response**
```rust
enum EnforcementAction {
    Warning,                    // First offense, low confidence
    TemporaryBan(Duration),     // 1-7 days, medium confidence
    RatingReset,                // Reset to baseline, high confidence
    PermanentBan,               // Very high confidence + manual review
    TournamentDisqualification, // Tournament-specific
}
```

2. **Appeal Process**
```rust
pub struct AppealSystem {
    appeal_queue: mpsc::Sender<Appeal>,
    review_board: Vec<ReviewerId>,
}

pub struct Appeal {
    player_id: PlayerId,
    original_action: EnforcementAction,
    evidence: SecurityEvidence,
    player_statement: String,
    additional_evidence: Option<String>,
}
```

3. **Confidence Thresholds**
```rust
const AUTO_BAN_CONFIDENCE: f64 = 0.95;
const AUTO_WARNING_CONFIDENCE: f64 = 0.70;
const MANUAL_REVIEW_THRESHOLD: f64 = 0.50;
```

---

## 4. Enforcement & Ban Policy

### 4.1 Ban Tiers

| Tier | Duration | Conditions | Appealable |
|------|----------|------------|------------|
| **Warning** | Permanent | First offense, low confidence (<70%) | Yes |
| **Temp Ban** | 1-7 days | Repeat offense, medium confidence (70-90%) | Yes |
| **Temp Ban** | 30-90 days | Severe offense, high confidence (>90%) | Yes |
| **Rating Reset** | Permanent | Confirmed rating manipulation | Yes (within 30 days) |
| **Perm Ban** | Permanent | Exploit attempt, account compromise, collusion | No |
| **Hardware Ban** | Permanent | Professional cheating, tournament fraud | No |

### 4.2 Specific Offense Penalties

**AI Assistance:**
- 1st offense: Warning + rating adjustment (-50 ELO)
- 2nd offense: 7-day ban + rating reset to 1000
- 3rd offense: 30-day ban
- 4th offense: Permanent ban

**Collusion/Win Trading:**
- All involved accounts: Permanent ban
- Rating adjustments for affected opponents
- Tournament prize clawback (if applicable)

**Account Sharing:**
- All accounts: 30-day ban
- Required identity verification to reactivate
- Repeat offense: Permanent ban

**Exploit Attempts:**
- Immediate permanent ban
- IP/hardware fingerprint ban
- Report to Lean security team (if compiler exploit)

### 4.3 Rating Restoration

**For False Positives:**
```rust
pub fn restore_rating(player_id: PlayerId, original_elo: u32) {
    // Restore to original rating
    // Remove ban flags
    // Send apology and compensation (e.g., free tournament entry)
    // Log for prevention of future false positives
}
```

**For Legitimate Players Affected by Cheaters:**
```rust
pub fn compensate_victims(cheater_id: PlayerId, affected_opponents: Vec<PlayerId>) {
    // Calculate ELO stolen
    // Restore to opponents
    // Mark cheater's matches as invalid for leaderboard
}
```

### 4.4 Transparency & Communication

**Public Ban Statistics:**
```rust
// Publish anonymized ban statistics
pub struct BanStatistics {
    period: DateRange,
    total_bans: u32,
    by_reason: HashMap<CheatType, u32>,
    by_rating_bracket: HashMap<EloBracket, u32>,
    false_positive_rate: f64,
    average_appeal_time: Duration,
}
```

**Player Notifications:**
```
Dear [Player],

Your account has been temporarily suspended for 7 days due to suspicious activity detected by our anti-cheat system.

Reason: Unusual proof submission patterns consistent with AI assistance
Detection: Submission times 3.5σ below your historical baseline
Evidence: [Link to detailed evidence]
Confidence: 82%

If you believe this is an error, you may appeal within 30 days:
[Appeal Form Link]

Regards,
ProofBattle Security Team
```

---

## 5. Implementation Roadmap

### Phase 1: Foundation (Weeks 1-4)

**Priority: CRITICAL**

- [ ] Implement ADR-003 (multi-signal verification)
- [ ] Add basic timing anomaly detection
- [ ] Set up audit logging for all submissions
- [ ] Implement device fingerprinting
- [ ] Add rate limiting to all endpoints
- [ ] Set up security alert dashboard

**Acceptance Criteria:**
- All proof submissions logged with full metadata
- Timing anomalies detected in real-time
- Device fingerprints collected on registration
- Rate limits prevent basic automation

### Phase 2: Detection Systems (Weeks 5-8)

**Priority: HIGH**

- [ ] Implement batch analysis pipeline
- [ ] Build ML model for cheat detection
- [ ] Add collusion detection (graph analysis)
- [ ] Implement network quality monitoring
- [ ] Set up manual review queue
- [ ] Add CAPTCHA integration

**Acceptance Criteria:**
- Daily batch analysis runs successfully
- ML model achieves >85% precision on test set
- Collusion rings detected with <5% false positives
- Manual review queue prioritizes correctly

### Phase 3: Enforcement (Weeks 9-12)

**Priority: HIGH**

- [ ] Implement graduated ban system
- [ ] Add appeal process
- [ ] Set up ban notification system
- [ ] Implement rating restoration
- [ ] Add hardware ban capability
- [ ] Publish transparency reports

**Acceptance Criteria:**
- Ban system enforces all tiers correctly
- Appeals processed within 48 hours
- Rating restoration works for false positives
- Transparency reports generated monthly

### Phase 4: Advanced Detection (Weeks 13-16)

**Priority: MEDIUM**

- [ ] Implement behavioral biometrics
- [ ] Add proof style fingerprinting
- [ ] Implement network traffic analysis for LLM detection
- [ ] Add browser integrity checks
- [ ] Implement Sybil account detection
- [ ] Add tournament-specific monitoring

**Acceptance Criteria:**
- Behavioral biometrics achieves >90% accuracy
- Proof fingerprinting detects >70% of copied proofs
- LLM API calls detected with <1% false positives
- Sybil rings detected before significant rating gain

### Phase 5: Hardening (Weeks 17-20)

**Priority: MEDIUM**

- [ ] Implement Lean sandbox hardening
- [ ] Add protocol message signing
- [ ] Implement session security
- [ ] Add MFA for high-rated accounts
- [ ] Implement WebAuthn support
- [ ] Set up bug bounty program

**Acceptance Criteria:**
- Lean sandbox passes security audit
- All protocol messages signed and validated
- MFA required for ELO > 1200
- Bug bounty program published and receiving reports

### Phase 6: Continuous Improvement (Ongoing)

**Priority: LOW**

- [ ] Regular ML model retraining
- [ ] Monthly security reviews
- [ ] Cheat technique research
- [ ] Red team exercises
- [ ] Community trust building
- [ ] Incident response drills

**Ongoing Metrics:**
- False positive rate < 1%
- False negative rate < 5%
- Average detection time < 24 hours
- Player trust score > 80%

---

## Appendix A: Detection Rule Examples

### Rule: AI Assistance Detection

```rust
pub fn detect_ai_assistance(submission: &Submission, history: &PlayerHistory) -> Option<Trigger> {
    // Calculate Z-score for submission time
    let baseline_mean = history.submission_times.mean();
    let baseline_std = history.submission_times.std();
    let z_score = (submission.time.as_secs() as f64 - baseline_mean) / baseline_std;
    
    if z_score > 3.5 {
        return Some(Trigger::SubmissionTimeAnomaly {
            z_score,
            submission_time: submission.time,
            baseline_mean,
        });
    }
    
    // Check proof style
    let fingerprint = analyze_proof_style(&submission.proof);
    if fingerprint.ai_confidence > 0.8 {
        return Some(Trigger::ProofStyleAnomaly {
            confidence: fingerprint.ai_confidence,
            reasons: fingerprint.ai_indicators,
        });
    }
    
    None
}
```

### Rule: Collusion Detection

```rust
pub fn detect_collusion(player_a: PlayerId, player_b: PlayerId, match_history: &[Match]) -> Option<Trigger> {
    let matches_together = match_history.iter()
        .filter(|m| m.involves(player_a, player_b))
        .collect::<Vec<_>>();
    
    if matches_together.len() < 5 {
        return None; // Not enough data
    }
    
    let win_rate_a = matches_together.iter()
        .filter(|m| m.winner == player_a)
        .count() as f64 / matches_together.len() as f64;
    
    let expected_win_rate = calculate_expected_win_rate(
        get_rating(player_a),
        get_rating(player_b)
    );
    
    let deviation = (win_rate_a - expected_win_rate).abs();
    
    if deviation > 0.3 && matches_together.len() > 10 {
        return Some(Trigger::Collusion {
            players: (player_a, player_b),
            deviation,
            match_count: matches_together.len(),
        });
    }
    
    None
}
```

---

## Appendix B: Incident Response Playbook

### Severity Levels

**SEV-0 (Critical):**
- Active exploit causing widespread rating manipulation
- Server compromise via Lean sandbox
- Database breach

**Response:**
1. Emergency shutdown of affected systems
2. Incident commander activated
3. Security team assembled
4. Public communication within 1 hour
5. Full post-mortem within 7 days

**SEV-1 (High):**
- New cheat technique bypassing detection
- Tournament fraud
- Mass account compromise

**Response:**
1. Deploy temporary mitigations within 4 hours
2. Permanent fix within 48 hours
3. Affected players compensated
4. Public communication within 24 hours

**SEV-2 (Medium):**
- Individual sophisticated cheater
- Detection system false positive spike
- Minor exploit attempt

**Response:**
1. Investigate within 24 hours
2. Fix within 1 week
3. Update detection rules
4. Internal documentation

**SEV-3 (Low):**
- Routine cheat detection
- Minor false positives
- Policy clarification

**Response:**
1. Handle via standard procedures
2. Update documentation
3. Monitor for patterns

---

## Appendix C: Metrics & KPIs

### Security KPIs

| Metric | Target | Measurement |
|--------|--------|-------------|
| False Positive Rate | < 1% | Appeals upheld / total bans |
| False Negative Rate | < 5% | Post-hoc analysis of banned players |
| Average Detection Time | < 24h | Time from first cheat to detection |
| Player Trust Score | > 80% | Quarterly player survey |
| Tournament Integrity | 100% | Confirmed cheaters in tournaments |

### Operational KPIs

| Metric | Target | Measurement |
|--------|--------|-------------|
| Appeal Response Time | < 48h | Time from appeal to resolution |
| Review Queue Backlog | < 100 items | Items awaiting review |
| Detection System Uptime | > 99.9% | System availability |
| ML Model Refresh | Monthly | Last model training date |
| Security Review Cadence | Monthly | Last security review date |

---

## Appendix D: Chess.com-Style Operational Diagnostics

### Real-Time Monitoring Dashboard

**Grafana Dashboard - Anti-Cheat Health**
```
┌─────────────────────────────────────────────────────────┐
│ Detection Pipeline Status                                │
│ ├─ Real-time detector: [OK] 99.8% uptime (24h)          │
│ ├─ Batch analyzer: [OK] Last run: 2h ago (0 errors)     │
│ ├─ ML model inference: [OK] 45ms avg latency             │
│ ├─ Review queue: [WARN] 23 items (SLA: <100)            │
│ └─ Alert system: [OK] No active incidents                │
├─────────────────────────────────────────────────────────┤
│ Detection Metrics (Last 24h)                            │
│ ├─ Submissions analyzed: 12,847                         │
│ ├─ Flags raised: 342 (2.66%)                            │
│ ├─ Auto-bans: 12 (3.5% of flags)                        │
│ ├─ Manual reviews: 330                                  │
│ └─ False positive rate: 0.8% (target: <1%)             │
├─────────────────────────────────────────────────────────┤
│ System Health                                            │
│ ├─ Database latency: 8ms (p95)                         │
│ ├─ Redis connection: [OK] 99.9%                          │
│ ├─ Lean worker pool: 8/12 active                        │
│ └─ Audit log throughput: 1.2k events/sec                 │
└─────────────────────────────────────────────────────────┘
```

### Alerting Escalation (PagerDuty Integration)

**P1 - Critical (Immediate):**
- Detection pipeline down >5 min
- ML model inference failure rate >5%
- Database connection loss
- Lean sandbox compromise suspected

**P2 - High (1 hour):**
- False positive rate >2%
- Review queue backlog >200
- Batch analysis job failure
- Unusual spike in ban rate

**P3 - Medium (4 hours):**
- Performance degradation (latency >2x baseline)
- ML model drift detected
- CAPTCHA failure rate increase

### Daily Health Report (Slack #security-ops)

```
Anti-Cheat Daily Summary - 2026-09-27

Detection Pipeline: [OK] Operational
- Real-time: 99.8% uptime, 12,847 submissions processed
- Batch: Completed successfully, 3 suspicious accounts flagged
- ML Model: Precision 87.2%, Recall 92.1% (within targets)

Review Queue: [OK] Healthy
- Backlog: 23 items (avg resolution: 6.2h)
- Appeals: 4 received, 2 approved (false positive), 2 denied

Incidents: None
[WARN] Watchlist: 2 accounts showing elevated AI-assistance signals

Team on-call: @security-lead
Next ML retraining: 2026-10-15
```

### On-Call Rotation & Runbooks

**Primary:** Security engineer (24/7 pager)
**Secondary:** Backend lead (backup)
**Escalation:** CTO (P0 incidents only)

**Runbooks:**
- `detection-pipeline-down.md` - Restart procedures, failover to backup
- `high-false-positive-rate.md` - Roll back ML model, pause auto-bans
- `lean-sandbox-compromise.md` - Emergency shutdown, forensic capture
- `review-queue-backlog.md** - Triage protocol, temporary staff augmentation

### Weekly Security Sync (Monday 10am)

**Agenda:**
1. Previous week metrics review (5 min)
2. New cheat technique intelligence (10 min)
3. ML model performance & retraining schedule (10 min)
4. High-profile manual review outcomes (10 min)
5. Incident post-mortems (if any) (15 min)
6. Action items & owner assignment (10 min)

**Attendees:** Security team, Backend lead, Data science, Product

### Continuous Diagnostics

**Automated Health Checks (every 5 min):**
```rust
async fn health_check() -> SystemHealth {
    SystemHealth {
        detector_uptime: check_detector_uptime().await,
        ml_model_latency: measure_inference_latency().await,
        db_connection: test_db_query().await,
        worker_pool_status: check_lean_workers().await,
        queue_depth: get_review_queue_depth().await,
    }
}
```

**Monthly Operational Review:**
- Full false positive/negative analysis
- ML model feature importance review
- New cheat technique research
- Competitor intelligence (chess.com, etc.)
- System capacity planning

---

## Conclusion

This anti-cheat system is designed to be **defense-in-depth**, **data-driven**, and **player-centric**. The key principles are:

1. **Prevent over detect**: Make cheating difficult through design, not just detection
2. **Graduated response**: Balance security with player experience
3. **Transparency**: Build trust through open communication
4. **Continuous improvement**: Adapt to new cheat techniques
5. **Human judgment**: Use automation for detection, humans for enforcement

The implementation roadmap prioritizes critical security holes (proof verification) first, then builds sophisticated detection systems, and finally adds advanced features. This phased approach ensures security improves incrementally without blocking development.

**Success metric**: Players trust that their opponents are playing fairly, and cheaters are caught before they can damage the competitive integrity of the platform.
