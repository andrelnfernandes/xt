//! The line filter of the text viewer (`&`): which lines stay on screen.
//!
//! Plain text uses the same rules as the viewer's search (`*` inside the term,
//! accents ignored along with case). Regular expressions use the `regex`
//! crate, whose time is linear in the size of the text: a hostile file cannot
//! make it backtrack.

use regex::{Regex, RegexBuilder};

use crate::search::{Pattern, SearchSpec};

/// What to filter by and how.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineFilterSpec {
    pub text: String,
    pub case_sensitive: bool,
    /// `text` is a regular expression.
    pub regex: bool,
    /// Keeps the lines that do not match.
    pub invert: bool,
}

impl LineFilterSpec {
    /// E.g. "regex, ignore case, inverted".
    pub fn mode_label(&self) -> String {
        let kind = if self.regex { "regex" } else { "text" };
        let case = if self.case_sensitive {
            "match case"
        } else {
            "ignore case"
        };
        let invert = if self.invert { ", inverted" } else { "" };
        format!("{kind}, {case}{invert}")
    }
}

#[derive(Debug, Clone)]
enum How {
    Text(Pattern),
    Regex(Regex),
}

/// A filter ready to run.
#[derive(Debug, Clone)]
pub struct LineMatcher {
    spec: LineFilterSpec,
    how: How,
}

impl LineMatcher {
    pub fn new(spec: &LineFilterSpec) -> Result<Self, String> {
        let how = if spec.regex {
            let regex = RegexBuilder::new(&spec.text)
                .case_insensitive(!spec.case_sensitive)
                .build()
                .map_err(|e| format!("Invalid regex: {}", regex_error(&e)))?;
            How::Regex(regex)
        } else {
            How::Text(Pattern::new(&SearchSpec {
                text: spec.text.clone(),
                case_sensitive: spec.case_sensitive,
                hex: false,
            })?)
        };
        Ok(LineMatcher {
            spec: spec.clone(),
            how,
        })
    }

    pub fn spec(&self) -> &LineFilterSpec {
        &self.spec
    }

    pub fn matches(&self, line: &str) -> bool {
        let found = match &self.how {
            How::Text(pattern) => pattern.find_in_line(line, 0).is_some(),
            How::Regex(regex) => regex.is_match(line),
        };
        found != self.spec.invert
    }
}

/// The reason, in one line: a syntax error spans several, with a drawing of
/// where it is, and the prompt's footer has only one.
fn regex_error(error: &regex::Error) -> String {
    match error {
        regex::Error::CompiledTooBig(_) => "too big".into(),
        other => {
            let text = other.to_string();
            let last = text.lines().last().unwrap_or_default();
            last.strip_prefix("error: ").unwrap_or(last).to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn matcher(text: &str, regex: bool) -> LineMatcher {
        LineMatcher::new(&LineFilterSpec {
            text: text.into(),
            regex,
            ..Default::default()
        })
        .unwrap()
    }

    #[test]
    fn plain_text_follows_the_search_rules() {
        let m = matcher("erro*disco", false);
        assert!(m.matches("ERRO no disco"));
        assert!(!m.matches("disco com erro"));
        assert!(matcher("AÇÃO", false).matches("uma ação"));
        // In plain text, regex characters are themselves.
        assert!(!matcher("a.c", false).matches("abc"));
        assert!(matcher("a.c", false).matches("a.c"));
    }

    #[test]
    fn regex_matches_and_ignores_case_by_default() {
        let m = matcher(r"^\d{3} (error|warn)\b", true);
        assert!(m.matches("500 ERROR disk"));
        assert!(!m.matches("x 500 error"));
        assert!(!m.matches("500 errors"));
    }

    #[test]
    fn case_and_inversion_apply_to_both_kinds() {
        for regex in [false, true] {
            let spec = LineFilterSpec {
                text: "Erro".into(),
                case_sensitive: true,
                regex,
                invert: false,
            };
            let m = LineMatcher::new(&spec).unwrap();
            assert!(m.matches("Erro"));
            assert!(!m.matches("erro"));
            let m = LineMatcher::new(&LineFilterSpec {
                invert: true,
                ..spec
            })
            .unwrap();
            assert!(!m.matches("Erro"));
            assert!(m.matches("erro"));
        }
    }

    #[test]
    fn invalid_regex_is_explained_in_one_line() {
        let spec = LineFilterSpec {
            text: "(abc".into(),
            regex: true,
            ..Default::default()
        };
        let error = LineMatcher::new(&spec).unwrap_err();
        assert_eq!(error, "Invalid regex: unclosed group");
    }

    #[test]
    fn a_regex_too_big_to_compile_is_refused() {
        let spec = LineFilterSpec {
            text: "(a{1000}){1000}{1000}".into(),
            regex: true,
            ..Default::default()
        };
        let error = LineMatcher::new(&spec).unwrap_err();
        assert_eq!(error, "Invalid regex: too big");
    }

    #[test]
    fn nested_repetition_does_not_backtrack() {
        // Exponential in a backtracking engine; here, immediate.
        let m = matcher("^(a+)+$", true);
        let line = format!("{}!", "a".repeat(100_000));
        assert!(!m.matches(&line));
    }

    proptest! {
        #[test]
        fn never_panics(text in ".{0,40}", line in ".{0,200}", regex: bool, invert: bool) {
            let spec = LineFilterSpec { text, regex, invert, case_sensitive: false };
            if let Ok(m) = LineMatcher::new(&spec) {
                let _ = m.matches(&line);
            }
        }
    }
}
