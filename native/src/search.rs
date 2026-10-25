mod cache;
mod parsing;

use cache::SearchCache;
pub use parsing::{clean_html, size_bytes, tokens};
use parsing::{fitgirl_row, fitgirl_rss, n, relevant, s, valid_hash};

use crate::model::{SearchResponse, SearchResult, magnet};
use anyhow::{Context, Result, bail};
use futures::{
    future::join_all,
    stream::{FuturesUnordered, StreamExt},
};
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
    cache: Arc<Mutex<SearchCache>>,
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
        if let Some(cached) = self
            .cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&key, Instant::now())
        {
            return cached;
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
            self.cache.lock().unwrap_or_else(|e| e.into_inner()).insert(
                key,
                response.clone(),
                Instant::now(),
            );
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
            results.extend(fitgirl_rss(&rss, &tokens));
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
        let query_tokens = tokens(q);
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
                            if !relevant(&name, &query_tokens) {
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
        let query_tokens = tokens(q);
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
                            || !relevant(&name, &query_tokens)
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
