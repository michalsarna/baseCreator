🌐 [English](Installation) · [Polski](Instalacja) · [简体中文](Installation-ZH) · **한국어**

# 설치

[릴리스 페이지](https://github.com/michalsarna/baseCreator/releases)의 모든 릴리스에는 시스템별로 바로 쓸 수 있는 패키지와 압축 파일, 그리고 다운로드를 검증하기 위한 `SHA256SUMS.txt`(`sha256sum -c SHA256SUMS.txt`)가 들어 있습니다.

| 시스템 | 설치 프로그램 / 패키지 | 압축 파일 |
|---|---|---|
| Windows 10/11 (64비트) | `basecreator-vX.Y.Z-windows-x86_64-setup.exe` | `basecreator-vX.Y.Z-windows-x86_64.zip` |
| macOS, Apple silicon (M1 이상) | `basecreator-vX.Y.Z-macos-arm64.dmg` | – |
| macOS, Intel | `basecreator-vX.Y.Z-macos-x86_64.dmg` | – |
| Debian / Ubuntu 및 파생 배포판, x86_64 | `basecreator-vX.Y.Z-linux-amd64.deb` | `basecreator-vX.Y.Z-linux-x86_64.tar.gz` |
| Debian / Ubuntu 및 파생 배포판, ARM64 | `basecreator-vX.Y.Z-linux-arm64.deb` | `basecreator-vX.Y.Z-linux-arm64.tar.gz` |
| Fedora, openSUSE, x86_64 | `basecreator-vX.Y.Z-linux-x86_64.rpm` | `basecreator-vX.Y.Z-linux-x86_64.tar.gz` |
| Fedora, openSUSE, ARM64 | `basecreator-vX.Y.Z-linux-aarch64.rpm` | `basecreator-vX.Y.Z-linux-arm64.tar.gz` |
| 기타 Linux | – | 위의 `.tar.gz` |
| FreeBSD, x86_64 | – | `basecreator-vX.Y.Z-freebsd-x86_64.tar.gz` |

## 패키지 관리자

| 시스템 | 명령 |
|---|---|
| macOS (Homebrew) | `brew install --cask michalsarna/tap/basecreator` (이후 `brew upgrade`) |
| Windows (Scoop) | `scoop bucket add creator https://github.com/michalsarna/scoop-bucket`, 그다음 `scoop install basecreator` (이후 `scoop update basecreator`) |
| Windows (winget) | Microsoft가 패키지를 카탈로그에 승인한 뒤 `winget install MichalSarna.BaseCreator` (제출됨, 검토 대기 중) |

Arch Linux (AUR) 패키지는 준비되어 있지만 아직 게시되지 않았습니다.

## Windows
`-setup.exe` 설치 프로그램을 실행하거나(시작 메뉴 항목과 *설정 → 앱*(*Settings → Apps*)의 제거 프로그램이 생깁니다) `.zip`을 아무 곳에나 풀고 `basecreator.exe`를 실행합니다. 설치 프로그램은 코드 서명이 되어 있지 않으므로 Windows SmartScreen이 경고할 수 있습니다. *추가 정보 → 실행*(*More info → Run anyway*)을 누르십시오.

## macOS
Mac에 맞는 `.dmg`를 열고(*Apple 메뉴 → 이 Mac에 관하여*(*Apple menu → About This Mac*)에서 Apple silicon인지 Intel인지 알 수 있습니다) *BaseCreator*를 *응용 프로그램*(*Applications*)으로 끌어다 놓습니다. 앱은 ad-hoc 서명만 되어 있고 공증(notarize)되지 않았으므로, 처음에는 **오른쪽 클릭 → 열기 → 열기**(**right-click → Open → Open**)를 사용하십시오. macOS가 앱이 "손상되었다"고 하면 격리 플래그를 한 번 제거합니다.

```sh
xattr -dr com.apple.quarantine /Applications/BaseCreator.app
```

## Linux
`.deb`를 설치하거나(`sudo apt install ./basecreator-vX.Y.Z-linux-amd64.deb`), Fedora나 openSUSE에서는 `.rpm`을 설치하거나(`sudo dnf install ./basecreator-vX.Y.Z-linux-x86_64.rpm`, openSUSE: `sudo zypper install ./...`; 최신 glibc가 필요하므로 RHEL / Alma / Rocky 9에서는 안 됩니다), `.tar.gz`를 풀고 `basecreator`를 실행합니다. 프로그램에는 일반적인 X11 / Wayland 및 OpenGL 라이브러리(`libxkbcommon-x11-0`, `libgl1`)가 필요하며, `.deb`와 `.rpm`에는 이것들이 의존성으로 적혀 있습니다.

## FreeBSD
`.tar.gz`를 풀고 `basecreator`를 실행합니다. 몇 가지 패키지(게임 컨트롤러는 `libudev-devd`를 통해 동작합니다)와 X11 또는 Wayland 데스크톱이 필요합니다(root로 실행).

```sh
pkg install libudev-devd libxkbcommon mesa-libs
```

FreeBSD 15.0(x86_64만)에서 빌드하고 테스트를 실행했습니다. FreeBSD 데스크톱에서는 아직 시험해 보지 않았습니다. USB 직렬 장치는 `/dev/cuaU0`입니다(사용자가 `dialer` 그룹에 속해야 합니다). 그곳에서 소스로 빌드하려면 먼저 `pkg install rust pkgconf libudev-devd evdev-proto`를 실행하십시오.

## 소스에서 빌드
```sh
git clone https://github.com/michalsarna/baseCreator
cd baseCreator
cargo run --release -p basecreator
```

## 업데이트
프로그램은 시작할 때 더 새로운 릴리스가 있는지 확인합니다(*설정 → 프로그램 옵션…*(*Settings → Program options…*)에서 끌 수 있습니다). *도움말 → 업데이트 확인…*(*Help → Check for updates…*)으로 언제든지 확인할 수 있습니다.
