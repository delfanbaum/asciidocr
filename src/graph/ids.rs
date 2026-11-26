//! IDs for sections that weren't given one, generated from their titles.
//!
//! The AsciiDoc specification describes this as the processor's job: a section without an
//! explicit ID gets one made from its title, unless the `sectids` attribute is unset, using the
//! `idprefix` (default `_`) and `idseparator` (default `_`) attributes. It follows Asciidoctor,
//! so `== Wiley & Sons, Inc.` gets `_wiley_sons_inc`, and a second `== Introduction` gets
//! `_introduction_2`.
//!
//! The generated IDs are not part of the graph's JSON (TCK) output, whose expected ASG has no ID
//! for such a section; they are for the backends, and for cross references, which resolve to the
//! section's title.

use std::collections::{HashMap, HashSet};

use once_cell::sync::Lazy;
use regex::Regex;

use super::inlines::{Inline, InlineLiteralName, InlineRefVariant, InlineSpanVariant};

/// How section IDs are generated, from the document's attributes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdSettings {
    /// `sectids` (set unless unset with `:sectids!:` or `:!sectids:`)
    pub enabled: bool,
    /// `idprefix`
    pub prefix: String,
    /// `idseparator` (one character, or none)
    pub separator: Option<char>,
}

impl Default for IdSettings {
    fn default() -> Self {
        IdSettings {
            enabled: true,
            prefix: "_".to_string(),
            separator: Some('_'),
        }
    }
}

impl IdSettings {
    /// The settings the document's attributes ask for. (An attribute that is changed partway
    /// through a document counts with its last value.)
    pub fn from_attributes(attributes: &HashMap<String, String>) -> Self {
        let mut settings = IdSettings::default();
        if attributes.contains_key("sectids!") || attributes.contains_key("!sectids") {
            settings.enabled = false;
        }
        if let Some(prefix) = attributes.get("idprefix") {
            settings.prefix = prefix.clone();
        }
        if let Some(separator) = attributes.get("idseparator") {
            // like Asciidoctor, a longer separator is cut to its first character
            settings.separator = separator.chars().next();
        }
        settings
    }
}

/// The ID for a section with this title (as HTML, see [`title_for_id`]), before it is made unique
pub fn generate_id(title_html: &str, settings: &IdSettings) -> String {
    // tags, character references, and characters that can't be in an ID
    static INVALID_CHARS: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"<[^>]+>|&(?:[a-z][a-z]+\d{0,2}|#\d{3,6}|#x[\da-f]{3,5});|[^ \w\-.]+").unwrap()
    });
    let lowered = title_html.to_lowercase();
    let id = format!(
        "{}{}",
        settings.prefix,
        INVALID_CHARS.replace_all(&lowered, "")
    );
    let Some(separator) = settings.separator else {
        return id.replace(' ', "");
    };
    // spaces, periods and hyphens (and the separator itself) become one separator per run
    let mut squeezed = String::with_capacity(id.len());
    let mut after_separator = false;
    for c in id.chars() {
        if matches!(c, ' ' | '.' | '-') || c == separator {
            if !after_separator {
                squeezed.push(separator);
            }
            after_separator = true;
        } else {
            squeezed.push(c);
            after_separator = false;
        }
    }
    let mut id = squeezed
        .strip_suffix(separator)
        .unwrap_or(&squeezed)
        .to_string();
    if settings.prefix.is_empty()
        && let Some(stripped) = id.strip_prefix(separator)
    {
        id = stripped.to_string();
    }
    id
}

/// Makes an ID unique among those already taken, as Asciidoctor does: `_title`, then `_title_2`,
/// `_title_3`, ...
pub fn unique_id(id: String, settings: &IdSettings, taken: &HashSet<String>) -> String {
    if !taken.contains(&id) {
        return id;
    }
    let separator = settings.separator.map(String::from).unwrap_or_default();
    (2..)
        .map(|count| format!("{id}{separator}{count}"))
        .find(|candidate| !taken.contains(candidate))
        .unwrap_or(id)
}

/// A title's text as its HTML would have it (text escaped, character references as they are),
/// which is what Asciidoctor generates IDs from: `Wiley & Sons` has `&amp;`, which isn't part of
/// the ID, and the markup of spans and links goes too.
pub fn title_for_id(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::InlineLiteral(literal) => match literal.name {
                InlineLiteralName::Text => text.push_str(
                    &literal
                        .value()
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;"),
                ),
                InlineLiteralName::Charref | InlineLiteralName::Raw => {
                    text.push_str(&literal.value())
                }
            },
            Inline::InlineSpan(span) if span.variant != InlineSpanVariant::Footnote => {
                text.push_str(&title_for_id(&span.inlines))
            }
            Inline::InlineRef(iref) => match iref.variant {
                InlineRefVariant::Image => {}
                _ if iref.inlines.is_empty() => text.push_str(&iref.target),
                _ => text.push_str(&title_for_id(&iref.inlines)),
            },
            _ => {}
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::simple("My Section", "_my_section")]
    #[case::punctuation("Wiley &amp; Sons, Inc.", "_wiley_sons_inc")]
    #[case::typographic_apostrophe("Don&#8217;t Stop", "_dont_stop")]
    #[case::markup("A <strong>bold</strong> move", "_a_bold_move")]
    #[case::hyphens_and_periods("A sub.section with -- dashes", "_a_sub_section_with_dashes")]
    #[case::underscores("snake__case", "_snake_case")]
    #[case::trailing("Ends with a period.", "_ends_with_a_period")]
    #[case::non_ascii("Café Crème", "_café_crème")]
    #[case::numbers_and_periods("Section 1.2 Overview", "_section_1_2_overview")]
    #[case::entities_and_punctuation("Q&amp;A: what's next?", "_qa_whats_next")]
    // Asciidoctor replaces " -- " with a thin space, an em dash and a thin space before making the
    // id, then drops the resulting entities
    #[case::replaced_punctuation("One&#8201;&#8212;&#8201;two", "_onetwo")]
    fn default_ids(#[case] title: &str, #[case] expected: &str) {
        assert_eq!(generate_id(title, &IdSettings::default()), expected);
    }

    #[rstest]
    #[case::github_style("", Some('-'), "My Section", "my-section")]
    #[case::prefix_and_dash("id_", Some('-'), "My Section", "id_my-section")]
    #[case::no_separator("", None, "My Section", "mysection")]
    #[case::empty_prefix_leading_separator("", Some('_'), "-Lead", "lead")]
    fn configured_ids(
        #[case] prefix: &str,
        #[case] separator: Option<char>,
        #[case] title: &str,
        #[case] expected: &str,
    ) {
        let settings = IdSettings {
            enabled: true,
            prefix: prefix.to_string(),
            separator,
        };
        assert_eq!(generate_id(title, &settings), expected);
    }

    #[test]
    fn unique_ids() {
        let settings = IdSettings::default();
        let taken = HashSet::from(["_intro".to_string(), "_intro_2".to_string()]);
        assert_eq!(unique_id("_intro".into(), &settings, &taken), "_intro_3");
        assert_eq!(unique_id("_other".into(), &settings, &taken), "_other");
    }

    #[test]
    fn settings_from_attributes() {
        let attributes = HashMap::from([
            ("idprefix".to_string(), "".to_string()),
            ("idseparator".to_string(), "-".to_string()),
            ("sectids!".to_string(), "".to_string()),
        ]);
        assert_eq!(
            IdSettings::from_attributes(&attributes),
            IdSettings {
                enabled: false,
                prefix: String::new(),
                separator: Some('-'),
            }
        );
    }
}
