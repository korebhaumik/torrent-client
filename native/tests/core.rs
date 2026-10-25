use sha1::{Digest, Sha1};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
use torrent_desktop::{
    config::{Paths, Settings, atomic_json},
    engine::{Command, Engine, import_legacy, parse_magnet, parse_torrent_file},
    model::{self, Snapshot, Torrent},
    search::{clean_html, size_bytes, tokens},
};

fn fixture(name: &str, data: &[u8], multi: bool) -> Vec<u8> {
    let pieces: Vec<u8> = data
        .chunks(16384)
        .flat_map(|chunk| Sha1::digest(chunk).to_vec())
        .collect();
    let mut info = if multi {
        format!("d5:filesld6:lengthi{}e4:pathl8:file.bineee4:name{}:{}12:piece lengthi16384e6:pieces{}:",data.len(),name.len(),name,pieces.len()).into_bytes()
    } else {
        format!(
            "d6:lengthi{}e4:name{}:{}12:piece lengthi16384e6:pieces{}:",
            data.len(),
            name.len(),
            name,
            pieces.len()
        )
        .into_bytes()
    };
    info.extend(pieces);
    info.extend(b"e");
    let mut torrent = b"d4:info".to_vec();
    torrent.extend(info);
    torrent.extend(b"e");
    torrent
}

fn settings(folder: &Path) -> Settings {
    Settings {
        download_dir: folder.into(),
        light: false,
        legacy_imported: true,
    }
}
fn wait(engine: &Engine, description: &str, predicate: impl Fn(&Snapshot) -> bool) {
    let start = Instant::now();
    loop {
        let snapshot = engine.snapshot.lock().unwrap().clone();
        if predicate(&snapshot) {
            return;
        }
        if start.elapsed() > Duration::from_secs(15) {
            panic!("Timed out {description}: {snapshot:?}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn inputs_and_search_normalization() {
    let hash = "0123456789abcdef0123456789abcdef01234567";
    let uri = model::magnet(hash, "A file & spaces");
    let (actual, name, _) = parse_magnet(&uri).unwrap();
    assert_eq!(actual, hash);
    assert_eq!(name, "A file & spaces");
    assert_eq!(parse_magnet(hash).unwrap().0, hash);
    for bad in [
        "",
        "hello",
        "https://example.com",
        "magnet:?xt=urn:btih:bad",
    ] {
        assert!(parse_magnet(bad).is_err());
    }
    assert!(parse_torrent_file(b"not a torrent").is_err());
    assert_eq!(size_bytes("1.5 GiB"), 1610612736);
    assert_eq!(
        clean_html("<script>bad()</script><p>One &amp; two</p><p>Next</p>"),
        "One & two\nNext"
    );
    assert_eq!(tokens("The Ubuntu and Linux"), vec!["ubuntu", "linux"]);
    let t = Torrent {
        hash: hash.into(),
        name: "Ubuntu ISO".into(),
        ..Default::default()
    };
    assert!(t.matches("downloading", "UBUNTU"));
    assert!(!t.matches("seeding", ""));
    assert_eq!(model::bytes(1024.0), "1.0 KB");
    assert_eq!(model::ratio(1, 0), "∞");
}

#[test]
fn atomic_state_and_legacy_import_preserve_original() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(".state.json");
    let original = serde_json::json!({"magnets":[model::magnet(&"a".repeat(40),"Fixture")]});
    atomic_json(&path, &original).unwrap();
    let before = fs::read(&path).unwrap();
    let imports = import_legacy(&path, temp.path().into()).unwrap();
    assert_eq!(imports.len(), 1);
    assert_eq!(fs::read(path).unwrap(), before);
    let paths = Paths {
        data: temp.path().join("app"),
    };
    fs::create_dir_all(&paths.data).unwrap();
    let lock = paths.lock().unwrap();
    assert!(paths.lock().is_err());
    drop(lock);
    assert!(paths.lock().is_ok());
}

#[test]
fn engine_restores_paused_files_and_removes_only_selected_torrent() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
        data: temp.path().join("state"),
    };
    fs::create_dir_all(paths.data.join("metadata")).unwrap();
    let download = temp.path().join("downloads");
    fs::create_dir_all(&download).unwrap();
    let data = vec![42u8; 65536];
    let file = download.join("fixture.bin");
    fs::write(&file, &data).unwrap();
    fs::write(download.join("unrelated.txt"), b"keep me").unwrap();
    let torrent = fixture("fixture.bin", &data, false);
    let (hash, _) = parse_torrent_file(&torrent).unwrap();
    let source = temp.path().join("fixture.torrent");
    fs::write(&source, &torrent).unwrap();
    let options = settings(&download);
    paths.save_settings(&options).unwrap();
    let mut engine = Engine::start(paths.clone(), options.clone());
    engine.send(Command::AddFile(source.clone()));
    wait(&engine, "verify fixture", |s| {
        s.torrents.first().is_some_and(|t| t.done)
    });
    engine.send(Command::Pause(hash.clone()));
    wait(&engine, "pause", |s| {
        s.torrents.first().is_some_and(|t| t.paused)
    });
    engine.shutdown();
    fs::remove_file(source).unwrap();
    let mut restored = Engine::start(paths.clone(), options);
    wait(&restored, "restore paused", |s| {
        s.torrents
            .first()
            .is_some_and(|t| t.paused && !t.files.is_empty())
    });
    restored.send(Command::Resume(hash.clone()));
    wait(&restored, "resume", |s| {
        s.torrents.first().is_some_and(|t| !t.paused && t.done)
    });
    assert_eq!(fs::read(&file).unwrap(), data);
    restored.send(Command::Remove(hash.clone(), false));
    wait(&restored, "remove keeping files", |s| {
        s.ready && s.torrents.is_empty()
    });
    assert!(file.exists());
    let cache = paths.metadata(&hash);
    restored.send(Command::AddFile(cache));
    wait(&restored, "add cached file", |s| {
        s.torrents.first().is_some_and(|t| t.done)
    });
    restored.send(Command::Remove(hash, true));
    wait(&restored, "delete files", |s| {
        s.ready && s.torrents.is_empty()
    });
    assert!(!file.exists());
    assert_eq!(
        fs::read(download.join("unrelated.txt")).unwrap(),
        b"keep me"
    );
    restored.shutdown();
}

