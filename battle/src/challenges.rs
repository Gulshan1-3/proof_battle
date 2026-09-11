#[derive(Clone, Debug)]
pub struct ProofChallenge {
    pub goal: String,
    pub imports: Vec<String>,
}

pub fn get_problems() -> Vec<ProofChallenge> {
    vec![
        ProofChallenge {
            goal: "∀ n : ℕ, n + 0 = n".to_string(),
            imports: vec!["import Mathlib.Data.Nat.Basic".to_string()],
        },
        ProofChallenge {
            goal: "∀ a b : ℕ, a + b = b + a".to_string(),
            imports: vec!["import Mathlib.Data.Nat.Basic".to_string()],
        },
        ProofChallenge {
            goal: "∀ P Q : Prop, P ∧ Q → Q ∧ P".to_string(),
            imports: vec!["import Mathlib.Logic.Basic".to_string()],
        },
    ]
}
