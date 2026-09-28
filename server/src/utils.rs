pub fn preprocess(code: &str) -> String {
    code.replace(">=", "≥")
        .replace("<=", "≤")
        .replace("->", "→")
        .replace("=>", "⇒")
}
