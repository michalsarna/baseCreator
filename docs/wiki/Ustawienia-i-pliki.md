🌐 [English](Settings-and-Files) · **Polski**

# Ustawienia i pliki

Ustawienia BaseCreatora są w jednym pliku, `app.ron`: język, schemat kolorów, siatka i opcje widoku, jednostki, rozmiar następnej strony, ostatnie pliki, kolejność kart bocznego panelu i to, czy jest ukryty, układ paneli oraz to, czy szukać aktualizacji.

| System | Miejsce |
|---|---|
| macOS | `~/Library/Application Support/baseCreator/apps/basecreator/app.ron` |
| Windows | `%APPDATA%\baseCreator\apps\basecreator\app.ron` |
| Linux | `$XDG_DATA_HOME/baseCreator/apps/basecreator/app.ron`, domyślnie `~/.local/share/baseCreator/apps/basecreator/app.ron` |

* **Wspólny katalog.** BaseCreator, LightCreator, MillCreator i PenCreator trzymają dane w jednym katalogu, `baseCreator`. Zmienna środowiskowa `BASECREATOR_DIR` wskazuje inny katalog. Są w nim `clipart/` (galeria wspólna dla programów), `plugins/` i własne ustawienia każdego programu.
* **Usuń `app.ron`, aby wyzerować wszystko.**
* **Automatyczny zapis:** około 20 sekund po zmianie i przy zamykaniu programu projekt mający plik jest zapisywany do tego pliku; rysunek bez pliku trafia do pliku odzyskiwania we wspólnym katalogu. Następny start proponuje odzyskanie takiej pracy.
* **Pomoc → Pobierz kopię zapasową…** zapisuje zip z ustawieniami BaseCreatora i wspólną galerią clipartów. **Pomoc → Przywróć z kopii zapasowej…** wczytuje taki plik po potwierdzeniu: ustawienia są zastępowane, a obrazki **dodawane** do galerii. Kopia z innego programu jest odrzucana.
* Projekty (`.cre`) i eksportowane pliki są zapisywane tam, gdzie wybierzesz.
