use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use std::collections::HashMap;
use std::sync::Mutex;

pub static CHARREF_MAP: Lazy<Mutex<HashMap<String, &'static str>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(String::from("&nbsp;"), " ");
    m.insert(String::from("&#160;"), " ");
    m.insert(String::from("&mdash;"), "—");
    m.insert(String::from("&#8212;"), "—");
    m.insert(String::from("&ndash;"), "—");
    m.insert(String::from("&dollar;"), "$");
    m.insert(String::from("&amp;"), "&");
    m.insert(String::from("&#38;"), "&");
    m.insert(String::from("&gt;"), ">");
    m.insert(String::from("&#62;"), ">");
    m.insert(String::from("&lt;"), "<");
    m.insert(String::from("&#60;"), "<");
    m.insert(String::from("&equals;"), "=");
    m.insert(String::from("&plus;"), "+");
    m.insert(String::from("&#169;"), "©");
    m.insert(String::from("&copy;"), "©");
    m.insert(String::from("&#174;"), "®");
    m.insert(String::from("&reg;"), "®");
    m.insert(String::from("&#8482;"), "™");
    m.insert(String::from("&trade;"), "™");
    m.insert(String::from("&#8230;"), "…");
    m.insert(String::from("&#8230;&#8203;"), "…\u{200b}");
    m.insert(String::from("&#8594;"), "→");
    m.insert(String::from("&#8658;"), "⇒");
    m.insert(String::from("&#8592;"), "←");
    m.insert(String::from("&#8656;"), "⇐");
    m.insert(String::from("&#sect;"), "§");
    m.insert(String::from("&#167;"), "§");
    m.insert(String::from("&euro;"), "€");
    m.insert(String::from("&#8364;"), "€");
    m.insert(String::from("&yen;"), "¥");
    m.insert(String::from("&#165;"), "¥");
    m.insert(String::from("&quot;"), "\"");
    m.insert(String::from("&#34;"), "\"");
    m.insert(String::from("&#39;"), "'");
    m.insert(String::from("&apos;"), "'");
    m.insert(String::from("&lsquo;"), "‘");
    m.insert(String::from("&#8216;"), "‘");
    m.insert(String::from("&rsquo;"), "’");
    m.insert(String::from("&#8217;"), "’");
    m.insert(String::from("&ldquo;"), "“");
    m.insert(String::from("&#8220;"), "“");
    m.insert(String::from("&rdquo;"), "”");
    m.insert(String::from("&#8221;"), "”");
    Mutex::new(m)
});

/// Replaces character references with their characters, where they are known
pub(crate) fn decode_references(text: &str) -> String {
    static REFERENCE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"&(?:#([0-9]+)|#[xX]([0-9a-fA-F]+)|[A-Za-z][A-Za-z0-9]*);").unwrap()
    });
    REFERENCE
        .replace_all(text, |found: &Captures| {
            let number = match (found.get(1), found.get(2)) {
                (Some(decimal), _) => decimal.as_str().parse().ok(),
                (_, Some(hex)) => u32::from_str_radix(hex.as_str(), 16).ok(),
                _ => None,
            };
            match number.and_then(char::from_u32) {
                Some(character) => character.to_string(),
                None => CHARREF_MAP
                    .lock()
                    .unwrap()
                    .get(&found[0])
                    .map_or_else(|| found[0].to_string(), |known| known.to_string()),
            }
        })
        .into_owned()
}
