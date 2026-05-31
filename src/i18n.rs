use std::collections::HashMap;
use std::sync::OnceLock;

/// All locale JSON files embedded at compile time.
/// Keys are the BCP 47 / ISO 639-1 language tag (or `x-*` for private-use).
/// `grc` uses the ISO 639-2 code for Ancient Greek.
macro_rules! loc {
    ($code:literal) => {
        ($code, include_str!(concat!("locale/", $code, ".json")))
    };
}

static LOCALES: &[(&str, &str)] = &[
    loc!("aa"),
    loc!("ab"),
    loc!("ae"),
    loc!("af"),
    loc!("ak"),
    loc!("am"),
    loc!("an"),
    loc!("ar"),
    loc!("as"),
    loc!("av"),
    loc!("ay"),
    loc!("az"),
    loc!("ba"),
    loc!("be"),
    loc!("bg"),
    loc!("bi"),
    loc!("bm"),
    loc!("bn"),
    loc!("bo"),
    loc!("br"),
    loc!("bs"),
    loc!("ca"),
    loc!("ce"),
    loc!("ch"),
    loc!("co"),
    loc!("cr"),
    loc!("cs"),
    loc!("cu"),
    loc!("cv"),
    loc!("cy"),
    loc!("da"),
    loc!("de"),
    loc!("dv"),
    loc!("dz"),
    loc!("ee"),
    loc!("el"),
    loc!("en"),
    loc!("eo"),
    loc!("es"),
    loc!("et"),
    loc!("eu"),
    loc!("fa"),
    loc!("ff"),
    loc!("fi"),
    loc!("fj"),
    loc!("fo"),
    loc!("fr"),
    loc!("fy"),
    loc!("ga"),
    loc!("gd"),
    loc!("gl"),
    loc!("gn"),
    loc!("grc"),
    loc!("gu"),
    loc!("gv"),
    loc!("ha"),
    loc!("he"),
    loc!("hi"),
    loc!("hr"),
    loc!("hu"),
    loc!("hy"),
    loc!("hz"),
    loc!("ia"),
    loc!("id"),
    loc!("ie"),
    loc!("ig"),
    loc!("ii"),
    loc!("ik"),
    loc!("io"),
    loc!("is"),
    loc!("it"),
    loc!("iu"),
    loc!("ja"),
    loc!("jv"),
    loc!("ka"),
    loc!("kg"),
    loc!("ki"),
    loc!("kj"),
    loc!("kk"),
    loc!("kl"),
    loc!("km"),
    loc!("kn"),
    loc!("ko"),
    loc!("kr"),
    loc!("ks"),
    loc!("ku"),
    loc!("kv"),
    loc!("kw"),
    loc!("ky"),
    loc!("la"),
    loc!("lb"),
    loc!("lg"),
    loc!("li"),
    loc!("ln"),
    loc!("lo"),
    loc!("lt"),
    loc!("lu"),
    loc!("lv"),
    loc!("mg"),
    loc!("mh"),
    loc!("mi"),
    loc!("ml"),
    loc!("mn"),
    loc!("mr"),
    loc!("ms"),
    loc!("mt"),
    loc!("my"),
    loc!("na"),
    loc!("nb"),
    loc!("nd"),
    loc!("ne"),
    loc!("ng"),
    loc!("nl"),
    loc!("nn"),
    loc!("nr"),
    loc!("nv"),
    loc!("ny"),
    loc!("oc"),
    loc!("oj"),
    loc!("om"),
    loc!("or"),
    loc!("os"),
    loc!("pa"),
    loc!("pi"),
    loc!("pl"),
    loc!("ps"),
    loc!("pt"),
    loc!("qu"),
    loc!("rm"),
    loc!("rn"),
    loc!("ro"),
    loc!("ru"),
    loc!("rw"),
    loc!("sa"),
    loc!("sc"),
    loc!("sd"),
    loc!("se"),
    loc!("sg"),
    loc!("si"),
    loc!("sk"),
    loc!("sl"),
    loc!("sm"),
    loc!("sn"),
    loc!("so"),
    loc!("sq"),
    loc!("sr"),
    loc!("ss"),
    loc!("st"),
    loc!("su"),
    loc!("sv"),
    loc!("sw"),
    loc!("ta"),
    loc!("te"),
    loc!("tg"),
    loc!("th"),
    loc!("ti"),
    loc!("tk"),
    loc!("tl"),
    loc!("tn"),
    loc!("to"),
    loc!("tr"),
    loc!("ts"),
    loc!("tt"),
    loc!("tw"),
    loc!("ty"),
    loc!("ug"),
    loc!("uk"),
    loc!("ur"),
    loc!("uz"),
    loc!("va"),
    loc!("ve"),
    loc!("vi"),
    loc!("vo"),
    loc!("wa"),
    loc!("wo"),
    loc!("xh"),
    loc!("yi"),
    loc!("yo"),
    loc!("za"),
    loc!("zh"),
    loc!("zh-tw"),
    loc!("zu"),
    // ISO 639-2 (not in 639-1)
    loc!("grc"),
    // IETF BCP 47 private-use (x-*) — fictional / constructed languages
    loc!("x-dothraki"),
    loc!("x-klingon"),
    loc!("x-lojban"),
    loc!("x-navii"),
    loc!("x-quenya"),
    loc!("x-sindarin"),
    loc!("x-valyrian"),
];

