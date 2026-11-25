🌐 [English](Installation) · [Polski](Instalacja) · **简体中文** · [한국어](Installation-KO)

> ⚠️ 本页由英文版机器翻译而成，可能有不准确之处。如有疑问，请以[英文版](Installation)为准。欢迎提出修改建议。

# 安装

[发布页面](https://github.com/michalsarna/baseCreator/releases)上的每个版本都为各个系统提供现成的安装包和压缩包，另有 `SHA256SUMS.txt` 用于校验下载的文件（`sha256sum -c SHA256SUMS.txt`）。

| 系统 | 安装程序 / 安装包 | 压缩包 |
|---|---|---|
| Windows 10/11（64 位） | `basecreator-vX.Y.Z-windows-x86_64-setup.exe` | `basecreator-vX.Y.Z-windows-x86_64.zip` |
| macOS，Apple 芯片（M1 及更新） | `basecreator-vX.Y.Z-macos-arm64.dmg` | – |
| macOS，Intel | `basecreator-vX.Y.Z-macos-x86_64.dmg` | – |
| Debian / Ubuntu 及其衍生版，x86_64 | `basecreator-vX.Y.Z-linux-amd64.deb` | `basecreator-vX.Y.Z-linux-x86_64.tar.gz` |
| Debian / Ubuntu 及其衍生版，ARM64 | `basecreator-vX.Y.Z-linux-arm64.deb` | `basecreator-vX.Y.Z-linux-arm64.tar.gz` |
| Fedora、openSUSE，x86_64 | `basecreator-vX.Y.Z-linux-x86_64.rpm` | `basecreator-vX.Y.Z-linux-x86_64.tar.gz` |
| Fedora、openSUSE，ARM64 | `basecreator-vX.Y.Z-linux-aarch64.rpm` | `basecreator-vX.Y.Z-linux-arm64.tar.gz` |
| 其他 Linux | – | 上面的 `.tar.gz` |
| FreeBSD，x86_64 | – | `basecreator-vX.Y.Z-freebsd-x86_64.tar.gz` |

## 包管理器

| 系统 | 命令 |
|---|---|
| macOS（Homebrew） | `brew install --cask michalsarna/tap/basecreator`（之后用 `brew upgrade` 更新） |
| Windows（Scoop） | `scoop bucket add creator https://github.com/michalsarna/scoop-bucket`，然后 `scoop install basecreator`（之后用 `scoop update basecreator` 更新） |
| Windows（winget） | `winget install MichalSarna.BaseCreator`，待 Microsoft 将该软件包收入其目录后可用（已提交，正在等待审核） |

Arch Linux（AUR）软件包已准备好，但尚未发布。

## Windows
运行 `-setup.exe` 安装程序（会在开始菜单中添加条目，并在 *设置 → 应用*（*Settings → Apps*）中提供卸载程序），或者把 `.zip` 解压到任意位置并启动 `basecreator.exe`。安装程序没有代码签名，因此 Windows SmartScreen 可能会发出警告：点击 *更多信息 → 仍要运行*（*More info → Run anyway*）。

## macOS
打开适合你的 Mac 的 `.dmg`（*苹果菜单 → 关于本机*（*Apple menu → About This Mac*）会告诉你是 Apple 芯片还是 Intel），并把 *BaseCreator* 拖到 *应用程序*（*Applications*）中。该应用只有临时（ad-hoc）签名，没有经过公证：第一次运行时请使用 **右键 → 打开 → 打开**（**right-click → Open → Open**）。如果 macOS 提示应用“已损坏”，请执行一次以下命令去掉隔离标记：

```sh
xattr -dr com.apple.quarantine /Applications/BaseCreator.app
```

## Linux
安装 `.deb`（`sudo apt install ./basecreator-vX.Y.Z-linux-amd64.deb`），在 Fedora 或 openSUSE 上安装 `.rpm`（`sudo dnf install ./basecreator-vX.Y.Z-linux-x86_64.rpm`，openSUSE：`sudo zypper install ./...`；它需要较新的 glibc，因此不适用于 RHEL / Alma / Rocky 9），或者解压 `.tar.gz` 并运行 `basecreator`。程序需要常见的 X11 / Wayland 和 OpenGL 库（`libxkbcommon-x11-0`、`libgl1`）；`.deb` 和 `.rpm` 已将它们列为依赖。

## FreeBSD
解压 `.tar.gz` 并运行 `basecreator`。它需要几个软件包（游戏控制器通过 `libudev-devd` 工作）以及 X11 或 Wayland 桌面（以 root 身份执行）：

```sh
pkg install libudev-devd libxkbcommon mesa-libs
```

程序已在 FreeBSD 15.0 上构建并运行测试（仅 x86_64）；尚未在 FreeBSD 桌面上试用。USB 串口设备是 `/dev/cuaU0`（用户必须属于 `dialer` 组）。要在那里从源代码构建，请先执行 `pkg install rust pkgconf libudev-devd evdev-proto`。

## 从源代码构建
```sh
git clone https://github.com/michalsarna/baseCreator
cd baseCreator
cargo run --release -p basecreator
```

## 更新
程序启动时会检查是否有更新的版本（可在 *设置 → 程序选项…*（*Settings → Program options…*）中关闭），*帮助 → 检查更新…*（*Help → Check for updates…*）则可随时检查。