#[test]
fn multi_file_layout_matches_webtorrent() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
        data: temp.path().join("state"),
    };
    fs::create_dir_all(paths.data.join("metadata")).unwrap();
    let downloads = temp.path().join("downloads");
    fs::create_dir_all(downloads.join("fixture-pack")).unwrap();
    let data = vec![7u8; 32768];
    let file = downloads.join("fixture-pack/file.bin");
    fs::write(&file, &data).unwrap();
    let source = temp.path().join("multi.torrent");
    fs::write(&source, fixture("fixture-pack", &data, true)).unwrap();
    let options = settings(&downloads);
    paths.save_settings(&options).unwrap();
    let mut engine = Engine::start(paths, options);
    engine.send(Command::AddFile(source));
    wait(&engine, "multi-file verification", |s| {
        s.torrents.first().is_some_and(|t| t.done)
    });
    let snapshot = engine.snapshot.lock().unwrap().clone();
    assert_eq!(snapshot.torrents[0].files[0].path, file);
    assert_eq!(fs::read(file).unwrap(), data);
    engine.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn real_bittorrent_transfer_between_local_peers() {
    use librqbit::{AddTorrent, AddTorrentOptions, Session, SessionOptions};
    let temp = tempfile::tempdir().unwrap();
    let seed = temp.path().join("seed");
    let destination = temp.path().join("destination");
    fs::create_dir_all(&seed).unwrap();
    fs::create_dir_all(&destination).unwrap();
    let data: Vec<u8> = (0..524288).map(|n| (n % 251) as u8).collect();
    fs::write(seed.join("fixture.bin"), &data).unwrap();
    let torrent = fixture("fixture.bin", &data, false);
    let seeder = Session::new_with_opts(
        seed.clone(),
        SessionOptions {
            disable_dht: true,
            disable_dht_persistence: true,
            listen_port_range: Some(43800..43900),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let seed_handle = seeder
        .add_torrent(
            AddTorrent::from_bytes(torrent.clone()),
            Some(AddTorrentOptions {
                overwrite: true,
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_handle()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), seed_handle.wait_until_completed())
        .await
        .unwrap()
        .unwrap();
    let peer = format!("127.0.0.1:{}", seeder.tcp_listen_port().unwrap())
        .parse()
        .unwrap();
    let downloader = Session::new_with_opts(
        destination.clone(),
        SessionOptions {
            disable_dht: true,
            disable_dht_persistence: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let handle = downloader
        .add_torrent(
            AddTorrent::from_bytes(torrent),
            Some(AddTorrentOptions {
                initial_peers: Some(vec![peer]),
                overwrite: true,
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_handle()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(20), handle.wait_until_completed())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fs::read(destination.join("fixture.bin")).unwrap(), data);
    assert!(handle.stats().finished);
    assert_eq!(handle.stats().total_bytes, 524288);
    downloader.stop().await;
    seeder.stop().await;
}
