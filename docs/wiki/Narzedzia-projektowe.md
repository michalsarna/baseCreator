🌐 [English](Design-Tools) · **Polski**

# Narzędzia projektowe

BaseCreator edytuje kształty tymi samymi narzędziami co LightCreator, MillCreator i PenCreator. Klawisze są na stronie [Skróty klawiszowe](Skroty-klawiszowe), a menu na stronie [Menu](Menu).

## Rysowanie

Paleta po lewej ma **Zaznaczanie (V)**, **Edycja węzłów (N)**, **Prostokąt (R)**, **Elipsa (E)**, **Trójkąt (Y)**, **Gwiazda (S)**, **Wielokąt (G)**, **Serce (K)**, **Spirala (I)**, **Linia (L)**, **Linia łamana / pióro (P)**, **Tekst (T)**, **Przesuwanie widoku (H)**, **Powiększ (Z)** i **Pomniejsz (X)**. Narzędzia wstawiające obiekty są też w menu **Wstawianie**, razem z **Generuj kształt…** (generatory wtyczek) i **Cliparty z internetu…**.

## Tekst

Wybierz **Tekst (T)** i kliknij na stronie: pojawia się tekst, a karta **Właściwości** otwiera się z polem do pisania. Ustawisz tam czcionkę, rozmiar, pogrubienie i kursywę, odstępy między literami i wierszami oraz wyrównanie. **Zamień na krzywe** robi z tekstu kształty, których węzły można edytować. Nowy tekst bierze ustawienia ostatnio zmienianego.

## Edycja

* **Prawy przycisk myszy** na kształcie otwiera jego menu: kopiowanie, grupowanie, blokowanie, odbicia i obroty o 90°, podmenu **Rozmieść** i operacji na kształtach, a dla obrazu **Dostosuj obraz…** i **Wektoryzacja obrazu…**. Pozycje niepasujące do zaznaczenia są szare.
* **Dwa razy Esc** (w ciągu pół sekundy) wraca do **Zaznaczanie (V)**.
* **Cofnij** pamięta do 100 kroków dla każdego rysunku.

## Operacje na kształtach

**Suma**, **Część wspólna**, **Odejmij** i **Różnica symetryczna** łączą zamknięte kształty. **Przesuń obrys…** robi większy lub mniejszy obrys, **Zaokrąglij rogi…** zaokrągla narożniki, **Zamień na krzywe** daje krzywe zamiast linii, a **Zamień na ścieżkę** robi ścieżkę z prostokąta lub gwiazdy, aby można było edytować jej węzły.

## Układanie i kopiowanie

Wszystko to jest w menu **Rozmieść**:

* **Połącz ścieżki automatycznie…** łączy otwarte ścieżki, których końce są bliżej niż tolerancja; **Zamknij ścieżkę z tolerancją…** zamyka te, których oba końce są tak blisko. **Narożniki dogbone / T-bone…** dodaje wycięcie w ostrych narożnikach kształtów, które będą frezowane.
* **Tablica siatkowa…** kopiuje w wierszach i kolumnach. **Tablica kołowa…** układa kopie wokół środka zaznaczenia. Powstaje najwyżej 2000 kopii.
* **Kopiuj wzdłuż ścieżki…**: zaznacz kształty, a ścieżkę **na końcu**. **Tekst na ścieżce…**: zaznacz tekst, a ścieżkę na końcu; potem są to obrysy, nie tekst.
* **Przytnij kształtem…**: zaznacz linie, a zamknięty kształt **na końcu**. **Usuń duplikaty** kasuje kształty o tym samym obrysie na tej samej warstwie. **Rozdziel** robi osobny kształt z każdego obrysu. **Uprość…** usuwa punkty leżące w granicach tolerancji od linii.
* **Tekst seryjny…**: tekst jest kopiowany z rosnącym `{n}` albo, z plikiem CSV, raz dla każdego wiersza. **Kod kreskowy / QR…** wstawia kod jako obrysy na używaną warstwę.
* **Rozmieść kształty (nesting)…** układa zaznaczone kształty w rzędach według ich prostokątów, nie obrysów. **Pomiar…** pokazuje rozmiar, położenie, pole i długość zaznaczenia, a dla dwóch kształtów odległości między nimi.

## Obrazy

**Plik → Importuj → Importuj obraz…** albo upuszczenie obrazu na okno otwiera okno importu. **Dostosuj obraz…** zmienia obraz na stronie, a **Wektoryzacja obrazu…** zamienia jego ciemne lub jasne obszary w obrysy. Obrazy nie mają własnych obrysów, więc eksport pomija je, dopóki nie zostaną zwektoryzowane.

**Widok → Obraz w tle…** kładzie zdjęcie lub skan pod rysunkiem, aby rysować po nim.

## Galeria clipartów

Karta **Clipart** ma gotowe obrazki w galerii wspólnej z innymi programami; **Wstawianie → Cliparty z internetu…** szuka darmowych obrazków w internecie.

## Prawy panel

Karty to **Właściwości**, **Warstwy**, **Clipart** i **Opis**. Zobacz [Panele i karty](Panele-i-karty#prawy-panel).
