use crate::{
    config::{Paths, Settings, atomic_json, atomic_write},
    model::*,
};
use anyhow::{Context, Result, bail};
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, Magnet, Session, SessionOptions,
    TorrentStatsState,
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};
use tokio::sync::mpsc as channel;

pub enum Command {
    AddMagnet(String),
    AddFile(PathBuf),
    Pause(String),
    Resume(String),
    Remove(String, bool),
    Import(PathBuf),
    SetFolder(PathBuf),
    Shutdown,
}

pub struct Engine {
    pub snapshot: Arc<Mutex<Snapshot>>,
    pub notices: mpsc::Receiver<Result<String, String>>,
    sender: channel::UnboundedSender<Command>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Engine {
    pub fn start(paths: Paths, settings: Settings) -> Self {
        let (sender, receiver) = channel::unbounded_channel();
        let (notice_tx, notices) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let shared = snapshot.clone();
        let thread = thread::spawn(move || {
            let result = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(anyhow::Error::from)
                .and_then(|rt| {
                    rt.block_on(run(paths, settings, receiver, shared, notice_tx.clone()))
                });
            if let Err(e) = result {
                let _ = notice_tx.send(Err(format!("{e:#}")));
            }
        });
        Self {
            snapshot,
            notices,
            sender,
            thread: Some(thread),
        }
    }
    pub fn send(&self, command: Command) {
        let _ = self.sender.send(command);
    }
    pub fn shutdown(&mut self) {
        self.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn parse_magnet(text: &str) -> Result<(String, String, String)> {
    let parsed =
        Magnet::parse(text.trim()).context("Enter a valid magnet URI or 40 character infohash")?;
    let hash = parsed
        .as_id20()
        .context("This engine requires a BitTorrent v1 infohash")?
        .as_string();
    let name = parsed.name.unwrap_or_else(|| hash.clone());
    let uri = if text.trim().len() == 40 {
        magnet(&hash, &name)
    } else {
        text.trim().to_string()
    };
    Ok((hash, name, uri))
}

pub fn parse_torrent_file(bytes: &[u8]) -> Result<(String, String)> {
    if bytes.len() > 10 * 1024 * 1024 {
        bail!("Torrent files must be smaller than 10 MB");
    }
    let parsed = librqbit::torrent_from_bytes_ext::<librqbit::ByteBuf>(bytes)
        .context("Invalid .torrent file")?;
    let hash = parsed.meta.info_hash.as_string();
    let name = parsed
        .meta
        .info
        .name
        .as_ref()
        .map(|n| String::from_utf8_lossy(n.as_ref()).into_owned())
        .unwrap_or_else(|| hash.clone());
    Ok((hash, name))
}

fn read_saved(paths: &Paths) -> Result<Vec<SavedTorrent>> {
    if !paths.state().exists() {
        return Ok(vec![]);
    }
    serde_json::from_slice(&fs::read(paths.state())?)
        .context("Cannot read torrents.json; original file preserved")
}

pub fn import_legacy(path: &std::path::Path, output: PathBuf) -> Result<Vec<SavedTorrent>> {
    #[derive(serde::Deserialize)]
    struct Legacy {
        magnets: Vec<String>,
    }
    let state: Legacy = serde_json::from_slice(&fs::read(path)?)?;
    state
        .magnets
        .iter()
        .map(|uri| {
            let (hash, name, magnet) = parse_magnet(uri)?;
            Ok(SavedTorrent {
                hash,
                name,
                magnet,
                output: output.clone(),
                output_resolved: false,
                paused: false,
                added_at: chrono::Utc::now().timestamp_millis(),
            })
        })
        .collect()
}

async fn run(
    paths: Paths,
    mut settings: Settings,
    mut commands: channel::UnboundedReceiver<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    notices: mpsc::Sender<Result<String, String>>,
) -> Result<()> {
    fs::create_dir_all(&settings.download_dir).context("Cannot create download folder")?;
    let mut records = read_saved(&paths)?;
    if !settings.legacy_imported {
        let old = settings.download_dir.join(".state.json");
        if old.exists() {
            let imported = import_legacy(&old, settings.download_dir.clone())?;
            for r in imported {
                if !records.iter().any(|v| v.hash == r.hash) {
                    records.push(r);
                }
            }
            atomic_json(&paths.state(), &records)?;
        }
        settings.legacy_imported = true;
        paths.save_settings(&settings)?;
    }
    let session = Session::new_with_opts(
        settings.download_dir.clone(),
        SessionOptions {
            disable_dht_persistence: true,
            listen_port_range: Some(42800..42900),
            enable_upnp_port_forwarding: false,
            ..Default::default()
        },
    )
    .await?;
    let mut tasks: HashMap<String, tokio::task::JoinHandle<()>> = HashMap::new();
    let mut errors: HashMap<String, String> = HashMap::new();
    let (completed_tx, mut completed) = channel::unbounded_channel::<(SavedTorrent, Result<()>)>();
    for r in &records {
        if !r.paused || paths.metadata(&r.hash).exists() {
            tasks.insert(
                r.hash.clone(),
                start_add(
                    session.clone(),
                    r.clone(),
                    paths.clone(),
                    completed_tx.clone(),
                ),
            );
        }
    }
    let mut tick = tokio::time::interval(Duration::from_millis(500));
    loop {
        tokio::select! {
            _ = tick.tick() => publish(&session, &records, &errors, &snapshot),
            Some((record, result)) = completed.recv() => {
                let hash=record.hash.clone();
                tasks.remove(&hash);
                if let Err(e) = result {
                    let message = format!("{e:#}");
                    errors.insert(hash, message.clone());
                    let _ = notices.send(Err(message));
                } else if let Some(saved)=records.iter_mut().find(|r|r.hash==hash) {
                    saved.output=record.output;
                    saved.output_resolved=record.output_resolved;
                    saved.name=record.name;
                    atomic_json(&paths.state(),&records)?;
                }
                publish(&session, &records, &errors, &snapshot);
            }
            command = commands.recv() => {
                let Some(command) = command else { break; };
                if matches!(command, Command::Shutdown) { break; }
                let requested_pause=matches!(command,Command::Pause(_));
                let result: Result<String> = async {
                    match command {
                        Command::AddMagnet(uri) => {
                            let (hash,name,magnet) = parse_magnet(&uri)?;
                            if records.iter().any(|r| r.hash == hash) { bail!("This torrent is already in your library"); }
                            let record = SavedTorrent { hash: hash.clone(), name, magnet, paused: false,
                                added_at: chrono::Utc::now().timestamp_millis(), output: settings.download_dir.clone(), output_resolved:false };
                            records.push(record.clone());
                            if let Err(e) = atomic_json(&paths.state(), &records) { records.pop(); return Err(e); }
                            tasks.insert(hash, start_add(session.clone(), record, paths.clone(), completed_tx.clone()));
                            Ok("Torrent added".into())
                        }
                        Command::AddFile(path) => {
                            if fs::metadata(&path)?.len() > 10 * 1024 * 1024 { bail!("Torrent files must be smaller than 10 MB"); }
                            let data = fs::read(&path)?;
                            let (hash, name) = parse_torrent_file(&data)?;
                            if records.iter().any(|r| r.hash == hash) { bail!("This torrent is already in your library"); }
                            atomic_write(&paths.metadata(&hash), &data)?;
                            let record = SavedTorrent { hash: hash.clone(), magnet: magnet(&hash, &name), name,
                                paused: false, added_at: chrono::Utc::now().timestamp_millis(), output: settings.download_dir.clone(), output_resolved:false };
                            records.push(record.clone());
                            if let Err(e) = atomic_json(&paths.state(), &records) { records.pop(); return Err(e); }
                            tasks.insert(hash, start_add(session.clone(), record, paths.clone(), completed_tx.clone()));
                            Ok("Torrent file added".into())
                        }
                        Command::Pause(hash) | Command::Resume(hash) => {
                            let record = records.iter_mut().find(|r| r.hash == hash).context("Torrent not found")?;
                            let want_pause = requested_pause;
                            let handle = session.with_torrents(|it| it.filter(|(_,h)| h.info_hash().as_string() == hash).map(|(_,h)|h.clone()).next());
                            if want_pause {
                                if let Some(task) = tasks.remove(&hash) { task.abort(); }
                                if let Some(h) = handle { session.pause(&h).await?; }
                                record.paused = true;
                            } else {
                                errors.remove(&hash);
                                record.paused = false;
                                if let Some(h) = handle { session.unpause(&h).await?; }
                                else { tasks.insert(hash, start_add(session.clone(), record.clone(), paths.clone(), completed_tx.clone())); }
                            }
                            atomic_json(&paths.state(), &records)?;
                            Ok(if want_pause { "Torrent paused" } else { "Torrent resumed" }.into())
                        }
                        Command::Remove(hash, delete_files) => {
                            if let Some(task) = tasks.remove(&hash) { task.abort(); }
                            let id = session.with_torrents(|it| it.filter(|(_,h)|h.info_hash().as_string() == hash).map(|(id,_)|id).next());
                            if let Some(id) = id { session.delete(id.into(), delete_files).await?; }
                            records.retain(|r|r.hash != hash);
                            errors.remove(&hash);
                            atomic_json(&paths.state(), &records)?;
                            // Keep the small cached .torrent so a crash cannot lose the source.
                            Ok("Torrent removed".into())
                        }
                        Command::Import(path) => {
                            let output = path.parent().context("Missing download folder")?.to_path_buf();
                            let imported = import_legacy(&path, output)?;
                            let existing: HashSet<_> = records.iter().map(|r| r.hash.clone()).collect();
                            let new: Vec<_> = imported.into_iter().filter(|r|!existing.contains(&r.hash)).collect();
                            let count = new.len();
                            records.extend(new.clone());
                            atomic_json(&paths.state(), &records)?;
                            for r in new { tasks.insert(r.hash.clone(), start_add(session.clone(), r, paths.clone(), completed_tx.clone())); }
                            Ok(format!("Imported {count} torrents"))
                        }
                        Command::SetFolder(path) => {
                            fs::create_dir_all(&path)?;
                            settings = paths.load_settings()?;
                            settings.download_dir = path;
                            paths.save_settings(&settings)?;
                            Ok("Download folder updated for new torrents".into())
                        }
                        Command::Shutdown => unreachable!(),
                    }
                }.await;
                let _ = notices.send(result.map_err(|e|format!("{e:#}")));
                publish(&session, &records, &errors, &snapshot);
            }
        }
    }
    for (_, task) in tasks {
        task.abort();
    }
    atomic_json(&paths.state(), &records)?;
    let _ = tokio::time::timeout(Duration::from_secs(8), session.stop()).await;
    Ok(())
}

fn start_add(
    session: Arc<Session>,
    mut record: SavedTorrent,
    paths: Paths,
    sender: channel::UnboundedSender<(SavedTorrent, Result<()>)>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let result: Result<()> = async {
            let cache = paths.metadata(&record.hash);
            let bytes = if cache.exists() {
                fs::read(cache)?
            } else {
                let response = session
                    .add_torrent(
                        AddTorrent::from_url(record.magnet.clone()),
                        Some(AddTorrentOptions {
                            list_only: true,
                            ..Default::default()
                        }),
                    )
                    .await?;
                match response {
                    AddTorrentResponse::ListOnly(data) => data.torrent_bytes.to_vec(),
                    _ => bail!("Unexpected metadata response"),
                }
            };
            let parsed = librqbit::torrent_from_bytes_ext::<librqbit::ByteBuf>(&bytes)?;
            if !record.output_resolved {
                if parsed.meta.info.files.is_some() {
                    let name = parsed
                        .meta
                        .info
                        .name
                        .as_ref()
                        .context("Multi-file torrent has no name")?;
                    let name = std::str::from_utf8(name.as_ref())?;
                    let mut parts = std::path::Path::new(name).components();
                    if !matches!(parts.next(), Some(std::path::Component::Normal(_)))
                        || parts.next().is_some()
                    {
                        bail!("Unsafe torrent folder name");
                    }
                    record.output = record.output.join(name);
                }
                record.output_resolved = true;
            }
            atomic_write(&paths.metadata(&record.hash), &bytes)?;
            let response = session
                .add_torrent(
                    AddTorrent::from_bytes(bytes),
                    Some(AddTorrentOptions {
                        paused: record.paused,
                        overwrite: true,
                        output_folder: Some(record.output.to_string_lossy().into_owned()),
                        ..Default::default()
                    }),
                )
                .await?;
            let handle = response
                .into_handle()
                .context("No torrent handle returned")?;
            if let Some(metadata) = handle.metadata.load().as_ref() {
                atomic_write(&paths.metadata(&record.hash), &metadata.torrent_bytes)?;
            }
            Ok(())
        }
        .await;
        let _ = sender.send((record, result));
    })
}

fn publish(
    session: &Session,
    records: &[SavedTorrent],
    errors: &HashMap<String, String>,
    shared: &Mutex<Snapshot>,
) {
    let handles: HashMap<_, _> = session.with_torrents(|it| {
        it.map(|(_, h)| (h.info_hash().as_string(), h.clone()))
            .collect()
    });
    let torrents = records
        .iter()
        .map(|r| {
            let mut t = Torrent {
                hash: r.hash.clone(),
                name: r.name.clone(),
                magnet: r.magnet.clone(),
                added_at: r.added_at,
                paused: r.paused,
                error: errors.get(&r.hash).cloned(),
                output: r.output.clone(),
                ..Default::default()
            };
            if let Some(handle) = handles.get(&r.hash) {
                let stats = handle.stats();
                t.name = handle.name().unwrap_or(t.name);
                t.size = stats.total_bytes;
                t.downloaded = stats.progress_bytes;
                t.uploaded = stats.uploaded_bytes;
                t.done = stats.finished;
                t.checking = matches!(stats.state, TorrentStatsState::Initializing);
                t.error = stats.error.or(t.error);
                if let Some(live) = stats.live {
                    t.down_speed = live.download_speed.mbps * 1024.0 * 1024.0;
                    t.up_speed = live.upload_speed.mbps * 1024.0 * 1024.0;
                    t.peers = live.snapshot.peer_stats.live;
                }
                if let Some(metadata) = handle.metadata.load().as_ref() {
                    t.files = metadata
                        .file_infos
                        .iter()
                        .enumerate()
                        .map(|(i, f)| TorrentFile {
                            name: f.relative_filename.to_string_lossy().into_owned(),
                            size: f.len,
                            completed: stats.file_progress.get(i).copied().unwrap_or(0),
                            path: r.output.join(&f.relative_filename),
                        })
                        .collect();
                }
            }
            t
        })
        .collect();
    *shared.lock().unwrap_or_else(|e| e.into_inner()) = Snapshot {
        torrents,
        ready: true,
    };
}
