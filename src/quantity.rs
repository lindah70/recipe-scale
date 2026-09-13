//! Parsing and formatting for the leading quantity on an ingredient line.
//!
//! Recipes write amounts as plain numbers ("2"), decimals ("0.5"), simple
//! fractions ("1/2"), or mixed numbers ("1 1/2"). This module only looks at
//! the start of a line, since that is the only place a scalable amount ever
//! appears in an ingredient list ("Bake at 350F" should pass through as-is).

/// Splits leading whitespace off `s`, then splits the first whitespace-
/// delimited word from the rest. Returns `None` if there is no word.
fn split_first_word(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    let idx = s.find(char::is_whitespace).unwrap_or(s.len());
    Some((&s[..idx], &s[idx..]))
}

/// Parses a single token as either a decimal number or an `a/b` fraction.
fn parse_token(tok: &str) -> Option<f64> {
    if let Some((num, den)) = tok.split_once('/') {
        let num: f64 = num.parse().ok()?;
        let den: f64 = den.parse().ok()?;
        if den == 0.0 {
            return None;
        }
        Some(num / den)
    } else {
        tok.parse().ok()
    }
}

/// Tries to read a quantity from the start of `line`. On success, returns the
/// parsed value together with whatever text follows it (unit and ingredient
/// name, including the separating whitespace). Returns `None` if the line
/// does not start with a number, in which case the caller should pass the
/// line through unchanged.
pub fn parse_leading_quantity(line: &str) -> Option<(f64, &str)> {
    let (first, rest) = split_first_word(line)?;
    let value = parse_token(first)?;

    // A bare integer might be the whole part of a mixed number like
    // "1 1/2 cups". Only try this when the first token itself was not
    // already a decimal or a fraction.
    if !first.contains('.') && !first.contains('/') {
        if let Some((second, rest2)) = split_first_word(rest) {
            if second.contains('/') {
                if let Some(frac) = parse_token(second) {
                    return Some((value + frac, rest2));
                }
            }
        }
    }

    Some((value, rest))
}

/// Looks for a serving-count header such as "Serves 4", "Serves: 6", or
/// "Yield 8 servings" at the start of a line, and returns the number if
/// found. This lets the caller fall back to it when `--from` is not given
/// on the command line.
pub fn detect_serving_count(line: &str) -> Option<f64> {
    let trimmed = line.trim_start();
    let lower = trimmed.to_ascii_lowercase();
    let keyword = ["serves", "yields", "yield"]
        .iter()
        .find(|k| lower.starts_with(**k))?;

    let rest = trimmed[keyword.len()..]
        .trim_start()
        .trim_start_matches(':')
        .trim_start();
    let (first, _) = split_first_word(rest)?;
    first.parse().ok()
}

const NAMED_FRACTIONS: &[(f64, &str)] = &[
    (0.125, "1/8"),
    (0.25, "1/4"),
    (0.333_333, "1/3"),
    (0.375, "3/8"),
    (0.5, "1/2"),
    (0.625, "5/8"),
    (0.666_667, "2/3"),
    (0.75, "3/4"),
    (0.875, "7/8"),
];

/// Formats a scaled quantity for display. Whole numbers print bare, and
/// fractional parts that are close to a common cooking fraction (eighths,
/// thirds) print as that fraction rather than a decimal. Anything else
/// falls back to a decimal trimmed of trailing zeros.
pub fn format_quantity(value: f64) -> String {
    // Round away floating point noise before splitting into whole/fraction.
    let rounded = (value * 1000.0).round() / 1000.0;
    let whole = rounded.trunc();
    let frac = (rounded - whole).abs();

    for (target, text) in NAMED_FRACTIONS {
        if (frac - target).abs() < 0.01 {
            return if whole.abs() < 0.5 {
                text.to_string()
            } else {
                format!("{} {}", whole as i64, text)
            };
        }
    }

    if frac < 0.01 {
        return format!("{}", whole as i64);
    }

    let mut s = format!("{:.2}", rounded);
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_integer() {
        assert_eq!(parse_leading_quantity("2 cups flour"), Some((2.0, " cups flour")));
    }

    #[test]
    fn parses_simple_fraction() {
        assert_eq!(parse_leading_quantity("1/2 tsp salt"), Some((0.5, " tsp salt")));
    }

    #[test]
    fn parses_mixed_number() {
        assert_eq!(
            parse_leading_quantity("1 1/2 cups ricotta"),
            Some((1.5, " cups ricotta"))
        );
    }

    #[test]
    fn passes_through_non_numeric_lines() {
        assert_eq!(parse_leading_quantity("Preheat oven to 375F"), None);
        assert_eq!(parse_leading_quantity(""), None);
    }

    #[test]
    fn formats_named_fractions() {
        assert_eq!(format_quantity(2.25), "2 1/4");
        assert_eq!(format_quantity(0.75), "3/4");
        assert_eq!(format_quantity(3.0), "3");
    }

    #[test]
    fn detects_serves_header() {
        assert_eq!(detect_serving_count("Serves 4"), Some(4.0));
        assert_eq!(detect_serving_count("Serves: 4"), Some(4.0));
        assert_eq!(detect_serving_count("serves 4 people"), Some(4.0));
    }

    #[test]
    fn detects_yield_header() {
        assert_eq!(detect_serving_count("Yield 6"), Some(6.0));
        assert_eq!(detect_serving_count("Yields: 8 servings"), Some(8.0));
    }

    #[test]
    fn detect_serving_count_ignores_non_header_lines() {
        assert_eq!(detect_serving_count("2 cups flour"), None);
        assert_eq!(detect_serving_count("Bake at 375F"), None);
        assert_eq!(detect_serving_count(""), None);
    }
}