/// Normalise legacy and alias language tags to our canonical codes.
fn normalise(lang: &str) -> &str {
    // Case-fold to lowercase before comparing; the BCP 47 tags we store are
    // already lowercase, so we only need to handle well-known legacy aliases.
    match lang {
        // Deprecated ISO 639-1 / common aliases
        "gr" => "el", // informal alias for Greek
        "no" => "nb", // Norwegian → Bokmål
        "in" => "id", // old Indonesian code
        "iw" => "he", // old Hebrew code
        "ji" => "yi", // old Yiddish code
        "jw" => "jv", // old Javanese code
        // Simplified-Chinese variants
        "zh-cn" | "zh-hans" | "zh-sg" => "zh",
        // Traditional-Chinese variants
        "zh-hant" | "zh-hk" => "zh-tw",
        other => other,
    }
}

/// Pre-parsed locale maps, initialized once on first access.
static PARSED_LOCALES: OnceLock<HashMap<&'static str, HashMap<String, String>>> = OnceLock::new();

fn parsed_locales() -> &'static HashMap<&'static str, HashMap<String, String>> {
    PARSED_LOCALES.get_or_init(|| {
        let mut map = HashMap::with_capacity(LOCALES.len());
        for &(code, json_str) in LOCALES {
            let mut strings = HashMap::new();
            if let Ok(serde_json::Value::Object(obj)) = serde_json::from_str(json_str) {
                for (k, v) in obj {
                    if k != "_comment" {
                        if let serde_json::Value::String(s) = v {
                            strings.insert(k, s);
                        }
                    }
                }
            }
            map.insert(code, strings);
        }
        map
    })
}

/// Internationalization lookup table for UI strings, loaded from embedded JSON locale files.
pub struct I18n {
    strings: &'static HashMap<String, String>,
}

impl I18n {
    /// Create an `I18n` instance for the given BCP 47 language tag.
    ///
    /// Resolution order:
    /// 1. Exact match after alias normalisation (e.g. `"el"`, `"x-klingon"`)
    /// 2. Base subtag (e.g. `"pt-BR"` → `"pt"`)
    /// 3. English fallback
    pub fn new(language: &str) -> Self {
        let lang = normalise(language);
        let locales = parsed_locales();

        let strings = locales
            .get(lang)
            .or_else(|| {
                // strip subtag: "pt-BR" → "pt"
                let base = lang.split('-').next().unwrap_or(lang);
                if base != lang {
                    locales.get(base)
                } else {
                    None
                }
            })
            .or_else(|| locales.get("en"))
            .expect("English locale must exist");

        Self { strings }
    }

    /// Return the localised string for `key`, or `key` itself if not found.
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.strings.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// Return an iterator over all known language codes (for tooling / docs).
    pub fn known_languages() -> impl Iterator<Item = &'static str> {
        LOCALES.iter().map(|(code, _)| *code)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const REQUIRED_KEYS: &[&str] = &[
        "From",
        "Date",
        "Subject",
        "Message-ID",
        "References",
        "In-Reply-To",
        "Attachment",
        "Author",
        "Next",
        "Previous",
        "Index",
        "Thread",
        "Date Index",
        "Subject Index",
        "Author Index",
        "Thread Index",
        "Starting",
        "Ending",
        "Last message date",
        "Archived on",
        "messages sorted by",
        "About this archive",
        "search",
        "next",
        "previous",
        "no subject",
        "unknown author",
        "Article",
        "Deleted message",
        "Expired message",
        "[Deleted]",
        "[Expired]",
        "Folders",
        "Generated by",
        "No messages found.",
        "Sun",
        "Mon",
        "Tue",
        "Wed",
        "Thu",
        "Fri",
        "Sat",
        "Jan",
        "Feb",
        "Mar",
        "Apr",
        "May",
        "Jun",
        "Jul",
        "Aug",
        "Sep",
        "Oct",
        "Nov",
        "Dec",
    ];

    // ── English sanity ────────────────────────────────────────────────────

    #[test]
    fn english_basic() {
        let i = I18n::new("en");
        assert_eq!(i.get("From"), "From:");
        assert_eq!(i.get("Subject"), "Subject:");
        assert_eq!(i.get("Index"), "Index");
    }

    // ── Unknown key falls back to the key itself ──────────────────────────

    #[test]
    fn unknown_key_returns_key() {
        let i = I18n::new("en");
        assert_eq!(i.get("NoSuchKey"), "NoSuchKey");
    }

    // ── Unknown language falls back to English ────────────────────────────

    #[test]
    fn unknown_language_falls_back_to_english() {
        let i = I18n::new("xx-unknown");
        assert_eq!(i.get("From"), "From:");
    }

