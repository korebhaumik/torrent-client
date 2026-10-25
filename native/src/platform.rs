use std::path::Path;

#[cfg(target_os = "macos")]
pub(crate) const REVEAL_LABEL: &str = "Show in Finder";
#[cfg(target_os = "windows")]
pub(crate) const REVEAL_LABEL: &str = "Show in Explorer";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) const REVEAL_LABEL: &str = "Show in folder";

pub(crate) fn reveal_file(path: &Path) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    std::process::Command::new("/usr/bin/open")
        .arg("-R")
        .arg(path)
        .spawn()?;

    #[cfg(target_os = "windows")]
    {
        let mut selection = std::ffi::OsString::from("/select,");
        selection.push(std::path::absolute(path)?);
        // Pass the selection as one native argument, without a command shell,
        // so Unicode, spaces, and shell metacharacters in filenames survive.
        std::process::Command::new("explorer.exe")
            .arg(selection)
            .spawn()?;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    open::that(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Missing parent folder"))?,
    )?;

    Ok(())
}
