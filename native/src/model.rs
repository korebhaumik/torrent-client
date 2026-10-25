use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const TRACKERS: &[&str] = &[
    "udp://tracker.opentrackr.org:1337",
    "udp://open.demonii.com:1337",
    "udp://tracker.openbittorrent.com:6969",
    "udp://exodus.desync.com:6969",
];

pub fn magnet(hash: &str, name: &str) -> String {
    let mut url = url::Url::parse("magnet:?").unwrap();
    url.query_pairs_mut()
        .append_pair("xt", &format!("urn:btih:{}", hash.to_lowercase()))
        .append_pair("dn", name);
    for tracker in TRACKERS {
        url.query_pairs_mut().append_pair("tr", tracker);
    }
    url.to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedTorrent {
    pub hash: String,
    pub name: String,
    pub magnet: String,
    pub added_at: i64,
    pub paused: bool,
    pub output: PathBuf,
    #[serde(default)]
    pub output_resolved: bool,
}

#[derive(Clone, Debug, Default)]
pub struct TorrentFile {
    pub name: String,
    pub size: u64,
    pub completed: u64,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct Torrent {
    pub hash: String,
    pub name: String,
    pub magnet: String,
    pub added_at: i64,
    pub size: u64,
    pub downloaded: u64,
    pub uploaded: u64,
    pub down_speed: f64,
    pub up_speed: f64,
    pub peers: usize,
    pub paused: bool,
    pub done: bool,
    pub checking: bool,
    pub error: Option<String>,
    pub files: Vec<TorrentFile>,
    pub output: PathBuf,
}

impl Torrent {
    pub fn progress(&self) -> f32 {
        if self.size == 0 {
            0.0
        } else {
            (self.downloaded as f64 / self.size as f64).clamp(0.0, 1.0) as f32
        }
    }
    pub fn status(&self) -> &'static str {
        if self.error.is_some() {
            "error"
        } else if self.paused {
            "paused"
        } else if self.checking {
            "checking"
        } else if self.done {
            "seeding"
        } else if self.size == 0 {
            "metadata"
        } else {
            "downloading"
        }
    }
    pub fn eta(&self) -> String {
        if self.paused || self.down_speed <= 0.0 || self.done {
            return "—".into();
        }
        let secs = ((self.size.saturating_sub(self.downloaded)) as f64 / self.down_speed) as u64;
        match secs {
            0..60 => format!("{secs}s"),
            60..3600 => format!("{}m", secs / 60),
            3600..172800 => format!("{}h {}m", secs / 3600, secs / 60 % 60),
            _ => format!("{}d {}h", secs / 86400, secs / 3600 % 24),
        }
    }
    pub fn matches(&self, filter: &str, query: &str) -> bool {
        let matches = match filter {
            "downloading" => matches!(
                self.status(),
                "downloading" | "metadata" | "checking" | "error"
            ),
            "seeding" => self.status() == "seeding",
            "paused" => self.paused,
            _ => true,
        };
        let q = query.trim().to_lowercase();
        matches
            && (q.is_empty()
                || self.name.to_lowercase().contains(&q)
                || self.hash.to_lowercase().contains(&q))
    }
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub torrents: Vec<Torrent>,
    pub ready: bool,
}

impl Snapshot {
    pub fn down(&self) -> f64 {
        self.torrents.iter().map(|t| t.down_speed).sum()
    }
    pub fn up(&self) -> f64 {
        self.torrents.iter().map(|t| t.up_speed).sum()
    }
    pub fn downloaded(&self) -> u64 {
        self.torrents.iter().map(|t| t.downloaded).sum()
    }
    pub fn uploaded(&self) -> u64 {
        self.torrents.iter().map(|t| t.uploaded).sum()
    }
    pub fn active(&self) -> usize {
        self.torrents
            .iter()
            .filter(|t| !t.paused && t.error.is_none())
            .count()
    }
}

pub fn bytes(n: impl Into<f64>) -> String {
    let n = n.into();
    if !n.is_finite() || n <= 0.0 {
        return "0 B".into();
    }
    let i = ((n.ln() / 1024_f64.ln()).floor() as usize).min(4);
    let value = n / 1024_f64.powi(i as i32);
    if i == 0 {
        format!("{value:.0} B")
    } else {
        format!("{value:.1} {}", ["B", "KB", "MB", "GB", "TB"][i])
    }
}
pub fn speed(n: f64) -> String {
    format!("{}/s", bytes(n))
}
pub fn ratio(up: u64, down: u64) -> String {
    if down == 0 {
        if up == 0 { "0.00".into() } else { "∞".into() }
    } else {
        format!("{:.2}", up as f64 / down as f64)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchResult {
    pub source: String,
    pub name: String,
    pub size: u64,
    pub seeders: u64,
    pub leechers: u64,
    pub hash: String,
    pub magnet: String,
    pub details_url: String,
    pub category: String,
    pub description: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub warnings: Vec<String>,
}
