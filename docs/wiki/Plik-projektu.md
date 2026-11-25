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
* **Edycje**: zapisy ręczne (ostatnie 200), najwyżej jeden na minutę dla tej samej osoby i programu. Zapis automatyczny nie dodaje edycji.

Każdy wpis ma imię i nazwisko, e-mail i dowolny tekst (firma, strona WWW…), czas (UTC) oraz program z wersją. Ustawia się je w **Ustawienia → Opcje programu**, w części **Autor**: **Twoje imię i nazwisko**, **E-mail** i **Inne**. Są w jednym pliku wspólnego folderu (`author.json`), więc są takie same w BaseCreatorze, LightCreatorze, MillCreatorze i PenCreatorze. Bez imienia i e-maila zapis zapamiętuje czas i program, ale nie osobę.

**Plik → Informacje o projekcie…** pokazuje plik (nazwa, folder, rozmiar, czas utworzenia i zmiany), autorów (czas utworzenia projektu, twórca i współtwórcy z e-mailem i resztą) oraz tabelę edycji: czas, osoba, program.

Projekt mający plik jest zapisywany sam około 20 sekund po zmianie. Zobacz [Ustawienia i pliki](Ustawienia-i-pliki).
