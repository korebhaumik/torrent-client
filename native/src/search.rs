use crate::model::{SearchResponse, SearchResult, magnet};
use anyhow::{Context, Result, bail};
use futures::{
    future::join_all,
    stream::{FuturesUnordered, StreamExt},
};
use regex::Regex;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct SearchClient {
    client: reqwest::Client,
    base: Option<String>,
    cache: Arc<Mutex<HashMap<String, (Instant, SearchResponse)>>>,
}

impl SearchClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder().timeout(Duration::from_secs(10))
                .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/124.0.0.0 Safari/537.36")
                .build()?, base:std::env::var("TORRENT_SEARCH_BASE").ok(), cache:Default::default(),
        })
    }
    fn endpoint(&self, source: &str, production: &str, path: &str) -> String {
        match &self.base {
            Some(base) => format!("{}/{source}{path}", base.trim_end_matches('/')),
            None => format!("{production}{path}"),
        }
    }
    async fn text(&self, url: String, query: &[(&str, &str)]) -> Result<String> {
        let mut res = self
            .client
            .get(&url)
            .query(query)
            .send()
            .await?
            .error_for_status()?;
        let mut data = Vec::new();
        while let Some(chunk) = res.chunk().await? {
            if data.len() + chunk.len() > 8 * 1024 * 1024 {
                bail!("Search response exceeds 8 MB");
            }
            data.extend_from_slice(&chunk);
        }
        String::from_utf8(data).context("Search returned invalid text")
    }
    async fn json(&self, url: String, query: &[(&str, &str)]) -> Result<Value> {
        serde_json::from_str(&self.text(url, query).await?)
            .context("Search provider returned invalid JSON")
    }
    pub async fn search(&self, query: &str, source: &str) -> SearchResponse {
        let query = query.trim();
        if query.len() < 2 {
            return SearchResponse::default();
        }
        let key = format!("{source}:{}", query.to_lowercase());
        if let Some((time, cached)) = self.cache.lock().unwrap().get(&key)
            && time.elapsed() < Duration::from_secs(300)
        {
            return cached.clone();
        }
        let sources = if source == "all" {
            vec!["piratebay", "fitgirl", "yts", "tv"]
        } else {
            vec![source]
        };
        let responses = join_all(sources.iter().map(|source| async move {
            let result = tokio::time::timeout(Duration::from_secs(35), async {
                match *source {
                    "piratebay" => self.piratebay(query).await,
                    "fitgirl" => self.fitgirl(query).await,
                    "yts" => self.yts(query).await,
                    "tv" => self.tv(query).await,
                    _ => anyhow::bail!("Unknown search source"),
                }
            })
            .await;
            (*source, result.map_err(anyhow::Error::from).and_then(|r| r))
        }))
        .await;
        let mut response = SearchResponse::default();
        for (source, result) in responses {
            match result {
                Ok(mut rows) => response.results.append(&mut rows),
                Err(e) => response
                    .warnings
                    .push(format!("{}: {e}", source_name(source))),
            }
        }
        response
            .results
            .sort_by_key(|a| std::cmp::Reverse(a.seeders));
        if response.warnings.is_empty() {
            self.cache
                .lock()
                .unwrap()
                .insert(key, (Instant::now(), response.clone()));
        }
        response
    }
    async fn piratebay(&self, q: &str) -> Result<Vec<SearchResult>> {
        let data = self
            .json(
                self.endpoint("piratebay", "https://apibay.org", "/q.php"),
                &[("q", q), ("cat", "0")],
            )
            .await?;
        let items = data
            .as_array()
            .context("Pirate Bay returned an unexpected response")?;
        let mut results = vec![];
        for item in items {
            let name = s(&item["name"]);
            let hash = s(&item["info_hash"]).to_lowercase();
            if s(&item["id"]) == "0" || name == "No results returned" || !valid_hash(&hash) {
                continue;
            }
            let category = match n(&item["category"]) {
                100..200 => "Audio",
                200..300 => "Video",
                300..400 => "Applications",
                400..500 => "Games",
                500..600 => "Porn",
                _ => "Other",
            };
            results.push(SearchResult {
                source: "piratebay".into(),
                magnet: magnet(&hash, &name),
                name,
                hash,
                size: n(&item["size"]),
                seeders: n(&item["seeders"]),
                leechers: n(&item["leechers"]),
                details_url: format!(
                    "https://thepiratebay.org/description.php?id={}",
                    s(&item["id"])
                ),
                category: category.into(),
                ..Default::default()
            });
        }
        Ok(results)
    }
    async fn fitgirl(&self, q: &str) -> Result<Vec<SearchResult>> {
        let tokens = tokens(q);
        if tokens.is_empty() {
            return Ok(vec![]);
        }
        let url = self.endpoint(
            "fitgirl",
            "https://fitgirl-repacks.site",
            "/wp-json/wp/v2/posts",
        );
        let mut results = vec![];
        let mut rest_error = None;
        match self
            .json(
                url,
                &[
                    ("search", q),
                    ("per_page", "50"),
                    ("_fields", "id,link,title,content,modified"),
                ],
            )
            .await
        {
            Ok(data) => {
                if let Some(items) = data.as_array() {
                    for post in items {
                        if let Some(row) = fitgirl_row(
                            &s(&post["title"]["rendered"]),
                            &s(&post["content"]["rendered"]),
                            &s(&post["link"]),
                            &tokens,
                        ) {
                            results.push(row);
                        }
                    }
                } else {
                    rest_error = Some("Unexpected REST response".to_owned());
                }
            }
            Err(e) => rest_error = Some(e.to_string()),
        }
        if results.is_empty() {
            let rss = self
                .text(
                    self.endpoint("fitgirl", "https://fitgirl-repacks.site", "/feed/"),
                    &[("s", q)],
                )
                .await
                .with_context(|| {
                    format!(
                        "REST and RSS unavailable ({})",
                        rest_error.unwrap_or_default()
                    )
                })?;
            if !rss.contains("<rss") && !rss.contains("<feed") {
                bail!("FitGirl returned an invalid RSS response");
            }
            let item_re = Regex::new(r"(?is)<item>(.*?)</item>").unwrap();
            for item in item_re.captures_iter(&rss) {
                let get = |tag: &str| -> String {
                    let re =
                        Regex::new(&format!(r"(?is)<{tag}(?:\s[^>]*)?>(.*?)</{tag}>")).unwrap();
                    re.captures(&item[1])
                        .map(|c| {
                            c[1].trim()
                                .trim_start_matches("<![CDATA[")
                                .trim_end_matches("]]>")
                                .to_owned()
                        })
                        .unwrap_or_default()
                };
                let title = get("title");
                let content = get("content:encoded");
                let link = get("link");
                if let Some(row) = fitgirl_row(&title, &content, &link, &tokens) {
                    results.push(row);
                }
            }
        }
        if self.base.is_none() {
            let swarm = scrape(&results.iter().map(|r| r.hash.clone()).collect::<Vec<_>>()).await;
            for r in &mut results {
                if let Some((seed, leech)) = swarm.get(&r.hash) {
                    r.seeders = *seed;
                    r.leechers = *leech;
                }
            }
        }
        Ok(results)
    }
    async fn yts(&self, q: &str) -> Result<Vec<SearchResult>> {
        let mut last_error = anyhow::anyhow!("All YTS mirrors failed");
        for base in ["https://yts.lt", "https://yts.mx"] {
            match self
                .json(
                    self.endpoint("yts", base, "/api/v2/list_movies.json"),
                    &[
                        ("query_term", q),
                        ("sort_by", "like_count"),
                        ("order_by", "desc"),
                        ("limit", "20"),
                    ],
                )
                .await
            {
                Err(e) => last_error = e,
                Ok(data) => {
                    if data["status"] == "error" {
                        last_error = anyhow::anyhow!(s(&data["status_message"]));
                        continue;
                    }
                    let mut results = vec![];
                    let Some(movies) = data["data"]["movies"].as_array() else {
                        if data["data"]["movie_count"].as_u64() == Some(0) {
                            return Ok(vec![]);
                        }
                        last_error = anyhow::anyhow!("YTS returned an unexpected response");
                        continue;
                    };
                    for movie in movies {
                        let Some(torrents) = movie["torrents"].as_array() else {
                            continue;
                        };
                        for t in torrents {
                            let hash = s(&t["hash"]).to_lowercase();
                            if !valid_hash(&hash) {
                                continue;
                            }
                            let name = format!(
                                "{} ({}) [{} {}]",
                                s(&movie["title"]),
                                n(&movie["year"]),
                                s(&t["quality"]),
                                s(&t["type"])
                            );
                            if !relevant(&name, &tokens(q)) {
                                continue;
                            }
                            results.push(SearchResult {
                                source: "yts".into(),
                                magnet: magnet(&hash, &name),
                                name,
                                hash,
                                size: n(&t["size_bytes"]),
                                seeders: n(&t["seeds"]),
                                leechers: n(&t["peers"]),
                                details_url: format!("{base}/movies/{}", s(&movie["slug"])),
                                category: "Movies".into(),
                                description: clean_html(&s(&movie["summary"])),
                            });
                        }
                    }
                    return Ok(results);
                }
            }
            if self.base.is_some() {
                break;
            }
        }
        Err(last_error)
    }
    async fn tv(&self, q: &str) -> Result<Vec<SearchResult>> {
        let shows = self
            .json(
                self.endpoint("tvmaze", "https://api.tvmaze.com", "/search/shows"),
                &[("q", q)],
            )
            .await?;
        let shows = shows
            .as_array()
            .context("TVMaze returned an unexpected response")?;
        let ids: Vec<String> = shows
            .iter()
            .filter_map(|show| show["show"]["externals"]["imdb"].as_str())
            .map(|id| id.trim_start_matches("tt").to_owned())
            .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
            .take(5)
            .collect();
        let responses = join_all(ids.iter().map(|id| async move {
            let mut last_error = anyhow::anyhow!("All TV mirrors failed");
            for base in [
                "https://eztv.it",
                "https://eztv.ch",
                "https://eztv.li",
                "https://eztv.wf",
                "https://eztv.re",
            ] {
                let params = [("imdb_id", id.as_str()), ("limit", "50"), ("page", "1")];
                let request = self.json(self.endpoint("eztv", base, "/api/get-torrents"), &params);
                match tokio::time::timeout(Duration::from_secs(5), request).await {
                    Ok(Ok(data)) if data["torrents"].is_array() => {
                        return Ok(data["torrents"].as_array().unwrap().clone());
                    }
                    Ok(Err(e)) => last_error = e,
                    _ => {
                        last_error =
                            anyhow::anyhow!("TV mirror unavailable or returned an invalid response")
                    }
                }
                if self.base.is_some() {
                    break;
                }
            }
            Err(last_error)
        }))
        .await;
        let mut results = vec![];
        let mut seen = HashSet::new();
        let mut failure = None;
        for response in responses {
            match response {
                Err(e) => failure = Some(e),
                Ok(items) => {
                    for t in items {
                        let hash = s(&t["hash"]).to_lowercase();
                        let name = s(&t["filename"]);
                        if !valid_hash(&hash)
                            || !seen.insert(hash.clone())
                            || !relevant(&name, &tokens(q))
                        {
                            continue;
                        }
                        let uri = s(&t["magnet_url"]);
                        results.push(SearchResult {
                            source: "tv".into(),
                            magnet: if uri.is_empty() {
                                magnet(&hash, &name)
                            } else {
                                uri
                            },
                            name,
                            hash,
                            size: n(&t["size_bytes"]),
                            seeders: n(&t["seeds"]),
                            leechers: n(&t["peers"]),
                            details_url: s(&t["episode_url"]),
                            category: "TV".into(),
                            ..Default::default()
                        });
                    }
                }
            }
        }
        if results.is_empty()
            && let Some(e) = failure
        {
            return Err(e);
        }
        Ok(results)
    }
}