    // ── Aliases ───────────────────────────────────────────────────────────

    #[test]
    fn alias_gr_resolves_to_el() {
        let gr = I18n::new("gr");
        let el = I18n::new("el");
        assert_eq!(gr.get("From"), el.get("From"));
        assert_eq!(gr.get("unknown author"), el.get("unknown author"));
    }

    #[test]
    fn alias_no_resolves_to_nb() {
        let no = I18n::new("no");
        let nb = I18n::new("nb");
        assert_eq!(no.get("From"), nb.get("From"));
    }

    #[test]
    fn alias_in_resolves_to_id() {
        let i_in = I18n::new("in");
        let i_id = I18n::new("id");
        assert_eq!(i_in.get("From"), i_id.get("From"));
    }

    #[test]
    fn alias_iw_resolves_to_he() {
        let iw = I18n::new("iw");
        let he = I18n::new("he");
        assert_eq!(iw.get("From"), he.get("From"));
    }

    #[test]
    fn alias_zh_cn_resolves_to_zh() {
        let cn = I18n::new("zh-cn");
        let zh = I18n::new("zh");
        assert_eq!(cn.get("From"), zh.get("From"));
    }

    #[test]
    fn alias_zh_hk_resolves_to_zh_tw() {
        let hk = I18n::new("zh-hk");
        let tw = I18n::new("zh-tw");
        assert_eq!(hk.get("From"), tw.get("From"));
    }

    // ── Subtag stripping ──────────────────────────────────────────────────

    #[test]
    fn subtag_pt_br_resolves_to_pt() {
        let br = I18n::new("pt-BR");
        let pt = I18n::new("pt");
        assert_eq!(br.get("From"), pt.get("From"));
    }

    // ── Non-English languages are actually different from English ─────────

    #[test]
    fn greek_not_english() {
        let el = I18n::new("el");
        assert_ne!(el.get("From"), "From:");
        assert_ne!(el.get("unknown author"), "Unknown");
    }

    #[test]
    fn german_not_english() {
        let de = I18n::new("de");
        assert_ne!(de.get("From"), "From:");
    }

    // ── Ancient Greek ─────────────────────────────────────────────────────

    #[test]
    fn ancient_greek_loads() {
        let grc = I18n::new("grc");
        assert_ne!(grc.get("From"), "From:");
        // Must not fall back to English
        assert_ne!(grc.get("From"), I18n::new("en").get("From"));
    }

    // ── x-* fictional languages load and differ from English ─────────────

    #[test]
    fn x_klingon_loads() {
        let tlh = I18n::new("x-klingon");
        assert_ne!(tlh.get("search"), "Search");
    }

    #[test]
    fn x_quenya_loads() {
        let q = I18n::new("x-quenya");
        assert_ne!(q.get("Index"), "");
    }

    #[test]
    fn x_lojban_loads() {
        let jbo = I18n::new("x-lojban");
        assert_ne!(jbo.get("From"), "From:");
    }

    // ── Every locale: all 27 required keys present, no empty value ────────

    #[test]
    fn all_locales_have_required_keys() {
        let mut failures: Vec<String> = Vec::new();

        for (code, json_str) in LOCALES {
            let val: serde_json::Value = serde_json::from_str(json_str)
                .unwrap_or_else(|e| panic!("JSON parse error in {code}: {e}"));
            let obj =
                val.as_object().unwrap_or_else(|| panic!("{code}: root is not a JSON object"));

            // _comment must be present
            if !obj.contains_key("_comment") {
                failures.push(format!("{code}: missing _comment"));
            }

            for key in REQUIRED_KEYS {
                match obj.get(*key) {
                    None => failures.push(format!("{code}: missing key [{key}]")),
                    Some(serde_json::Value::String(s)) if s.is_empty() => {
                        failures.push(format!("{code}: empty value for [{key}]"))
                    },
                    Some(serde_json::Value::String(_)) => {},
                    Some(other) => {
                        failures.push(format!("{code}: [{key}] is not a string: {other}"))
                    },
                }
            }
        }

        if !failures.is_empty() {
            panic!("Locale validation failures:\n{}", failures.join("\n"));
        }
    }

    // ── I18n::new round-trip: every locale resolves without panicking ─────

    #[test]
    fn all_locales_load_via_new() {
        for (code, _) in LOCALES {
            let i = I18n::new(code);
            // At minimum the key must not be empty
            assert!(!i.get("From").is_empty(), "{code}: get(From) returned empty");
            assert!(!i.get("Index").is_empty(), "{code}: get(Index) returned empty");
        }
    }

    // ── known_languages() covers expected codes ───────────────────────────

    #[test]
    fn known_languages_includes_expected_codes() {
        let langs: Vec<&str> = I18n::known_languages().collect();
        for expected in &[
            "en",
            "de",
            "fr",
            "ja",
            "zh",
            "ar",
            "el",
            "grc",
            "x-klingon",
            "x-quenya",
            "x-lojban",
        ] {
            assert!(langs.contains(expected), "known_languages missing: {expected}");
        }
    }
}
