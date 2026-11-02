# Plan: baseCreator jako samodzielna aplikacja (macOS, Windows, Linux, tablet z Androidem)

Stan na 2026-11-02. Plan, nic jeszcze nie zrobione. (Uwaga z prac nad G-code: edytor bez maszyn nie wysyła nic do sterownika, więc poprawka ASCII z base v1.58.0 go nie dotyczy.)

## 1. Cel

Edytor rysunku zbudowany na bibliotekach baseCreator (`base-core`, `base-ui`), **bez żadnego związku z maszynami**:
bez urządzeń, połączeń, G-code, frezów, materiałów, pisaków i laserów. Działa na trzech systemach biurkowych i na
tablecie z Androidem.

Najważniejsza zaleta: zapisuje ten sam plik projektu `.cre` co LightCreator, MillCreator i PenCreator. Rysunek zrobiony
na tablecie otwiera się potem w każdym z tych programów, a sekcje maszyn, których edytor nie zna, zostają w pliku
nietknięte (`base_core::project` już tak działa).

## 2. Zakres

**Jest:**
- płótno z narzędziami: zaznaczanie, węzły, prostokąt, elipsa, wielokąt, gwiazda, linia, Bézier, tekst, lasso;
- operacje: grupowanie, wyrównanie, rozmieszczenie, kolejność, odbicie, obrót, operacje logiczne (suma, różnica,
  część wspólna, wykluczenie), offset, zaokrąglanie narożników, łączenie ścieżek;
- obrazy i ich obrysowanie (trace), galeria clipartów (lokalna i z internetu), wtyczki generujące kształty;
- warstwy jako **grupy kolorów**: nazwa, kolor, widoczna, zablokowana; bez parametrów maszyny;
- **strona zamiast stołu maszyny:** A4, A3, Letter, własny rozmiar, pionowo i poziomo; mm albo cale;
- import: SVG, DXF, PDF, AI, LightBurn, PNG/JPEG;
- eksport: SVG, DXF, PDF, PNG;
- projekt `.cre` z opisem, autozapis, kopie zapasowe, cofnij i ponów, karty wielu projektów, panele;
- 30 języków, motyw jasny i ciemny, menu tego samego układu bez **Maszyny** i menu procesu:
  Plik · Edycja · Wstaw · Rozmieść · Widok · Ustawienia · Pomoc.

**Nie ma:** urządzeń, połączeń (serial, TCP, ser2net), sterowania, konsoli, firmware, kamery, ffmpeg, G-code (tekstu,
podglądu, eksportu), symulatora, bibliotek frezów, materiałów i pisaków, kalibracji.

## 3. Gdzie i pod jaką nazwą (decyzja użytkownika)

- **Propozycja:** nowy crate `crates/basecreator` (program) w repozytorium baseCreator. Zależności przez `path`, więc
  nie trzeba przypinać tagów, a zmiana w bibliotece i w programie to jeden PR. Wydania aplikacji mają osobne tagi,
  np. `app-v0.1.0`, żeby nie myliły się z tagami biblioteki `v1.x`.
- **Alternatywa:** osobne repozytorium, przypinane jak LC, Mill i Pen. Daje czystszą listę wydań, ale dokłada jeszcze
  jedno przypinanie po każdej wersji base.
- **Nazwa** wyświetlana w programie, na stronie, w Homebrew i w sklepie: „baseCreator” albo inna (np. „DrawCreator”),
  żeby nie mylić programu z biblioteką. Do decyzji.

## 4. Etapy

### Etap 0: przygotowanie base-ui (średni)
- Cechy (features) w `base-ui`:
  - `machine`: `serialport`, `link`, `control_ui`, `console_ui`, `firmware_ui`, `job_ui`, `machine_ctl`, `duet`,
    `device_pages`, `gcode_*`, `tools_ui` (kalibracja), `camera*`, `ffmpeg`;
  - `native-dialogs`: `rfd`, którego nie ma na Androidzie.

  Obie są domyślnie włączone, więc LC, Mill i Pen się nie zmieniają. Edytor buduje się z
  `default-features = false`.
- Moduły, które dziś używają `rfd` w kodzie edytora (`image_ui`, `design_ui`, `shape_ops`, `background_ui`,
  `clipart_ui`, `plugins_ui`, `backup_ui`), dostają jedną warstwę „wybierz plik / zapisz plik” z dwiema
  implementacjami: `rfd` na komputerze i system Androida na tablecie (etap 3).
