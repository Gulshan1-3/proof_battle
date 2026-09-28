use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_lines: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 8192,
            max_lines: 120,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterError {
    TooLarge {
        bytes: usize,
        lines: usize,
        max_bytes: usize,
        max_lines: usize,
    },
    InvalidControlChar {
        byte: u8,
        line: usize,
        col: usize,
    },
    UnterminatedComment {
        line: usize,
        col: usize,
    },
    UnterminatedString {
        line: usize,
        col: usize,
    },
    ForbiddenConstruct {
        token: String,
        line: usize,
        col: usize,
    },
    ForbiddenToken {
        token: String,
        line: usize,
        col: usize,
    },
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterError::TooLarge {
                bytes,
                lines,
                max_bytes,
                max_lines,
            } => {
                write!(
                    f,
                    "submission exceeds limits: {bytes} bytes (max {max_bytes}), {lines} lines (max {max_lines})"
                )
            }
            FilterError::InvalidControlChar { byte, line, col } => {
                write!(
                    f,
                    "invalid control character 0x{byte:02x} at line {line}, column {col}"
                )
            }
            FilterError::UnterminatedComment { line, col } => {
                write!(
                    f,
                    "unterminated block comment starting at line {line}, column {col}"
                )
            }
            FilterError::UnterminatedString { line, col } => {
                write!(
                    f,
                    "unterminated string literal starting at line {line}, column {col}"
                )
            }
            FilterError::ForbiddenConstruct { token, line, col } => {
                write!(
                    f,
                    "forbidden tactic construct '{token}' at line {line}, column {col}"
                )
            }
            FilterError::ForbiddenToken { token, line, col } => {
                write!(f, "forbidden token '{token}' at line {line}, column {col}")
            }
        }
    }
}

impl std::error::Error for FilterError {}

const ALLOWED_TACTIC_HEADS: &[&str] = &[
    "intro",
    "intros",
    "intro1",
    "intro2",
    "rename",
    "rename_i",
    "obtain",
    "rcases",
    "rintro",
    "match_cases",
    "match",
    "use",
    "exact",
    "exact_mod_cast",
    "refine",
    "apply",
    "constructor",
    "constructors",
    "cases",
    "case_tac",
    "on_left",
    "on_right",
    "rfl",
    "assumption",
    "assumption_mod_cast",
    "trivial",
    "trivial_mod_cast",
    "simp",
    "simpa",
    "simp_all",
    "simp_arith",
    "simp_rw",
    "dsimp",
    "norm_num",
    "norm_num1",
    "norm_cast",
    "push_cast",
    "decide",
    "ring",
    "ring_nf",
    "ring_exp",
    "ring!",
    "field_simp",
    "linarith",
    "nlinarith",
    "abel",
    "positivity",
    "omega",
    "grind",
    "aesop",
    "haveI",
    "have",
    "show",
    "suffices",
    "subst",
    "substs",
    "induction",
    "induction'",
    "inductionOn",
    "cases'",
    "rw",
    "rw'",
    "erw",
    "nth_rw",
    "left",
    "right",
    "guard_hyp",
    "guard_target",
    "unfold",
    "delta",
    "change",
    "convert",
    "unfold_using",
    "simp_where_generalize",
    "done",
    "trivial_1",
    "focus",
    "all_goals",
    "any_goals",
    "repeat",
    "try",
    "first",
    "solve",
    "ext",
    "ext1",
    "lift",
    "split_ifs",
    "split",
    "congr",
    "congr'",
    "gcongr",
    "revert",
    "clear",
    "symm",
    "trans",
    "tauto",
    "fin_cases",
    "interval_cases",
    "choose",
    "contrapose",
    "exfalso",
    "by_contra",
    "by_cases",
    "generalize",
    "apply_fun",
    "contradiction",
];

const FORBIDDEN_ANYWHERE_TOKENS: &[&str] = &[
    "sorry",
    "sorryAx",
    "admit",
    "stop",
    "native_decide",
    "unsafe",
    "extern",
    "#eval",
    "#check",
    "#print",
    "#exit",
    "#load",
    "#help",
    "#guard_msgs",
    "IO.println",
    "IO.getEnv",
    "System.Process",
];