pub fn source_name(source: &str) -> &str {
    match source {
        "piratebay" => "Pirate Bay",
        "fitgirl" => "FitGirl",
        "yts" => "YTS",
        "tv" => "TV Shows",
        _ => "All",
    }
}
fn s(v: &Value) -> String {
    v.as_str().map(str::to_owned).unwrap_or_else(|| {
        if v.is_null() {
            String::new()
        } else {
            v.to_string()
        }
    })
}
fn n(v: &Value) -> u64 {
    v.as_u64().unwrap_or_else(|| s(v).parse().unwrap_or(0))
}
fn valid_hash(h: &str) -> bool {
    h.len() == 40 && h.chars().all(|c| c.is_ascii_hexdigit())
}
pub fn tokens(q: &str) -> Vec<String> {
    q.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| s.len() >= 2 && !["the", "and", "for"].contains(s))
        .map(str::to_owned)
        .collect()
}
fn relevant(name: &str, tokens: &[String]) -> bool {
    let name = name.to_lowercase();
    tokens.iter().any(|t| name.contains(t))
}
pub fn clean_html(html: &str) -> String {
    let no_code = Regex::new(r"(?is)<(script|style)\b[^>]*>.*?</(?:script|style)>")
        .unwrap()
        .replace_all(html, "");
    let breaks = Regex::new(r"(?i)<br\s*/?>|</(?:p|div|li|h[1-6])>")
        .unwrap()
        .replace_all(&no_code, "\n");
    let stripped = Regex::new(r"<[^>]+>").unwrap().replace_all(&breaks, " ");
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
    let re = Regex::new(r"(?i)^\s*([\d.]+)\s*([a-z]+)\s*$").unwrap();
    let Some(c) = re.captures(text) else {
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
fn fitgirl_row(title: &str, content: &str, link: &str, tokens: &[String]) -> Option<SearchResult> {
    let title = clean_html(title);
    if !relevant(&title, tokens) {
        return None;
    }
    let decoded = html_escape::decode_html_entities(content);
    let re = Regex::new(r#"(?i)magnet:\?xt=urn:btih:([a-f0-9]{40})[^"'\s<]*"#).unwrap();
    let m = re.captures(&decoded)?;
    let uri = m[0].to_owned();
    let hash = m[1].to_lowercase();
    let text = clean_html(content);
    let size_re=Regex::new(r"(?i)(?:repack size|selective download)\s*:?\s*(?:from\s*)?([\d.]+)(?:\s*[/–-]\s*[\d.]+)?\s*(gib|mib|gb|mb)\b").unwrap();
    let size = size_re
        .captures(&text)
        .map(|c| size_bytes(&format!("{} {}", &c[1], &c[2])))
        .unwrap_or(0);
    let name = Regex::new(r"(?i)\s*[-–—|]?\s*\[?fitgirl(?:\s+repacks?)?\]?\s*$")
        .unwrap()
        .replace(&title, "")
        .trim()
        .to_owned();
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

async fn scrape(hashes: &[String]) -> HashMap<String, (u64, u64)> {
    if hashes.is_empty() {
        return HashMap::new();
    }
    let valid: Vec<_> = hashes
        .iter()
        .filter(|h| valid_hash(h))
        .take(60)
        .cloned()
        .collect();
    let mut requests = FuturesUnordered::new();
    for endpoint in ["tracker.opentrackr.org:1337", "tracker.torrent.eu.org:451"] {
        requests.push(tokio::time::timeout(
            Duration::from_secs(4),
            scrape_tracker(endpoint, &valid),
        ));
    }
    while let Some(result) = requests.next().await {
        if let Ok(Ok(stats)) = result {
            return stats;
        }
    }
    HashMap::new()
}
async fn scrape_tracker(endpoint: &str, hashes: &[String]) -> Result<HashMap<String, (u64, u64)>> {
    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    let address = tokio::net::lookup_host(endpoint)
        .await?
        .find(|a| a.is_ipv4())
        .context("No IPv4 tracker address")?;
    socket.connect(address).await?;
    let tx: u32 = rand::random();
    let mut packet = 0x0000041727101980u64.to_be_bytes().to_vec();
    packet.extend_from_slice(&0u32.to_be_bytes());
    packet.extend_from_slice(&tx.to_be_bytes());
    socket.send(&packet).await?;
    let mut response = [0u8; 2048];
    let len = socket.recv(&mut response).await?;
    if len < 16 || response[..4] != 0u32.to_be_bytes() || response[4..8] != tx.to_be_bytes() {
        bail!("Invalid tracker handshake");
    }
    let tx: u32 = rand::random();
    let mut packet = response[8..16].to_vec();
    packet.extend_from_slice(&2u32.to_be_bytes());
    packet.extend_from_slice(&tx.to_be_bytes());
    for h in hashes {
        packet.extend_from_slice(&hex::decode(h)?);
    }
    socket.send(&packet).await?;
    let len = socket.recv(&mut response).await?;
    if len < 8 + 12 * hashes.len()
        || response[..4] != 2u32.to_be_bytes()
        || response[4..8] != tx.to_be_bytes()
    {
        bail!("Invalid tracker scrape response");
    }
    Ok(hashes
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let offset = 8 + i * 12;
            (
                h.clone(),
                (
                    u32::from_be_bytes(response[offset..offset + 4].try_into().unwrap()) as u64,
                    u32::from_be_bytes(response[offset + 8..offset + 12].try_into().unwrap())
                        as u64,
                ),
            )
        })
        .collect())
}
