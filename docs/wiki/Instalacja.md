🌐 [English](Installation) · **Polski**

# Instalacja

Każde wydanie na [stronie Releases](https://github.com/michalsarna/baseCreator/releases) zawiera gotowe pakiety i archiwa dla każdego systemu oraz `SHA256SUMS.txt` do sprawdzenia pobranych plików (`sha256sum -c SHA256SUMS.txt`).

| System | Instalator / pakiet | Archiwum |
|---|---|---|
| Windows 10/11 (64-bit) | `basecreator-vX.Y.Z-windows-x86_64-setup.exe` | `basecreator-vX.Y.Z-windows-x86_64.zip` |
| macOS, Apple silicon (M1 i nowsze) | `basecreator-vX.Y.Z-macos-arm64.dmg` | – |
| macOS, Intel | `basecreator-vX.Y.Z-macos-x86_64.dmg` | – |
| Debian / Ubuntu i pochodne, x86_64 | `basecreator-vX.Y.Z-linux-amd64.deb` | `basecreator-vX.Y.Z-linux-x86_64.tar.gz` |
| Debian / Ubuntu i pochodne, ARM64 | `basecreator-vX.Y.Z-linux-arm64.deb` | `basecreator-vX.Y.Z-linux-arm64.tar.gz` |
| Inny Linux | – | `.tar.gz` powyżej |

## Menedżery pakietów

| System | Polecenie |
|---|---|
| macOS (Homebrew) | `brew install --cask michalsarna/tap/basecreator` (potem `brew upgrade`) |
| Windows (Scoop) | `scoop bucket add creator https://github.com/michalsarna/scoop-bucket`, potem `scoop install basecreator` (potem `scoop update basecreator`) |
| Windows (winget) | `winget install MichalSarna.BaseCreator`, gdy Microsoft przyjmie pakiet do swojego katalogu (zgłoszony, czeka na przegląd) |

Pakiet dla Arch Linuxa (AUR) jest przygotowany, ale jeszcze nieopublikowany.

## Windows
Uruchom instalator `-setup.exe` (wpis w menu Start i dezinstalator w *Ustawienia → Aplikacje*) albo rozpakuj `.zip` gdziekolwiek i uruchom `basecreator.exe`. Instalator nie jest podpisany, więc Windows SmartScreen może ostrzec: *Więcej informacji → Uruchom mimo to*.

## macOS
Otwórz `.dmg` dla swojego Maca (*Menu Apple → Ten Mac* mówi, czy to Apple silicon, czy Intel) i przeciągnij *BaseCreator* do *Programów*. Aplikacja jest podpisana tylko ad hoc, bez notaryzacji: za pierwszym razem użyj **prawy przycisk → Otwórz → Otwórz**. Jeśli macOS pisze, że aplikacja jest „uszkodzona”, jednorazowo usuń flagę kwarantanny:

```sh
xattr -dr com.apple.quarantine /Applications/BaseCreator.app
```

## Linux
Zainstaluj `.deb` (`sudo apt install ./basecreator-vX.Y.Z-linux-amd64.deb`) albo rozpakuj `.tar.gz` i uruchom `basecreator`. Program potrzebuje zwykłych bibliotek X11 / Wayland i OpenGL (`libxkbcommon-x11-0`, `libgl1`); pakiet `.deb` je wymienia.

## Ze źródeł
```sh
git clone https://github.com/michalsarna/baseCreator
cd baseCreator
cargo run --release -p basecreator
```

## Aktualizacje
Program sprawdza przy starcie, czy jest nowsze wydanie (*Ustawienia → Opcje programu…* to wyłącza), a *Pomoc → Sprawdź aktualizacje…* robi to w dowolnej chwili.
