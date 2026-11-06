//! Small helpers shared by the pick and place file, the stencil and the fabrication package: the
//! reading of decimals by integer arithmetic, the matching of footprint names against patterns, the
//! reading of `key = value` settings text and the JSON text of strings.

use crate::fabcodes::FabError;

/// A decimal text (`0.12`, `-3`, `+1.5`) as millionths, with at most six decimals. `None` when the
/// text is not a plain decimal or does not fit.
pub fn parse_micro(text: &str) -> Option<i64> {
    let text = text.trim();
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (whole, fraction) = match body.split_once('.') {
        Some((w, f)) => (w, f),
        None => (body, ""),
    };
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    if !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 6
    {
        return None;
    }
    let whole: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let mut fraction_micro: i64 = if fraction.is_empty() {
        0
    } else {
        fraction.parse().ok()?
    };
    for _ in fraction.len()..6 {
        fraction_micro *= 10;
    }
    let value = whole.checked_mul(1_000_000)?.checked_add(fraction_micro)?;
    Some(if negative { -value } else { value })
}

/// True when `text` matches `pattern`, where `*` stands for any run of characters (also none) and
/// `?` for one character; the comparison ignores the case.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None;
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some((pi, ti));
            pi += 1;
        } else if let Some((sp, st)) = star {
            pi = sp + 1;
            ti = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// A line of a settings text, without its comment and its spaces. `None` for a blank line or a
/// comment.
pub fn settings_line(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        None
    } else {
        Some(trimmed)
    }
}

/// Splits `key = value`; the error names the line.
pub fn split_pair<'a>(
    line: &'a str,
    number: usize,
    code: &'static str,
) -> Result<(&'a str, &'a str), FabError> {
    line.split_once('=')
        .map(|(k, v)| (k.trim(), v.trim()))
        .ok_or_else(|| FabError::new(code, format!("line {number}: not key = value")))
}

/// A string as a JSON string, with its quotes.
pub fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_are_read_by_integer_arithmetic() {
        assert_eq!(parse_micro("0.12"), Some(120_000));
        assert_eq!(parse_micro("-3"), Some(-3_000_000));
        assert_eq!(parse_micro("+1.5"), Some(1_500_000));
        assert_eq!(parse_micro(".5"), Some(500_000));
        assert_eq!(parse_micro("2."), Some(2_000_000));
        assert_eq!(parse_micro("0.000001"), Some(1));
        for bad in ["", ".", "1.2.3", "0.1234567", "1e3", "abc", "-", "1 2"] {
            assert_eq!(parse_micro(bad), None, "{bad}");
        }
        assert_eq!(parse_micro("99999999999999999999"), None);
    }

    #[test]
    fn patterns_match_footprint_names_without_regard_to_case() {
        assert!(glob_match("SOT23*", "sot23-5"));
        assert!(glob_match("*QFN*", "UQFN16"));
        assert!(glob_match("R0603", "r0603"));
        assert!(glob_match("C?402", "C0402"));
        assert!(glob_match("*", ""));
        assert!(!glob_match("SOT23*", "SOIC8"));
        assert!(!glob_match("R0603", "R06031"));
        assert!(!glob_match("C?402", "C402"));
        assert!(glob_match("a*b*c", "aXXbYYc"));
        assert!(!glob_match("a*b*c", "aXXbYY"));
    }

    #[test]
    fn settings_lines_skip_blanks_and_comments() {
        assert_eq!(settings_line("  # note"), None);
        assert_eq!(settings_line("   "), None);
        assert_eq!(settings_line("  a = 1 "), Some("a = 1"));
        let (k, v) = split_pair("a = b = c", 1, "X").unwrap();
        assert_eq!((k, v), ("a", "b = c"));
        assert!(split_pair("nothing", 4, "X").is_err());
    }

    #[test]
    fn strings_become_json_strings() {
        assert_eq!(json_string("a\"b"), "\"a\\\"b\"");
        assert_eq!(json_string("é"), "\"é\"");
    }
}
