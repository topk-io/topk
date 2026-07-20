use crate::Error;

// Translates a Postgres-style regexp (pattern, flags) pair into Rust regex.
pub fn translate(pattern: &str, pg_flags: &str) -> Result<(String, Option<String>), Error> {
    let mut case_insensitive = false;
    let mut dot_matches_newline = true; // PG default
    let mut line_anchors = false;
    let mut expanded = false;
    let mut literal = false;

    for c in pg_flags.chars() {
        match c {
            'i' => case_insensitive = true,
            'c' => case_insensitive = false,
            's' => {
                dot_matches_newline = true;
                line_anchors = false;
            }
            'n' | 'm' => {
                dot_matches_newline = false;
                line_anchors = true;
            }
            'p' => {
                dot_matches_newline = false;
                line_anchors = false;
            }
            'w' => {
                dot_matches_newline = true;
                line_anchors = true;
            }
            'x' => expanded = true,
            't' => expanded = false,
            'q' => literal = true,
            'g' => {
                return Err(Error::Invalid(
                    "regexp_like() does not support the \"global\" option".to_string(),
                ));
            }
            'b' | 'e' => {
                return Err(Error::Unsupported(format!(
                    "regular expression option \"{c}\" is not supported"
                )));
            }
            other => {
                return Err(Error::Invalid(format!(
                    "invalid regular expression option: \"{other}\""
                )));
            }
        }
    }

    let pattern = if literal {
        regex::escape(pattern)
    } else {
        pattern.to_string()
    };

    let mut flags = String::new();
    if dot_matches_newline {
        flags.push('s');
    }
    if line_anchors {
        flags.push('m');
    }
    if case_insensitive {
        flags.push('i');
    }
    // `q` makes the whole pattern literal, so expanded-whitespace mode must
    // not apply — escaped patterns keep their whitespace significant.
    if expanded && !literal {
        flags.push('x');
    }

    let composed = if flags.is_empty() {
        pattern.clone()
    } else {
        format!("(?{flags}){pattern}")
    };
    // Reactor validates too, but only when the expression reaches an executor
    // compile — validating here makes invalid patterns error unconditionally,
    // like PG, even for queries that short-circuit before execution.
    regex::Regex::new(&composed)
        .map_err(|e| Error::Invalid(format!("invalid regular expression: {e}")))?;

    let flags = if flags.is_empty() { None } else { Some(flags) };

    Ok((pattern, flags))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::default("foo", "", "foo", Some("s"))]
    #[case::case_insensitive("foo", "i", "foo", Some("si"))]
    #[case::case_sensitive_wins("foo", "ic", "foo", Some("s"))]
    #[case::newline_sensitive("foo", "n", "foo", Some("m"))]
    #[case::partial_newline("foo", "p", "foo", None)]
    #[case::inverse_partial("foo", "w", "foo", Some("sm"))]
    #[case::last_newline_wins("foo", "ns", "foo", Some("s"))]
    #[case::expanded("f o o", "x", "f o o", Some("sx"))]
    #[case::tight_wins("foo", "xt", "foo", Some("s"))]
    #[case::literal("a.b*", "q", "a\\.b\\*", Some("s"))]
    #[case::literal_ignores_expanded("a b", "qx", "a b", Some("s"))]
    fn test_translate(
        #[case] pattern: &str,
        #[case] pg_flags: &str,
        #[case] expected_pattern: &str,
        #[case] expected_flags: Option<&str>,
    ) {
        let (pattern, flags) = translate(pattern, pg_flags).unwrap();
        assert_eq!(pattern, expected_pattern);
        assert_eq!(flags.as_deref(), expected_flags);
    }

    #[rstest]
    #[case::global("foo", "g")]
    #[case::bre("foo", "b")]
    #[case::unknown("foo", "z")]
    #[case::backreference("(a)\\1", "")]
    fn test_translate_invalid(#[case] pattern: &str, #[case] pg_flags: &str) {
        let err = translate(pattern, pg_flags).expect_err("translate should fail");
        assert!(matches!(err, Error::Invalid(..) | Error::Unsupported(..)));
    }
}
