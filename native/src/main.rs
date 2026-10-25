#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use torrent_desktop::{app::TorrentApp, config::Paths, search::SearchClient};

fn main() {
    #[cfg(target_os = "windows")]
    if std::env::args().any(|arg| arg == "--search") {
        // Release builds are GUI applications. Attach only for the diagnostic
        // CLI so Explorer launches never create a console window. Redirected
        // handles remain usable even when there is no parent console.
        unsafe {
            windows_sys::Win32::System::Console::AttachConsole(
                windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
            );
        }
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .init();
    if let Err(e) = run() {
        eprintln!("Torrent: {e:#}");
        if !std::env::args().any(|a| a == "--search") {
            rfd::MessageDialog::new()
                .set_title("Torrent Desktop")
                .set_description(format!("{e:#}"))
                .set_level(rfd::MessageLevel::Error)
                .show();
        }
        std::process::exit(1);
    }
}
fn run() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--search") {
        let query = args.get(i + 1).ok_or_else(|| {
            anyhow::anyhow!(
                "Usage: torrent-desktop --search QUERY [--source all|piratebay|yts|tv|fitgirl]"
            )
        })?;
        let source = args
            .iter()
            .position(|a| a == "--source")
            .and_then(|i| args.get(i + 1))
            .map(String::as_str)
            .unwrap_or("all");
        let rt = tokio::runtime::Runtime::new()?;
        let client = SearchClient::new()?;
        let result = rt.block_on(client.search(query, source));
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }
    let paths = Paths::discover()?;
    let lock = paths.lock()?;
    let settings = paths.load_settings()?;
    // Keep framework window/UI persistence isolated with an explicitly selected
    // session. Normal launches retain eframe's existing default storage path.
    let persistence_path =
        std::env::var_os("TORRENT_DATA_DIR").map(|_| paths.data.join("egui.ron"));
    let viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Torrent")
        .with_icon(eframe::icon_data::from_png_bytes(include_bytes!(
            "../assets/logo-preview.png"
        ))?)
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([820.0, 580.0]);
    eframe::run_native(
        "Torrent Desktop",
        eframe::NativeOptions {
            viewport,
            renderer: eframe::Renderer::Glow,
            persistence_path,
            ..Default::default()
        },
        Box::new(move |cc| {
            TorrentApp::new(cc, paths, settings, lock)
                .map(|app| Box::new(app) as Box<dyn eframe::App>)
                .map_err(|e| e.into())
        }),
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(())
}
