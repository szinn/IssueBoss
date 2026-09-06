/// Convert `text` to a URL-safe, filesystem-safe slug.
///
/// Rules:
/// - Lowercase
/// - ASCII letters and digits kept; everything else → hyphen
/// - Consecutive hyphens collapsed to one
/// - Leading and trailing hyphens stripped
/// - Truncated to 50 characters (at a hyphen boundary when possible)
pub fn slugify(text: &str) -> String {
    let raw: String = text.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();

    // Collapse consecutive hyphens
    let mut slug = String::with_capacity(raw.len());
    let mut prev_hyphen = false;
    for c in raw.chars() {
        if c == '-' {
            if !prev_hyphen && !slug.is_empty() {
                slug.push('-');
            }
            prev_hyphen = true;
        } else {
            slug.push(c);
            prev_hyphen = false;
        }
    }

    // Strip trailing hyphen
    let slug = slug.trim_end_matches('-').to_owned();

    // Truncate to 50 chars, preferring a hyphen boundary
    if slug.len() <= 50 {
        return slug;
    }
    let truncated = &slug[..50];
    if let Some(pos) = truncated.rfind('-') {
        truncated[..pos].to_owned()
    } else {
        truncated.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn basic_lowercase_and_hyphens() {
        assert_eq!(slugify("Hello World!"), "hello-world");
    }

    #[test]
    fn trims_whitespace_and_collapses_spaces() {
        assert_eq!(slugify("  Add  Rate  Limiting  "), "add-rate-limiting");
    }

    #[test]
    fn strips_non_ascii() {
        assert_eq!(slugify("café"), "caf");
    }

    #[test]
    fn truncates_at_50_chars() {
        let long = "a".repeat(100);
        assert!(slugify(&long).len() <= 50);
    }

    #[test]
    fn truncates_at_hyphen_boundary() {
        // 48 'a's + "-bc" = 51 chars total — truncates at 50, finds hyphen at
        // 48
        let input = format!("{}-bc", "a".repeat(48));
        let result = slugify(&input);
        assert_eq!(result.len(), 48);
        assert!(!result.ends_with('-'));
    }

    #[test]
    fn empty_input() {
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn all_special_chars() {
        assert_eq!(slugify("!!!"), "");
    }

    #[test]
    fn pure_unicode_returns_empty() {
        // All non-ASCII chars map to hyphens; after collapse and strip, result
        // is "".
        assert_eq!(slugify("日本語"), "");
    }

    #[test]
    fn leading_special_chars_stripped() {
        // Leading non-alphanumeric chars produce leading hyphens which are
        // collapsed but then stripped because the slug must not start
        // with a hyphen.
        assert_eq!(slugify("!!!hello"), "hello");
    }

    #[test]
    fn exactly_50_chars_not_truncated() {
        // A slug that is exactly 50 characters long must be returned unchanged.
        let input = "a".repeat(50);
        let result = slugify(&input);
        assert_eq!(result.len(), 50);
        assert_eq!(result, input);
    }
}