/// Filters and sanitizes user-submitted tactic script according to ADR-002.
/// Returns the original body unchanged on success, or FilterError on violation.
pub fn filter_tactic_body(body: &str, limits: &Limits) -> Result<String, FilterError> {
    // -------------------------------------------------------------------------
    // a. NORMALISE: Check length, line count, and forbidden control characters
    // -------------------------------------------------------------------------
    let bytes_count = body.len();
    let lines_count = body.lines().count();

    if bytes_count > limits.max_bytes || lines_count > limits.max_lines {
        return Err(FilterError::TooLarge {
            bytes: bytes_count,
            lines: lines_count,
            max_bytes: limits.max_bytes,
            max_lines: limits.max_lines,
        });
    }

    let mut line_num = 1;
    let mut col_num = 1;
    for &b in body.as_bytes() {
        if b == b'\n' {
            line_num += 1;
            col_num = 1;
            continue;
        }
        if (b < 0x20 && b != b'\t' && b != b'\r') || b == 0x7F {
            return Err(FilterError::InvalidControlChar {
                byte: b,
                line: line_num,
                col: col_num,
            });
        }
        col_num += 1;
    }

    // -------------------------------------------------------------------------
    // b. STRIP COMMENTS + STRINGS: Replace with spaces of equal length
    // -------------------------------------------------------------------------
    let stripped = strip_comments_and_strings(body)?;

    // -------------------------------------------------------------------------
    // c. TOKEN SCAN & ALLOW-LIST ON TACTIC HEADS
    // -------------------------------------------------------------------------
    // Tactic statements can be separated by newline or semicolon ';'
    validate_tactic_heads(&stripped)?;

    // -------------------------------------------------------------------------
    // d. FORBIDDEN TOKENS ANYWHERE: Reject sorry, sorryAx, @[, etc.
    // -------------------------------------------------------------------------
    validate_forbidden_tokens(&stripped)?;

    // -------------------------------------------------------------------------
    // e. SUCCESS: Return original body unmodified
    // -------------------------------------------------------------------------
    Ok(body.to_string())
}

fn strip_comments_and_strings(input: &str) -> Result<String, FilterError> {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut out: Vec<char> = Vec::with_capacity(len);
    let mut i = 0;

    let mut line = 1;
    let mut col = 1;

    // Helper to calculate line and col at index k
    let pos_at = |k: usize| -> (usize, usize) {
        let mut l = 1;
        let mut c = 1;
        for &ch in &chars[..k] {
            if ch == '\n' {
                l += 1;
                c = 1;
            } else {
                c += 1;
            }
        }
        (l, c)
    };

    while i < len {
        let c = chars[i];

        // Check for line comment `--`
        if c == '-' && i + 1 < len && chars[i + 1] == '-' {
            // Check it's not a block comment starting with `/-` or something
            while i < len && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
            continue;
        }

        // Check for block comment `/-`
        if c == '/' && i + 1 < len && chars[i + 1] == '-' {
            let (start_line, start_col) = pos_at(i);
            let mut depth = 1;
            out.push(' ');
            out.push(' ');
            i += 2;

            while i < len && depth > 0 {
                if chars[i] == '/' && i + 1 < len && chars[i + 1] == '-' {
                    depth += 1;
                    out.push(' ');
                    out.push(' ');
                    i += 2;
                } else if chars[i] == '-' && i + 1 < len && chars[i + 1] == '/' {
                    depth -= 1;
                    out.push(' ');
                    out.push(' ');
                    i += 2;
                } else {
                    if chars[i] == '\n' {
                        out.push('\n');
                    } else {
                        out.push(' ');
                    }
                    i += 1;
                }
            }

            if depth > 0 {
                return Err(FilterError::UnterminatedComment {
                    line: start_line,
                    col: start_col,
                });
            }
            continue;
        }

        // Check for string literal `"..."`
        if c == '"' {
            let (start_line, start_col) = pos_at(i);
            out.push(' ');
            i += 1;
            let mut escaped = false;
            let mut closed = false;

            while i < len {
                let ch = chars[i];
                if escaped {
                    escaped = false;
                    out.push(if ch == '\n' { '\n' } else { ' ' });
                    i += 1;
                } else if ch == '\\' {
                    escaped = true;
                    out.push(' ');
                    i += 1;
                } else if ch == '"' {
                    out.push(' ');
                    i += 1;
                    closed = true;
                    break;
                } else {
                    out.push(if ch == '\n' { '\n' } else { ' ' });
                    i += 1;
                }
            }

            if !closed {
                return Err(FilterError::UnterminatedString {
                    line: start_line,
                    col: start_col,
                });
            }
            continue;
        }

        // Check for character literal `'...'`
        if c == '\'' {
            out.push(' ');
            i += 1;
            let mut escaped = false;
            while i < len {
                let ch = chars[i];
                if escaped {
                    escaped = false;
                    out.push(if ch == '\n' { '\n' } else { ' ' });
                    i += 1;
                } else if ch == '\\' {
                    escaped = true;
                    out.push(' ');
                    i += 1;
                } else if ch == '\'' {
                    out.push(' ');
                    i += 1;
                    break;
                } else {
                    out.push(if ch == '\n' { '\n' } else { ' ' });
                    i += 1;
                }
            }
            continue;
        }

        // Check for French quotes `«...»`
        if c == '«' {
            out.push(' ');
            i += 1;
            while i < len && chars[i] != '»' {
                out.push(if chars[i] == '\n' { '\n' } else { ' ' });
                i += 1;
            }
            if i < len && chars[i] == '»' {
                out.push(' ');
                i += 1;
            }
            continue;
        }

        out.push(c);
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
        i += 1;
    }

    let _ = (line, col);
    Ok(out.into_iter().collect())
}

