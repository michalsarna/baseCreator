🌐 [English](Project-File) · **Polski** · [简体中文](Project-File-ZH) · [한국어](Project-File-KO)

# Plik projektu

BaseCreator zapisuje **`.cre`**, plik projektu, który zapisują też LightCreator, MillCreator i PenCreator: jeden plik JSON z rysunkiem, opisem i **jedną sekcją na program** (`draw`, `laser`, `mill`, `pen`). Każdy program zapisuje własną sekcję i nie rusza pozostałych, więc projekt przechodzi między programami bez strat.

Sekcja `draw` zawiera stronę i 64 warstwy (nazwa, kolor, widoczność).

* Projekt zrobiony tutaj otwiera się w pozostałych programach z rysunkiem i opisem. Zachowują one `draw`.
* Projekt zrobiony tam otwiera się tutaj z rysunkiem. Dostaje domyślne warstwy i stronę ostatniego nowego rysunku, powiększoną, gdy rysunek się nie mieści. Jego ustawienia maszyny zostają w pliku po zapisie tutaj.
* Plik z nowszym formatem `draw` jest odrzucany, z prośbą o aktualizację programu.

## Twórca i współtwórcy

Plik zapisuje też, kto zrobił projekt i kto nad nim pracował, w bloku `authorship`:

* **Twórca**: osoba, która zapisała projekt pierwsza.
* **Współtwórcy**: każdy inny, kto go zapisał, dodany raz. Twórca jest zapisany z imieniem i nazwiskiem, e-mailem i polem Inne; współtwórca (i edycja) tylko z imieniem i nazwiskiem oraz e-mailem, a gdy e-mail jest taki jak twórcy, tylko z imieniem i nazwiskiem.
* **Edycje**: zapisy ręczne (ostatnie 200), najwyżej jeden na minutę dla tej samej osoby i programu. Zapis automatyczny nie dodaje edycji.

Każdy wpis ma imię i nazwisko, e-mail i dowolny tekst (firma, strona WWW…), czas (UTC) oraz program z wersją. Ustawia się je w **Ustawienia → Opcje programu**, w części **Autor**: **Twoje imię i nazwisko**, **E-mail** i **Inne**. Są w jednym pliku wspólnego folderu (`author.json`), więc są takie same w BaseCreatorze, LightCreatorze, MillCreatorze i PenCreatorze. Bez imienia i e-maila zapis zapamiętuje czas i program, ale nie osobę.

**Plik → Informacje o projekcie…** pokazuje plik (nazwa, folder, rozmiar, czas utworzenia i zmiany), autorów (czas utworzenia projektu, twórca z e-mailem i polem Inne w osobnych wierszach, współtwórcy z e-mailem) oraz tabelę edycji: czas, osoba, program.

Projekt mający plik jest zapisywany sam około 20 sekund po zmianie. Zobacz [Ustawienia i pliki](Ustawienia-i-pliki).

## Ochrona hasłem

Projekt można chronić hasłami: w **Ustawieniach projektu…** LightCreatora, MillCreatora i PenCreatora oraz w **Plik → Ustawienia strony…** BaseCreatora, w części **Ochrona**.

* **Hasło właściciela**: wszystko (edycja, kopiowanie, eksport, zmiana lub usunięcie ochrony).
* **Hasło otwarcia** (opcjonalne): projekt można otworzyć, oglądać, zamienić w G-code i uruchomić na maszynie, można też zmienić materiał i ustawienia warstw (prędkość, moc, przejścia, głębokość, frez, pisak), ale rysunku nie da się edytować, kopiować, eksportować jako rysunku (SVG, DXF, Illustrator, LightBurn …) ani drukować. Wbudowana strona WWW nie pokazuje wtedy rysunku.
* Gdy jest tylko hasło właściciela, nikt inny nie otworzy pliku. Bez hasła pliku nie da się w ogóle odczytać, a **zapomnianego hasła nie da się odzyskać**.

Plik pozostaje tekstem JSON z kluczem `protected_cre`; projekt w nim jest zaszyfrowany **XChaCha20-Poly1305** losowym kluczem 256-bitowym, zapisanym raz dla każdego hasła i zaszyfrowanym kluczem zrobionym z hasła przez **Argon2id** (64 MiB, 3 przebiegi, własna sól). Każdy zapis ma nowy nonce; zmieniony plik się nie otworzy. Hasła muszą mieć co najmniej 8 znaków, a hasło otwarcia musi być inne niż hasło właściciela.

To, czego zabrania hasło otwarcia, egzekwują programy, tak jak hasło właściciela w PDF: kto je zna, może w zasadzie odszyfrować projekt innymi narzędziami. Hasło właściciela zachowaj dla siebie.
