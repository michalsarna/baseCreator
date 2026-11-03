🌐 [English](Plugins) · **Polski**

# Wtyczki

Wtyczki dodają funkcje bez zmiany programu. Są wspólne dla BaseCreatora, LightCreatora, MillCreatora i PenCreatora: wtyczka leży w katalogu `plugins` we wspólnym katalogu danych (`baseCreator`), a każdy program używa tych jej części, które do niego pasują.

**Ustawienia → Wtyczki…** otwiera okno: instalacja wtyczki z `.zip`, włączanie, wyłączanie i usuwanie wtyczek, otwarcie katalogu.

Co tu działa:

* **Importery:** **Plik → Importuj → Importuj wtyczką…**, przeciągnięcie pliku albo **Plik → Otwórz…** na pliku, którego rozszerzenie czyta wtyczka. Kształty są dodawane do rysunku.
* **Generatory:** **Plik → Importuj → Generuj kształt…** (także **Wstawianie → Generuj kształt…**) pokazuje formularz generatora i dodaje utworzone kształty.
* **Cliparty i tłumaczenia.**

Wtyczkę z **postprocesorami G-code** (`gcode-tools`, `gcode-dialects`) można zainstalować, ale tutaj nic nie robi, bo nie ma G-code.

**Bezpieczeństwo.** Skrypty wtyczek (Rhai) działają w piaskownicy bez dostępu do plików, sieci i systemu, z limitem czasu i pamięci. Instaluj tylko wtyczki, którym ufasz.

## Oficjalne wtyczki

**Ustawienia → Wtyczki… → Oficjalne wtyczki** pokazuje listę z repozytorium [baseCreator-plugins](https://github.com/michalsarna/baseCreator-plugins). **Zainstaluj** pobiera wtyczkę, ze sprawdzeniem sumy kontrolnej, do wspólnego katalogu wtyczek.

Pisanie własnej: [przewodnik dla autorów](https://github.com/michalsarna/baseCreator-plugins/blob/main/docs/GUIDE.md) i [PLUGINS.md](https://github.com/michalsarna/baseCreator/blob/main/docs/PLUGINS.md).