- Części powielone w trzech aplikacjach (karty projektów, panele, autozapis, kopie, wtyczki, menu) trafiają do
  `base-ui`, gdzie się da. Zyskują na tym też LC, Mill i Pen.
- **Test:** CI buduje `base-ui` bez domyślnych cech (pilnuje, że edytor nie wciąga kodu maszyn).

### Etap 1: program na komputer, MVP (duży)
- Nowy program: około 3–4 tys. linii kleju. Dla porównania PenCreator ma 9,7 tys., z czego około 5 tys. to część
  maszynowa, której tu nie będzie.
- Wzorem jest PenCreator (najmniejszy).
- Sekcja projektu `draw` z ustawieniami strony i warstw. Sekcje `laser`, `mill` i `pen` zostają nietknięte.
- Testy:
  - otwarcie i zapis pliku z LC, Mill i Pen nie gubi ich sekcji;
  - eksport SVG, DXF i PDF;
  - kolizje skrótów w menu (`key_clashes`);
  - testy egui jak w aplikacjach.
- Wiki EN+PL: Szybki start, Menu, Skróty, Plik projektu.

### Etap 2: wydania na komputery (mały)
- `release.yml` skopiowany z PenCreatora, bez ffmpeg i bez kamery:
  - macOS `.dmg` (arm64 i x86_64),
  - Windows `.zip` i instalator,
  - Linux AppImage, `.deb` i AUR.
- Homebrew cask, winget (z tym samym znanym problemem co dziś), strona creatorapps, zasada 10 wydań.

### Etap 3: tablet z Androidem (duży, najwięcej niewiadomych)
- **Budowanie:**
  - eframe z cechą `android-game-activity` (GameActivity lepiej obsługuje klawiaturę ekranową niż NativeActivity);
  - crate jako `cdylib` z `android_main`;
  - budowanie przez `cargo-ndk` i małe opakowanie Gradle;
  - architektury arm64-v8a, a x86_64 dla emulatora.
- **Ekran dotykowy:**
  - szczypanie przybliża, dwa palce przesuwają płótno;
  - długie przytrzymanie działa jak prawy przycisk (menu kontekstowe);
  - większe odstępy i przyciski (skala stylu egui);
  - podpowiedzi „po najechaniu” nie działają na dotyku, więc ważne opisy muszą być widoczne;
  - rysik działa jak palec, bez nacisku.
- **Układ:** pasek menu za wąski, więc pasek narzędzi plus menu „☰”. Panele w poziomie, w pionie jeden panel na raz.
- **Klawiatura ekranowa:** `AndroidApp::show_soft_input`, gdy egui chce wpisywania tekstu (pola, narzędzie Tekst).
  **Największe ryzyko:** wpisywanie tekstu w egui na Androidzie bywa kłopotliwe (polskie znaki, autokorekta).
- **Pliki:**
  - Storage Access Framework przez JNI (`ACTION_OPEN_DOCUMENT` i `ACTION_CREATE_DOCUMENT`) do otwierania i zapisu
    `.cre`, SVG i PDF z dowolnego miejsca (Dysk Google, pobrane);
  - pierwszy krok, prostszy: lista projektów w pamięci aplikacji plus „Udostępnij” do eksportu.
- **Cykl życia:** autozapis przy `onPause` (Android może zamknąć aplikację w tle), odtworzenie przy powrocie, obrót
  ekranu.
- **Czcionki:** `/system/fonts` plus czcionki wbudowane.
- **Bez zmian:** cliparty z sieci (`ureq` z rustls) i wtyczki.
- **Dystrybucja:**
  - najpierw podpisany `.apk` w wydaniach GitHub (instalacja ręczna);
  - później Google Play: konto dewelopera (opłata jednorazowa), polityka prywatności, wymagany docelowy poziom API,
    zrzuty ekranu.
- **Test:** CI buduje APK. Uruchomienie w emulatorze (smoke test) i na prawdziwym tablecie użytkownika.

## 5. Ryzyka

1. Klawiatura ekranowa i polskie znaki w egui na Androidzie. Sprawdzić na prototypie przed resztą etapu 3.
2. Wybór i zapis plików przez SAF (JNI) to nowy kod, którego dziś nie ma nigdzie.
3. Wydajność dużych rysunków (tysiące ścieżek, obrysowane zdjęcia) na tablecie.
4. Rozdzielenie `base-ui` na cechy może odsłonić ukryte zależności edytora od kodu maszyn.
5. Brak urządzenia testowego po mojej stronie: każdą wersję na tablet trzeba sprawdzić na tablecie użytkownika.

