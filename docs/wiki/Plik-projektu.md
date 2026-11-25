🌐 [English](Project-File) · **Polski**

# Plik projektu

BaseCreator zapisuje **`.cre`**, plik projektu, który zapisują też LightCreator, MillCreator i PenCreator: jeden plik JSON z rysunkiem, opisem i **jedną sekcją na program** (`draw`, `laser`, `mill`, `pen`). Każdy program zapisuje własną sekcję i nie rusza pozostałych, więc projekt przechodzi między programami bez strat.

Sekcja `draw` zawiera stronę i 64 warstwy (nazwa, kolor, widoczność).

* Projekt zrobiony tutaj otwiera się w pozostałych programach z rysunkiem i opisem. Zachowują one `draw`.
* Projekt zrobiony tam otwiera się tutaj z rysunkiem. Dostaje domyślne warstwy i stronę ostatniego nowego rysunku, powiększoną, gdy rysunek się nie mieści. Jego ustawienia maszyny zostają w pliku po zapisie tutaj.
* Plik z nowszym formatem `draw` jest odrzucany, z prośbą o aktualizację programu.

## Twórca i współtwórcy

Plik zapisuje też, kto zrobił projekt i kto nad nim pracował, w bloku `authorship`:

* **Twórca**: osoba, która zapisała projekt pierwsza.
* **Współtwórcy**: każdy inny, kto go zapisał, dodany raz.
* **Edycje**: każdy zapis (ostatnie 200). Zapis automatyczny łączy się z poprzednią edycją, gdy jest tej samej osoby w tym samym programie.

Każdy wpis ma imię, czas (UTC) oraz program z wersją. Imię ustawia się w **Ustawienia → Opcje programu → Twoje imię i nazwisko**. Jest w jednym pliku wspólnego folderu (`author.json`), więc jest takie samo w BaseCreatorze, LightCreatorze, MillCreatorze i PenCreatorze. Bez imienia zapis zapamiętuje czas i program, ale nie osobę.

**Plik → Informacje o projekcie…** pokazuje nazwę, folder i rozmiar pliku, czas utworzenia i zmiany pliku, czas utworzenia projektu, twórcę, współtwórców i edycje.

Projekt mający plik jest zapisywany sam około 20 sekund po zmianie. Zobacz [Ustawienia i pliki](Ustawienia-i-pliki).