fn validate_tactic_heads(stripped: &str) -> Result<(), FilterError> {
    let mut line_num = 1;

    for raw_line in stripped.split('\n') {
        // A single line can contain multiple tactic statements separated by ';'
        let mut col_offset = 1;
        for segment in raw_line.split(';') {
            let trimmed = segment.trim_start();
            let leading_spaces = segment.len() - trimmed.len();
            let stmt_col = col_offset + leading_spaces;

            if !trimmed.is_empty() {
                // Find first token
                let (token, _token_len) = extract_first_token(trimmed);
                if !token.is_empty() {
                    // Check if token starts with punctuation like '(' or '{' or '·' or '<'
                    let clean_token = token.trim_start_matches(|c: char| {
                        c == '(' || c == '{' || c == '[' || c == '⟨' || c == '·' || c == '|'
                    });

                    if !clean_token.is_empty() {
                        // Check for forbidden constructs / heads
                        if clean_token.starts_with('#')
                            || clean_token == "native_decide"
                            || clean_token == "import"
                            || clean_token == "set_option"
                            || clean_token == "axiom"
                            || clean_token == "opaque"
                            || clean_token == "def"
                            || clean_token == "abbrev"
                            || clean_token == "theorem"
                            || clean_token == "lemma"
                            || clean_token == "example"
                            || clean_token == "structure"
                            || clean_token == "inductive"
                            || clean_token == "class"
                            || clean_token == "macro"
                            || clean_token == "macro_rules"
                            || clean_token == "elab"
                            || clean_token == "elab_rules"
                            || clean_token == "syntax"
                            || clean_token == "notation"
                            || clean_token == "scoped"
                            || clean_token == "initialize"
                            || clean_token == "unsafe"
                            || clean_token == "partial"
                            || clean_token == "extern"
                            || clean_token == "deriving"
                            || clean_token == "universe"
                            || clean_token == "run_cmd"
                            || clean_token == "attribute"
                            || clean_token == "instance"
                            || clean_token == "open"
                            || clean_token == "namespace"
                            || clean_token == "section"
                            || clean_token == "end"
                            || clean_token == "variable"
                        {
                            return Err(FilterError::ForbiddenConstruct {
                                token: clean_token.to_string(),
                                line: line_num,
                                col: stmt_col,
                            });
                        }

                        // Check against allowed tactic heads
                        if !ALLOWED_TACTIC_HEADS.contains(&clean_token) {
                            return Err(FilterError::ForbiddenConstruct {
                                token: clean_token.to_string(),
                                line: line_num,
                                col: stmt_col,
                            });
                        }
                    }
                }
            }
            col_offset += segment.len() + 1; // +1 for ';'
        }
        line_num += 1;
    }

    Ok(())
}

