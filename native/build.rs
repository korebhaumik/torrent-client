use std::{env, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=assets/windows.manifest");
    // Build the Explorer icon from the same PNGs used for the macOS bundle.
    let images = [
        (16u16, "assets/Torrent.iconset/icon_16x16.png"),
        (32, "assets/Torrent.iconset/icon_32x32.png"),
        (64, "assets/Torrent.iconset/icon_32x32@2x.png"),
        (128, "assets/Torrent.iconset/icon_128x128.png"),
        (256, "assets/Torrent.iconset/icon_256x256.png"),
    ];
    for (_, path) in images {
        println!("cargo:rerun-if-changed={path}");
    }
    if env::var("CARGO_CFG_TARGET_OS")? != "windows" {
        return Ok(());
    }

    let output = PathBuf::from(env::var_os("OUT_DIR").ok_or("Missing Cargo output directory")?);
    let mut icon = vec![0, 0, 1, 0];
    icon.extend(u16::try_from(images.len())?.to_le_bytes());
    let mut pixels = Vec::new();
    let mut offset = u32::try_from(6 + images.len() * 16)?;
    for (size, path) in images {
        let png = fs::read(path)?;
        let length = u32::try_from(png.len())?;
        // ICO represents the 256px dimension with zero.
        let dimension = if size == 256 { 0 } else { u8::try_from(size)? };
        icon.extend([dimension, dimension, 0, 0]);
        icon.extend(1u16.to_le_bytes());
        icon.extend(32u16.to_le_bytes());
        icon.extend(length.to_le_bytes());
        icon.extend(offset.to_le_bytes());
        offset += length;
        pixels.extend(png);
    }
    icon.extend(pixels);
    let icon_path = output.join("Torrent.ico");
    fs::write(&icon_path, icon)?;

    winresource::WindowsResource::new()
        .set_icon(icon_path.to_str().ok_or("Cannot encode icon path")?)
        .set("ProductName", "Torrent Desktop")
        .set("FileDescription", "Torrent Desktop")
        .set("OriginalFilename", "Torrent.exe")
        .set_manifest_file("assets/windows.manifest")
        .compile()?;
    Ok(())
}
