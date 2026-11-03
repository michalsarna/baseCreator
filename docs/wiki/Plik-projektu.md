🌐 [English](Project-File) · **Polski**

# Plik projektu

BaseCreator zapisuje **`.cre`**, plik projektu, który zapisują też LightCreator, MillCreator i PenCreator: jeden plik JSON z rysunkiem, opisem i **jedną sekcją na program** (`draw`, `laser`, `mill`, `pen`). Każdy program zapisuje własną sekcję i nie rusza pozostałych, więc projekt przechodzi między programami bez strat.

Sekcja `draw` zawiera stronę i 64 warstwy (nazwa, kolor, widoczność).

* Projekt zrobiony tutaj otwiera się w pozostałych programach z rysunkiem i opisem. Zachowują one `draw`.
* Projekt zrobiony tam otwiera się tutaj z rysunkiem. Dostaje domyślne warstwy i stronę ostatniego nowego rysunku, powiększoną, gdy rysunek się nie mieści. Jego ustawienia maszyny zostają w pliku po zapisie tutaj.
* Plik z nowszym formatem `draw` jest odrzucany, z prośbą o aktualizację programu.

Projekt mający plik jest zapisywany sam około 20 sekund po zmianie. Zobacz [Ustawienia i pliki](Ustawienia-i-pliki).