fn validate_forbidden_tokens(stripped: &str) -> Result<(), FilterError> {
    let mut line_num = 1;

    for raw_line in stripped.split('\n') {
        let mut col_num = 1;
        let mut idx = 0;
        let chars: Vec<char> = raw_line.chars().collect();

        while idx < chars.len() {
            let ch = chars[idx];

            // Check for @[ attribute syntax
            if ch == '@' && idx + 1 < chars.len() && chars[idx + 1] == '[' {
                return Err(FilterError::ForbiddenToken {
                    token: "@[".to_string(),
                    line: line_num,
                    col: col_num,
                });
            }

            if is_token_char(ch) || ch == '#' {
                let start_col = col_num;
                let mut token_str = String::new();
                while idx < chars.len() && (is_token_char(chars[idx]) || chars[idx] == '#') {
                    token_str.push(chars[idx]);
                    idx += 1;
                    col_num += 1;
                }

                for &forbidden in FORBIDDEN_ANYWHERE_TOKENS {
                    if token_str == forbidden || token_str.starts_with(forbidden) {
                        return Err(FilterError::ForbiddenToken {
                            token: token_str,
                            line: line_num,
                            col: start_col,
                        });
                    }
                }
            } else {
                idx += 1;
                col_num += 1;
            }
        }
        line_num += 1;
    }

    Ok(())
}

fn extract_first_token(s: &str) -> (&str, usize) {
    let mut end = 0;
    for c in s.chars() {
        if is_token_char(c)
            || c == '#'
            || c == '('
            || c == '{'
            || c == '['
            || c == '⟨'
            || c == '·'
            || c == '|'
        {
            end += c.len_utf8();
        } else {
            break;
        }
    }
    (&s[..end], end)
}

fn is_token_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '\'' || c == '.'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accepts_valid_tactic_scripts() {
        let limits = Limits::default();
        let cases = [
            "intro n\nsimp",
            "intro n; simp",
            "  exact Nat.add_zero n",
            "simp only [Nat.add_zero]",
            "rcases h with ⟨a, b⟩",
            "obtain ⟨x, hx⟩ := h",
            "intro a b\nomega",
            "intro P Q ⟨hp, hq⟩\nexact ⟨hq, hp⟩",
        ];

        for code in cases {
            let res = filter_tactic_body(code, &limits);
            assert!(
                res.is_ok(),
                "Expected '{}' to be accepted, got: {:?}",
                code,
                res.err()
            );
        }
    }

    #[test]
    fn test_rejects_forbidden_constructs() {
        let limits = Limits::default();
        let cases = [
            ("import Mathlib", "import"),
            ("set_option maxHeartbeats 1 in simp", "set_option"),
            ("exact unsafe 1", "unsafe"),
            ("native_decide", "native_decide"),
            ("axiom h : False", "axiom"),
            ("opaque foo : True := trivial", "opaque"),
            ("attribute [instance] Foo.bar", "attribute"),
            ("run_cmd IO.println 1", "run_cmd"),
            ("#eval 1", "#eval"),
            ("macro_rules | `(x) => x", "macro_rules"),
            ("example : True := trivial", "example"),
            ("theorem goal : True := trivial", "theorem"),
            ("initialize registerBuiltinAttribute 1", "initialize"),
            ("elab_rules : term | `(`x`) => x", "elab_rules"),
            ("exact sorryAx", "sorryAx"),
            ("exact (by sorry)", "sorry"),
        ];

        for (code, expected_token) in cases {
            let res = filter_tactic_body(code, &limits);
            assert!(
                res.is_err(),
                "Expected '{}' to be rejected with token '{}'",
                code,
                expected_token
            );
        }
    }

    #[test]
    fn test_does_not_reject_comments_or_string_literals() {
        let limits = Limits::default();
        let cases = [
            "-- #eval is dangerous\nintro n\nsimp",
            "exact \"#eval\"",
            "/- #eval -/ intro n\nsimp",
            "/- a /- nested -/ b -/\nintro n\nsimp",
            "exact '/'",
        ];

        for code in cases {
            let res = filter_tactic_body(code, &limits);
            assert!(
                res.is_ok(),
                "Expected '{}' to pass comment/string scanner, got: {:?}",
                code,
                res.err()
            );
        }
    }

    #[test]
    fn test_size_limits() {
        let limits = Limits {
            max_bytes: 50,
            max_lines: 3,
        };

        let long_bytes = "intro n\n".repeat(10);
        let res = filter_tactic_body(&long_bytes, &limits);
        assert!(matches!(res, Err(FilterError::TooLarge { .. })));

        let many_lines = "simp\n".repeat(5);
        let res = filter_tactic_body(&many_lines, &limits);
        assert!(matches!(res, Err(FilterError::TooLarge { .. })));
    }
}