## 6. Kolejność i pierwszy krok

1. Prototyp na Androida w jeden dzień pracy: puste okno eframe na tablecie plus płótno `base-ui` z jednym prostokątem,
   pole tekstowe z klawiaturą ekranową, szczypanie. To potwierdza albo obala ryzyko nr 1, zanim powstanie reszta.
2. Etap 0, potem 1, potem 2 (program na komputer da się wydać niezależnie od Androida).
3. Etap 3.

## 7. Decyzje (2026-11-02)

- Nazwa **BaseCreator** (jak LightCreator, MillCreator, PenCreator); crate `crates/basecreator` w repozytorium baseCreator.
- Android: podpisany APK w wydaniach GitHub, bez Google Play.
- Tablet: użytkownik nie ma tabletu z Androidem. Cel: Android 10+ (API 29), arm64-v8a, ekran 10–11", test w emulatorze.

## 8. Wynik prototypu na Androida (2026-11-02)

Źródła: `~/Downloads/basecreator-prototyp` (`env.sh` ustawia narzędzia; budowa:
`cargo ndk -t arm64-v8a -P 29 -o android/app/src/main/jniLibs build`, potem `./gradlew assembleDebug` w `android/`).
Emulator `bc_tablet` (Pixel Tablet, Android 15, bez klawiatury sprzętowej).

Narzędzia: eframe 0.37 (glow) z cechą `android-game-activity`, winit 0.30.13, android-activity 0.6.1,
`androidx.games:games-activity:4.4.0`, AGP 9.4.1, Gradle 9.8.2, NDK 29, minSdk 29, targetSdk 35. Rust z rustup
(1.99.1 z celami Androida) tylko do budowy na Androida; LC, Mill i Pen nadal budują się Rustem z Homebrew.

Działa:
- program startuje, egui rysuje, dotyk działa;
- klawiatura ekranowa pokazuje się, gdy pole dostaje fokus (winit sam wywołuje `show_soft_input`);
- znaki spoza ASCII (sprawdzone na „à” z długiego przytrzymania), Backspace i podpowiedzi klawiatury;
- obrót ekranu i powrót z tła nie gubią stanu.

Do zrobienia w etapie 3:
- **Most klawiatury:** winit 0.30 pomija zdarzenia tekstu GameActivity. Prototyp co klatkę czyta
  `AndroidApp::text_input_state()` i różnicę podaje egui jako tekst i Backspace (`raw_input_hook`).
  Odczyt jest możliwy dopiero po pierwszym `set_text_input_state`, bo wcześniej android-activity 0.6.1 robi slice z
  pustego wskaźnika i program pada.
- **Enter (✓)** na klawiaturze nic nie robi: to akcja IME, nie klawisz. Trzeba ją zamienić na Enter.
- **Klawiatura po powrocie z tła** jest schowana, a program myśli, że jest widoczna. Trzeba ją pokazać ponownie
  przy wznowieniu.
- **Wcięcia systemowe:** treść wchodzi pod pasek stanu. Trzeba odsunąć ją o wysokość paska.
- **Szczypanie:** jeszcze nie sprawdzone (`adb` nie umie dwóch palców). Do sprawdzenia ręcznie w oknie emulatora,
  na przykład gestem z Ctrl, albo na prawdziwym tablecie.

`base-ui` na Androida: cały kod, łącznie z `serialport`, `ureq`/`ring`, `base-core` i `image`, kompiluje się na
`aarch64-linux-android`. Jedyna przeszkoda to `rfd`: 20 miejsc w 13 plikach. Etap 0 to więc głównie jedna warstwa
„wybierz/zapisz plik” zamiast `rfd`. Cecha `machine` jest potrzebna tylko po to, żeby program był mniejszy.

## 9. Pytania do użytkownika (stare)

- Nazwa programu i repozytorium: crate w baseCreator czy osobne repo?
- Jaki tablet: model, wersja Androida? Wyznacza minimalny poziom API i rozmiar ekranu do projektowania.
- Android: wystarczy APK z GitHuba, czy od razu Google Play?
- Czy program ma mieć przycisk „Otwórz w LightCreator/MillCreator/PenCreator” na komputerze?
- Czy rysik ma coś dawać ponad palec (np. rysowanie odręczne linią)?
