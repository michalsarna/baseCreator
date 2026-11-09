🌐 [English](Style-Trace) · **Polski**

# Obrysuj w stylu

**Rozmieść → Obrysuj w stylu…** rysuje zaznaczony obraz na nowo liniami w jednym z 19 stylów, gotowymi, by je wyeksportować, wydrukować albo otworzyć w innych programach: wężyki, spirala, fale, koła rastra, kreskowanie, puentylizm, jedna ciągła linia, linie przepływu, szkic i inne. Polecenie jest też w menu prawego przycisku obrazu i w sekcji **Obraz** karty **Właściwości**. Działa tak samo w LightCreatorze, MillCreatorze, PenCreatorze i BaseCreatorze.

![Okno Obrysuj w stylu, linie przepływu](https://raw.githubusercontent.com/michalsarna/LightCreator/main/docs/screenshots/14b-style-trace.png)

## Krok po kroku

1. Zaznacz obraz i wybierz **Obrysuj w stylu…**.
2. Wybierz **Styl**; wiersz pod nim mówi, co rysuje. Niektóre style mają **Rodzaj** (na przykład kropki, kreski albo trójkąty w puentylizmie).
3. Przesuwaj ustawienia stylu. Podgląd pokazuje linie dokładnie tak, jak trafią na rysunek; liczy się w tle, więc okno nigdy nie staje (stare linie są szare, dopóki nowe nie będą gotowe). Wiersz pod przyciskami podaje, ile jest linii i jak są długie razem.
4. **Ton:** **Punkt czerni** i **Punkt bieli** odcinają najciemniejsze i najjaśniejsze szarości, **Gamma** rozjaśnia (powyżej 1) albo przyciemnia półtony, **Wektoryzuj jasne obszary** zamienia ciemne z jasnymi.
5. **Umieść na warstwie:** program wybiera pierwszą warstwę bez kształtów (nie warstwę samego obrazu). Z listy można wybrać inną warstwę.
6. **Usuń obraz po wektoryzacji** usuwa obraz, gdy linie są gotowe.
7. **Wektoryzuj** kładzie linie na rysunek jako jeden kształt, w miejscu obrazu (przesunięte, obrócone i przeskalowane razem z nim). **Edycja → Cofnij** je zabiera.

Style, które korzystają z przypadku (puentylizm, linie przepływu, szkic, bazgroły, kwadraty, igiełki, płytki Trucheta), mają numer **Losowy wzór**: ten sam numer daje zawsze te same linie, **Inny** próbuje innego.

## Style

| Styl | Co rysuje | Ustawienia |
|---|---|---|
| **Wężyk** | Rzędy linii, które falują gęściej i wyżej tam, gdzie obraz jest ciemniejszy. | **Odstęp linii**, **Wysokość fali**, **Fale na mm**, **Kąt**; Rodzaj: rzędy od brzegu do brzegu albo **Podnoś pisak nad bielą**. |
| **Piła** | Te same rzędy jako zygzaki. | Jak Wężyk. |
| **Sprężynki** | Rzędy pętelek jak rozciągnięta sprężyna, większych tam, gdzie jest ciemniej. | Jak Wężyk. |
| **Spirala** | Jedna spirala od środka, która w cieniach drga mocniej (*linie częstotliwości* innych programów). | **Odstęp zwojów**, **Wysokość fali**, **Fale na mm**; Rodzaj: fale albo zygzak. |
| **Fale** | Rzędy uniesione przez ciemność, jak krajobraz widziany z boku; rzędy z przodu zasłaniają to, co za nimi. | **Odstęp linii**, **Wysokość fal**; Rodzaj: ukryj to, co z tyłu, albo rysuj każdy rząd w całości. |
| **Koła rastra** | Siatka kół, których wielkość idzie za tonem, jak w drukowanym zdjęciu. | **Rozmiar komórki**, **Największe koło**, **Odstęp pierścieni**, **Kąt**; Rodzaj: koła, pierścienie (wypełnione pierścieniami) albo spirale. |
| **Akwaforta** | Kreskowanie warstwami, jak w dawnej rycinie: każdy ciemniejszy ton dodaje linie w innym kierunku. | **Odstęp linii**, **Poziomy tonu** (1 do 6), **Kąt**; Rodzaj: proste albo faliste linie. |
| **Kwadraty** | Siatka kwadratów jeden w drugim, więcej ich tam, gdzie jest ciemniej. | **Rozmiar komórki**, **Najwięcej kwadratów w komórce**, **Drżenie** (losowy obrót), **Kąt**. |
| **Igiełki** | Krótkie kreski na siatce, dłuższe tam, gdzie jest ciemniej. | **Rozmiar komórki**, **Długość kreski**, **Kąt**; Rodzaj: wzdłuż kształtów, pod kątem albo losowo. |
| **Płytki Trucheta** | Płytki z ćwiartek koła obrócone losowo, więcej łuków tam, gdzie jest ciemniej. | **Rozmiar płytki**, **Najwięcej łuków w płytce**. |
| **Poziomice** | Linie jednakowej jasności, jak poziomice na mapie. | **Wygładzenie**, **Poziomy tonu** (2 do 40). |
| **Krzywa Hilberta** | Jedna krzywa wypełniająca płaszczyznę, z drobniejszymi zakrętami tam, gdzie jest ciemniej. | **Najmniejszy krok**, **Poziomy tonu**; Rodzaj: jedna linia albo **Podnoś pisak nad bielą**. |
| **Puentylizm** | Kropki rozłożone według tonu (ważony puentylizm Woronoja): gęsto w cieniach, rzadko w światłach. | **Liczba kropek**, **Rozmiar kropki**; Rodzaj: kropki, koła według tonu, kreski albo trójkąty. |
| **Jedna linia (TSP)** | Kropki puentylizmu połączone w jedną linię, która się nie przecina, rysowaną bez podnoszenia pisaka. | **Liczba kropek** (do 15 000). |
| **Trójkąty i komórki** | Siatka trójkątów (Delaunay) albo komórek wokół kropek (Woronoj), na kropkach rozłożonych według tonu. | **Liczba kropek**; Rodzaj: trójkąty albo komórki. |
| **Rysunek konturowy** | Tylko krawędzie obrazu, jako linie. | **Najmniejszy szczegół**, **Czułość krawędzi**, **Najkrótsza linia**; Rodzaj: krawędzie albo krawędzie i ciemne obszary. |
| **Linie przepływu** | Długie linie płynące wzdłuż kształtów, bliżej siebie tam, gdzie jest ciemniej. | **Najmniejszy odstęp** (w najciemniejszych miejscach), **Największy odstęp** (w najjaśniejszych), **Wielkość zakrętów**, **Kąt**; Rodzaj: wzdłuż kształtów albo losowe pole przepływu. |
| **Szkic** | Wiele krótkich prostych kresek tam, gdzie obraz jest najciemniejszy, jak w szkicu ołówkiem. | **Długość kreski**, **Liczba kresek**, **Tusz jednej kreski** (ile ciemności zabiera jedna kreska); Rodzaj: osobne kreski albo kreski w łańcuchach. |
| **Bazgroły** | Jedna błądząca linia pętelek, która zabazgruje ciemne części. | **Wielkość pętli**, **Długość linii** (metry), **Tusz jednego przejścia**. |

## Dobrze wiedzieć

* Ustawienia są w milimetrach obrazu, więc większy obraz dostaje więcej linii, a nie grubsze. Niech odstęp linii będzie większy niż szerokość pisaka, plamki lasera albo freza.
* Kropka to małe zamknięte kółko, więc jest rysowana, wypalana albo grawerowana jak każda inna linia.
* **Jedna linia (TSP)**, **Spirala** i **Krzywa Hilberta** są rysowane jednym długim ruchem; pozostałe style są ułożone w kolejności, która skraca przejazdy między liniami.
* Bardzo drobne ustawienia dają mnóstwo punktów: wiersz pod przyciskami ostrzega, gdy rysunek zwolni, a wynik kończy się na dwóch milionach punktów.
* Gdy styl potrzebuje większego kontrastu, najpierw dostosuj obraz (**Rozmieść → Dostosuj obraz…**: jasność, kontrast, poziomy).

Zobacz też: [Narzędzia projektowe](Narzedzia-projektowe), [Menu](Menu).
