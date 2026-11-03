🌐 **English** · [Polski](Instalacja)

# Installation

Every release on the [Releases page](https://github.com/michalsarna/baseCreator/releases) holds ready-made packages and archives for each system, plus `SHA256SUMS.txt` to verify the downloads (`sha256sum -c SHA256SUMS.txt`).

| System | Installer / package | Archive |
|---|---|---|
| Windows 10/11 (64-bit) | `basecreator-vX.Y.Z-windows-x86_64-setup.exe` | `basecreator-vX.Y.Z-windows-x86_64.zip` |
| macOS, Apple silicon (M1 and later) | `basecreator-vX.Y.Z-macos-arm64.dmg` | – |
| macOS, Intel | `basecreator-vX.Y.Z-macos-x86_64.dmg` | – |
| Debian / Ubuntu and derivatives, x86_64 | `basecreator-vX.Y.Z-linux-amd64.deb` | `basecreator-vX.Y.Z-linux-x86_64.tar.gz` |
| Debian / Ubuntu and derivatives, ARM64 | `basecreator-vX.Y.Z-linux-arm64.deb` | `basecreator-vX.Y.Z-linux-arm64.tar.gz` |
| Other Linux | – | the `.tar.gz` above |

## Package managers

| System | Command |
|---|---|
| macOS (Homebrew) | `brew install --cask michalsarna/tap/basecreator` (later `brew upgrade`) |
| Windows (Scoop) | `scoop bucket add creator https://github.com/michalsarna/scoop-bucket`, then `scoop install basecreator` (later `scoop update basecreator`) |
| Windows (winget) | `winget install MichalSarna.BaseCreator` once Microsoft has accepted the package into its catalogue (submitted, waiting for review) |

An Arch Linux (AUR) package is prepared but not published yet.

## Windows
Run the `-setup.exe` installer (Start-menu entry and an uninstaller in *Settings → Apps*) or unpack the `.zip` anywhere and start `basecreator.exe`. The installer is not code-signed, so Windows SmartScreen may warn: *More info → Run anyway*.

## macOS
Open the `.dmg` for your Mac (*Apple menu → About This Mac* tells you whether it is Apple silicon or Intel) and drag *BaseCreator* to *Applications*. The app is only ad-hoc signed, not notarized: the first time use **right-click → Open → Open**. If macOS says the app is "damaged", remove the quarantine flag once:

```sh
xattr -dr com.apple.quarantine /Applications/BaseCreator.app
```

## Linux
Install the `.deb` (`sudo apt install ./basecreator-vX.Y.Z-linux-amd64.deb`) or unpack the `.tar.gz` and run `basecreator`. The program needs the usual X11 / Wayland and OpenGL libraries (`libxkbcommon-x11-0`, `libgl1`); the `.deb` lists them.

## From source
```sh
git clone https://github.com/michalsarna/baseCreator
cd baseCreator
cargo run --release -p basecreator
```

## Updates
The program looks for a newer release when it starts (*Settings → Program options…* switches that off), and *Help → Check for updates…* looks at any time.
