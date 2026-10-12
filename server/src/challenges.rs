use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofChallenge {
    pub id: String,
    pub slug: String,
    pub goal: String,
    pub imports: Vec<String>,
    #[serde(default)]
    pub statement: String,
    #[serde(default)]
    pub canonical_proof: String,
    #[serde(default = "default_difficulty")]
    pub difficulty: u8,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub tactic_hint: Option<String>,
}

fn default_difficulty() -> u8 {
    1
}

fn default_category() -> String {
    "logic".to_string()
}

impl Default for ProofChallenge {
    fn default() -> Self {
        Self {
            id: "nat/add_zero".to_string(),
            slug: "nat/add_zero".to_string(),
            goal: "∀ n : ℕ, n + 0 = n".to_string(),
            imports: vec!["import Mathlib.Data.Nat.Basic".to_string()],
            statement: "theorem goal : ∀ n : ℕ, n + 0 = n := by".to_string(),
            canonical_proof: "intro n\nsimp".to_string(),
            difficulty: 1,
            category: "nat_arithmetic".to_string(),
            tactic_hint: Some("try simp".to_string()),
        }
    }
}

impl ProofChallenge {
    pub fn new(goal: impl Into<String>, imports: Vec<String>) -> Self {
        let goal_str = goal.into();
        Self {
            id: "custom".to_string(),
            slug: "custom".to_string(),
            goal: goal_str,
            imports,
            statement: String::new(),
            canonical_proof: String::new(),
            difficulty: 1,
            category: "custom".to_string(),
            tactic_hint: None,
        }
    }
}

#[derive(Deserialize)]
struct RawProblemRecord {
    #[serde(default)]
    id: Option<String>,
    slug: String,
    goal: String,
    imports: Vec<String>,
    #[serde(default)]
    statement: Option<String>,
    #[serde(default)]
    canonical_proof: Option<String>,
    #[serde(default)]
    difficulty: Option<u8>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    tactic_hint: Option<String>,
}

pub fn get_problems() -> Vec<ProofChallenge> {
    if let Some(problems) = load_from_curated_file() {
        if !problems.is_empty() {
            return problems;
        }
    }
    fallback_problems()
}

fn find_curated_file() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("PROBLEMS_JSONL_PATH") {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return Some(p);
        }
    }

    let candidates = [
        "problems_curated.jsonl",
        "../problems_curated.jsonl",
        "../../problems_curated.jsonl",
    ];

    for candidate in candidates {
        let p = Path::new(candidate);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }
    None
}

fn load_from_curated_file() -> Option<Vec<ProofChallenge>> {
    let path = find_curated_file()?;
    let file = File::open(&path).ok()?;
    let reader = BufReader::new(file);
    let mut result = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(raw) = serde_json::from_str::<RawProblemRecord>(trimmed) {
            let id = raw.id.unwrap_or_else(|| raw.slug.clone());
            result.push(ProofChallenge {
                id,
                slug: raw.slug,
                goal: raw.goal,
                imports: raw.imports,
                statement: raw.statement.unwrap_or_default(),
                canonical_proof: raw.canonical_proof.unwrap_or_default(),
                difficulty: raw.difficulty.unwrap_or(1),
                category: raw.category.unwrap_or_else(default_category),
                tactic_hint: raw.tactic_hint,
            });
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

pub fn fallback_problems() -> Vec<ProofChallenge> {
    vec![
        ProofChallenge {
            id: "nat/add_zero".to_string(),
            slug: "nat/add_zero".to_string(),
            goal: "∀ n : ℕ, n + 0 = n".to_string(),
            imports: vec!["import Mathlib.Data.Nat.Basic".to_string()],
            statement: "theorem goal : ∀ n : ℕ, n + 0 = n := by".to_string(),
            canonical_proof: "intro n\nsimp".to_string(),
            difficulty: 1,
            category: "nat_arithmetic".to_string(),
            tactic_hint: Some("try simp".to_string()),
        },
        ProofChallenge {
            id: "nat/add_comm".to_string(),
            slug: "nat/add_comm".to_string(),
            goal: "∀ a b : ℕ, a + b = b + a".to_string(),
            imports: vec!["import Mathlib.Data.Nat.Basic".to_string()],
            statement: "theorem goal : ∀ a b : ℕ, a + b = b + a := by".to_string(),
            canonical_proof: "intro a b\nomega".to_string(),
            difficulty: 2,
            category: "nat_arithmetic".to_string(),
            tactic_hint: Some("try omega".to_string()),
        },
        ProofChallenge {
            id: "logic/and_comm".to_string(),
            slug: "logic/and_comm".to_string(),
            goal: "∀ P Q : Prop, P ∧ Q → Q ∧ P".to_string(),
            imports: vec!["import Mathlib.Logic.Basic".to_string()],
            statement: "theorem goal : ∀ P Q : Prop, P ∧ Q → Q ∧ P := by".to_string(),
            canonical_proof: "intro P Q ⟨hp, hq⟩\nexact ⟨hq, hp⟩".to_string(),
            difficulty: 3,
            category: "logic".to_string(),
            tactic_hint: Some("intro and construct".to_string()),
        },
    ]
}
