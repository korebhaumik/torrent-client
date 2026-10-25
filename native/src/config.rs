use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub download_dir: PathBuf,
    pub light: bool,
    #[serde(default)]
    pub legacy_imported: bool,
}

#[derive(Clone)]
pub struct Paths {
    pub data: PathBuf,
}

impl Paths {
    pub fn discover() -> Result<Self> {
        let data = match std::env::var_os("TORRENT_DATA_DIR") {
            Some(p) => PathBuf::from(p),
            None => directories::ProjectDirs::from("", "", "Torrent Desktop")
                .context("Cannot locate the application data folder")?
                .data_local_dir()
                .to_owned(),
        };
        fs::create_dir_all(data.join("metadata"))?;
        Ok(Self { data })
    }
    pub fn settings(&self) -> PathBuf {
        self.data.join("settings.json")
    }
    pub fn state(&self) -> PathBuf {
        self.data.join("torrents.json")
    }
    pub fn metadata(&self, hash: &str) -> PathBuf {
        self.data.join("metadata").join(format!("{hash}.torrent"))
    }
    pub fn load_settings(&self) -> Result<Settings> {
        if self.settings().exists() {
            return serde_json::from_slice(&fs::read(self.settings())?)
                .context("Cannot read settings.json; original file preserved");
        }
        let home = directories::UserDirs::new().context("Cannot locate home folder")?;
        let legacy = legacy_downloads();
        let settings = Settings {
            download_dir: legacy.unwrap_or_else(|| {
                home.download_dir()
                    .unwrap_or(home.home_dir())
                    .join("Torrent")
            }),
            light: false,
            legacy_imported: false,
        };
        self.save_settings(&settings)?;
        Ok(settings)
    }
    pub fn save_settings(&self, value: &Settings) -> Result<()> {
        atomic_json(&self.settings(), value)
    }
    pub fn lock(&self) -> Result<File> {
        let f = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.data.join("session.lock"))?;
        fs2::FileExt::try_lock_exclusive(&f)
            .context("Torrent is already running. Open the existing window.")?;
        Ok(f)
    }
}

fn legacy_downloads() -> Option<PathBuf> {
    let mut starts = vec![];
    if let Ok(p) = std::env::current_exe() {
        starts.push(p);
    }
    if let Ok(p) = std::env::current_dir() {
        starts.push(p);
    }
    for start in starts {
        for base in start.ancestors().take(7) {
            let path = base.join("downloads");
            if path.join(".state.json").is_file() && base.join("native/Cargo.toml").is_file() {
                return Some(path);
            }
        }
    }
    None
}

pub fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    atomic_write(path, &serde_json::to_vec_pretty(value)?)
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("Missing parent folder")?;
    fs::create_dir_all(parent)?;
    let temp = path.with_extension("tmp");
    let result = (|| -> Result<()> {
        let mut file = File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)?;
        // Unix can flush the renamed directory entry. Windows cannot open a
        // directory with File::open; the temporary file has already been flushed.
        #[cfg(unix)]
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.with_context(|| format!("Cannot save {}", path.display()))
}
