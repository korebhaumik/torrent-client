Torrent Desktop for Windows (x64)

Extract the entire ZIP to a folder, then open Torrent.exe. No installer,
administrator access, or separate Visual C++ runtime installation is required.
Use Windows 10 or Windows 11 on an x64 PC with an OpenGL 2.0-capable driver.

Add a magnet link or choose/drag a .torrent file into the app.
Ctrl+K: Add torrent    Ctrl+O: Choose file    Ctrl+F: Search
Ctrl+,: Preferences   Ctrl+Enter: Submit    Esc: Close dialog

Downloads default to your Downloads\Torrent folder. Change this in Preferences.
Settings and session metadata live under %LOCALAPPDATA%\Torrent Desktop\data.
The ZIP is portable; settings and downloads are stored separately from the EXE.
Replacing the app does not remove them. Close the app before replacing the EXE.

Windows may ask whether to allow incoming connections for torrent peers.
Choose the networks on which you want to allow peer connections.
This build is unsigned; Windows may display an unknown publisher warning.

The .zip.sha256 file contains the SHA-256 checksum of the ZIP. In PowerShell:
Get-FileHash .\Torrent-*-windows-x64.zip -Algorithm SHA256

For a diagnostic search from PowerShell, capture output and wait for completion:
$p = Start-Process .\Torrent.exe -ArgumentList '--search ubuntu --source yts' -Wait -PassThru -RedirectStandardOutput search.json -RedirectStandardError search-error.txt
Get-Content search.json
The process exit status is available as $p.ExitCode.
