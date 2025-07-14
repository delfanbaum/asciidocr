use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;

pub static CHARREF_MAP: Lazy<Mutex<HashMap<String, &'static str>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(String::from("&nbsp;"), " ");
    m.insert(String::from("&mdash;"), "—");
    m.insert(String::from("&ndash;"), "—");
    m.insert(String::from("&dollar;"), "$");
    m.insert(String::from("&amp;"), "&");
    m.insert(String::from("&gt;"), ">");
    m.insert(String::from("&lt;"), "<");
    m.insert(String::from("&equals;"), "=");
    m.insert(String::from("&plus;"), "+");
    m.insert(String::from("&#169;"), "©");
    m.insert(String::from("&#174;"), "®");
    m.insert(String::from("&#8212;"), "—");
    m.insert(String::from("&#8482;"), "™");
    m.insert(String::from("&#8230;"), "…");
    m.insert(String::from("&#8594;"), "→");
    m.insert(String::from("&#8658;"), "⇒");
    m.insert(String::from("&#8592;"), "←");
    m.insert(String::from("&#8656;"), "⇐");
    Mutex::new(m)
});
