🌐 [English](Import-and-Export) · **Polski**

# Import i eksport

Listy są takie same w LightCreatorze, MillCreatorze, PenCreatorze i BaseCreatorze. Nie ma tu tego, co wymaga maszyny: nie ma G-code, a model 3D i relief STL zostają w MillCreatorze.

## Import

Wszystko z **Plik → Importuj** jest dodawane do otwartego rysunku, zostaje zaznaczone i znika po jednym cofnięciu. Na ekranie startowym powstaje dla tego nowy rysunek.

| Pozycja | Pliki | Co przychodzi |
|---|---|---|
| **Importuj obraz…** | PNG, JPG, BMP, GIF, WebP | Obraz, przez okno importu. |
| **Importuj SVG…** (Ctrl+I) | `.svg` | Ścieżki, każda na warstwie o najbliższym kolorze. |
| **Importuj Adobe Illustrator / PDF / EPS / VectorCraft…** | `.pdf`, `.ai` zgodny z PDF, `.eps`, `.vectorcraft` | To samo. Plik z warstwami Illustratora (lub VectorCrafta) pyta, czy układać według koloru, czy według tych warstw. EPS jest czytany jako PostScript (ścieżki; tekst i obrazy nie są czytane); EPS lub plik `.vectorcraft` zapisany przez VectorCraft 0.8 lub nowszy jest czytany razem z obrazami. |
| **Importuj DXF…** | tekstowy `.dxf` | Linie, łuki i krzywe, w milimetrach. Warstwy DXF trafiają na warstwy bez kształtów. |
| **Importuj projekt LightBurn…** | `.lbrn2`, `.lbrn` | Kształty. |
| **Importuj projekt UGS…** | `.ugsd` | Kształty na ich warstwach. Posuwy i głębokości są pomijane. |
| **Importuj wtyczką…** | to, co czyta zainstalowana wtyczka | Zobacz [Wtyczki](Wtyczki). |

**Plik → Otwórz…** i upuszczenie pliku na okno robią **nową kartę** z `.cre`, SVG, PDF lub pliku Illustratora. Plik DXF, LightBurn lub UGS jest importowany do otwartego rysunku.

## Eksport

| Pozycja | Co zapisuje |
|---|---|
| **Eksportuj SVG…** | Wszystkie kształty jako SVG, w rozmiarze strony. Krzywe zostają. |
| **Eksportuj PLT / DXF…** | Obrysy wszystkich kształtów jako HPGL (`.plt`, jeden pisak na warstwę) lub DXF R12, zależnie od wybranego rozszerzenia. |
| **Eksportuj zaznaczenie jako SVG…** / **Eksportuj zaznaczenie jako DXF…** | Zaznaczone kształty (wszystkie, gdy nic nie jest zaznaczone). |

Eksport całego rysunku zapisuje też kształty na ukrytych warstwach. **Obrazy nie mają obrysów i są pomijane**, a program mówi, ile ich jest; najpierw użyj **Wektoryzacja obrazu…**.

Plik PLT nie był jeszcze próbowany na prawdziwym ploterze. Zobacz [Ograniczenia](Ograniczenia).
