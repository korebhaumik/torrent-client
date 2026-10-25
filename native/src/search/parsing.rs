use crate::model::SearchResult;
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

// These expressions are source literals, compiled once rather than for each result.
static HTML_CODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<(script|style)\b[^>]*>.*?</(?:script|style)>")
        .expect("valid HTML code pattern")
});
static HTML_BREAKS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)<br\s*/?>|</(?:p|div|li|h[1-6])>").expect("valid HTML break pattern")
});
static HTML_TAGS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[^>]+>").expect("valid HTML tag pattern"));
static SIZE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*([\d.]+)\s*([a-z]+)\s*$").expect("valid size pattern"));
static MAGNET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)magnet:\?xt=urn:btih:([a-f0-9]{40})[^"'\s<]*"#).expect("valid magnet pattern")
});
static REPACK_SIZE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:repack size|selective download)\s*:?\s*(?:from\s*)?([\d.]+)(?:\s*[/–-]\s*[\d.]+)?\s*(gib|mib|gb|mb)\b").expect("valid repack size pattern")
});
static REPACK_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s*[-–—|]?\s*\[?fitgirl(?:\s+repacks?)?\]?\s*$")
        .expect("valid repack suffix pattern")
});
static RSS_ITEMS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<item>(.*?)</item>").expect("valid RSS item pattern"));
static RSS_TITLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<title(?:\s[^>]*)?>(.*?)</title>").expect("valid RSS title pattern")
});
static RSS_CONTENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<content:encoded(?:\s[^>]*)?>(.*?)</content:encoded>")
        .expect("valid RSS content pattern")
});
static RSS_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<link(?:\s[^>]*)?>(.*?)</link>").expect("valid RSS link pattern")
});

pub(super) fn s(v: &Value) -> String {
    v.as_str().map(str::to_owned).unwrap_or_else(|| {
        if v.is_null() {
            String::new()
        } else {
            v.to_string()
        }
    })
}
pub(super) fn n(v: &Value) -> u64 {
    v.as_u64().unwrap_or_else(|| s(v).parse().unwrap_or(0))
}
pub(super) fn valid_hash(h: &str) -> bool {
    h.len() == 40 && h.chars().all(|c| c.is_ascii_hexdigit())
}
pub fn tokens(q: &str) -> Vec<String> {
    q.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| s.len() >= 2 && !["the", "and", "for"].contains(s))
        .map(str::to_owned)
        .collect()
}
pub(super) fn relevant(name: &str, tokens: &[String]) -> bool {
    let name = name.to_lowercase();
    tokens.iter().any(|t| name.contains(t))
}
pub fn clean_html(html: &str) -> String {
    let no_code = HTML_CODE.replace_all(html, "");
    let breaks = HTML_BREAKS.replace_all(&no_code, "\n");
    let stripped = HTML_TAGS.replace_all(&breaks, " ");
    let decoded = html_escape::decode_html_entities(&stripped);
    decoded
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(3000)
        .collect()
}
pub fn size_bytes(text: &str) -> u64 {
    let Some(c) = SIZE.captures(text) else {
        return 0;
    };
    let value = c[1].parse::<f64>().unwrap_or(0.0);
    let power = match c[2].to_lowercase().as_str() {
        "k" | "kb" | "kib" => 1,
        "m" | "mb" | "mib" => 2,
        "g" | "gb" | "gib" => 3,
        "t" | "tb" | "tib" => 4,
        _ => 0,
    };
    (value * 1024_f64.powi(power)).round() as u64
}
pub(super) fn fitgirl_row(
    title: &str,
    content: &str,
    link: &str,
    tokens: &[String],
) -> Option<SearchResult> {
    let title = clean_html(title);
    if !relevant(&title, tokens) {
        return None;
    }
    let decoded = html_escape::decode_html_entities(content);
    let m = MAGNET.captures(&decoded)?;
    let uri = m[0].to_owned();
    let hash = m[1].to_lowercase();
    let text = clean_html(content);
    let size = REPACK_SIZE
        .captures(&text)
        .map(|c| size_bytes(&format!("{} {}", &c[1], &c[2])))
        .unwrap_or(0);
    let name = REPACK_SUFFIX.replace(&title, "").trim().to_owned();
    Some(SearchResult {
        source: "fitgirl".into(),
        name,
        size,
        hash,
        magnet: uri,
        details_url: link.into(),
        category: "Games".into(),
        description: text,
        ..Default::default()
    })
}

pub(super) fn fitgirl_rss(rss: &str, tokens: &[String]) -> Vec<SearchResult> {
    RSS_ITEMS
        .captures_iter(rss)
        .filter_map(|item| {
            let field = |pattern: &Regex| {
                pattern
                    .captures(&item[1])
                    .map(|c| {
                        c[1].trim()
                            .trim_start_matches("<![CDATA[")
                            .trim_end_matches("]]>")
                            .to_owned()
                    })
                    .unwrap_or_default()
            };
            fitgirl_row(
                &field(&RSS_TITLE),
                &field(&RSS_CONTENT),
                &field(&RSS_LINK),
                tokens,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rss_preserves_cdata_entities_sizes_and_relevance() {
        let hash = "ABCDEF0123456789ABCDEF0123456789ABCDEF01";
        let rss = format!(
            r#"<rss><channel>
            <item><title><![CDATA[Ubuntu &amp; Linux – FitGirl Repack]]></title>
            <link>https://example.org/ubuntu</link>
            <content:encoded><![CDATA[<script>discard()</script><p>Selective Download: from 1.5 / 2 GB</p><a href="magnet:?xt=urn:btih:{hash}&amp;dn=Ubuntu">Magnet</a>]]></content:encoded></item>
            <item><title>Unrelated game</title><content:encoded>magnet:?xt=urn:btih:{hash}</content:encoded></item>
            <item><title>Ubuntu with missing magnet</title></item>
            </channel></rss>"#
        );
        let rows = fitgirl_rss(&rss, &tokens("Ubuntu"));
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.name, "Ubuntu & Linux");
        assert_eq!(row.hash, hash.to_lowercase());
        assert_eq!(row.size, 1_610_612_736);
        assert_eq!(row.magnet, format!("magnet:?xt=urn:btih:{hash}&dn=Ubuntu"));
        assert_eq!(row.details_url, "https://example.org/ubuntu");
        assert!(!row.description.contains("discard"));
    }

    #[test]
    fn normalization_preserves_unicode_and_limits_descriptions() {
        assert_eq!(
            clean_html("<style>body {}</style><p>Café &amp; 日本語<br />Next</p>"),
            "Café & 日本語\nNext"
        );
        assert_eq!(clean_html(&"é".repeat(4000)).chars().count(), 3000);
        for (text, expected) in [(" 2 KiB ", 2048), ("3 mb", 3_145_728), ("bad", 0)] {
            assert_eq!(size_bytes(text), expected);
        }
        assert!(fitgirl_rss("<rss><channel/></rss>", &tokens("Ubuntu")).is_empty());
    }
}
