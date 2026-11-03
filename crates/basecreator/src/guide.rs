//! The built-in quick guide: short descriptions of the program's options.
//! Written in English and Polish; other interface languages show the English text.
use base_ui::dialogs::Section;
use base_ui::i18n::{lang, Lang};

pub const EN: &[Section] = &[
    Section {
        title: "Getting started",
        lines: &[
            "The start screen has New, Open… and the recent files, with the language and the colour scheme below them. There is no device to choose: New starts a drawing on a page, and File > Page setup… (or a click on the size in the status bar) sets its size and direction.",
            "Open the .cre project in LightCreator, MillCreator or PenCreator to prepare it for a machine, or write it for other programs with File > Export.",
        ],
    },
    Section {
        title: "Layers",
        lines: &[
            "The row of colours under the work area has the 64 layers. Select shapes and click a colour to put them on that layer; a new shape goes to the layer in use. The Layers tab gives a layer a name and a colour and hides it. A hidden layer can be left out when printing.",
        ],
    },
    Section {
        title: "Window and tabs",
        lines: &[
            "Each open drawing has a tab above the work area (File > New and File > Open… make one, up to 10); a dot marks unsaved work. File > Open recent lists the last files, and File > Print… prints the design on paper or saves it as a PDF. Settings > Program options… sets auto-save and the check for updates.",
            "Help > Download backup… saves your settings and your clipart in a zip file; Help > Restore from backup… reads it back. Help > Check for updates… looks for a newer version.",
        ],
    },
    Section {
        title: "Drawing and editing",
        lines: &[
            "The palette on the left: select (V), node edit (N), rectangle (R), ellipse (E), triangle (Y), star (S), polygon (G, asks for the number of sides), heart (K), spiral (I, asks for the number of turns), line (L), polyline (P), text (T), pan (H), zoom in (Z) and zoom out (X). Drag to draw; Shift keeps the proportions. The Insert menu has the same tools.",
            "Click to select, Shift+click to add. A frame from left to right selects what is wholly inside, from right to left also what it touches. The handles resize; a click on a handle changes them to rotate and slant. The arrows move the selection (Shift: 10 mm). Escape clears the selection, twice gives the select tool. Hold Space to pan with any tool.",
            "The Arrange menu has align, flip, rotate, group, lock, order, the operations on shapes (union, intersection, subtract, exclusive or, offset, round corners), arrays, text on a path, nesting and measuring. The right button opens the same commands at the pointer.",
        ],
    },
    Section {
        title: "Text, pictures and import",
        lines: &[
            "File > Import brings in pictures, SVG, Illustrator / PDF, DXF, LightBurn projects and UGS designs. File > Export writes SVG and PLT / DXF. The Clipart tab has ready-made pictures, and Insert > Online clipart… searches the internet; Insert > Generate shape… uses the generators of plugins.",
            "View > Background picture… puts a photo or a plan under the drawing; drag its four corners onto the corners of the page. It is saved with the project.",
        ],
    },
    Section {
        title: "Keyboard shortcuts",
        lines: &[
            "Ctrl+N new, Ctrl+O open, Ctrl+S save, Ctrl+Shift+S save as, Ctrl+P print, Ctrl+Z undo, Ctrl+Y redo, Ctrl+C copy, Ctrl+V paste, Ctrl+D duplicate, Delete delete, Ctrl+A select all, Ctrl+G group, Ctrl+Shift+G ungroup, Ctrl+L lock, Ctrl+Shift+L unlock, F1 this guide.",
        ],
    },
];

const PL: &[Section] = &[
    Section {
        title: "Pierwsze kroki",
        lines: &[
            "Ekran startowy ma Nowy, Otwórz… i ostatnie pliki, a pod nimi język i schemat kolorów. Nie ma urządzenia do wybrania: Nowy zaczyna rysunek na stronie, a Plik > Ustawienia strony… (albo kliknięcie rozmiaru na pasku stanu) ustawiają jej rozmiar i kierunek.",
            "Otwórz projekt .cre w LightCreatorze, MillCreatorze albo PenCreatorze, żeby przygotować go dla maszyny, albo zapisz go dla innych programów przez Plik > Eksport.",
        ],
    },
    Section {
        title: "Warstwy",
        lines: &[
            "Rząd kolorów pod obszarem roboczym to 64 warstwy. Zaznacz kształty i kliknij kolor, żeby przenieść je na tę warstwę; nowy kształt trafia na warstwę w użyciu. Karta Warstwy nadaje warstwie nazwę i kolor oraz ją ukrywa. Ukrytą warstwę można pominąć przy drukowaniu.",
        ],
    },
    Section {
        title: "Okno i karty",
        lines: &[
            "Każdy otwarty rysunek ma kartę nad obszarem roboczym (Plik > Nowy i Plik > Otwórz… tworzą kartę, do 10); kropka oznacza niezapisaną pracę. Plik > Otwórz ostatnie pokazuje ostatnie pliki, a Plik > Drukuj… drukuje projekt na papierze albo zapisuje go jako PDF. Ustawienia > Opcje programu… ustawiają automatyczny zapis i sprawdzanie aktualizacji.",
            "Pomoc > Pobierz kopię zapasową… zapisuje Twoje ustawienia i cliparty w pliku zip, a przywracanie z kopii w menu Pomoc wczytuje je z powrotem. Pomoc > Sprawdź aktualizacje… szuka nowszej wersji.",
        ],
    },
    Section {
        title: "Rysowanie i edycja",
        lines: &[
            "Paleta po lewej: zaznaczanie (V), edycja węzłów (N), prostokąt (R), elipsa (E), trójkąt (Y), gwiazda (S), wielokąt (G, pyta o liczbę boków), serce (K), spirala (I, pyta o liczbę zwojów), linia (L), łamana (P), tekst (T), rączka (H), powiększanie (Z) i pomniejszanie (X). Rysuj przeciągając; Shift utrzymuje proporcje. Menu Wstaw ma te same narzędzia.",
            "Kliknięcie zaznacza, Shift+kliknięcie dodaje. Ramka od lewej do prawej zaznacza to, co jest w niej w całości, od prawej do lewej także to, czego dotyka. Uchwyty zmieniają rozmiar; kliknięcie uchwytu zamienia je na obrót i pochylenie. Strzałki przesuwają zaznaczenie (Shift: 10 mm). Escape czyści zaznaczenie, dwa razy wraca do zaznaczania. Trzymana Spacja daje rączkę przy każdym narzędziu.",
            "Menu Rozmieść ma wyrównanie, odbicia, obroty, grupowanie, blokadę, kolejność, działania na kształtach (suma, część wspólna, różnica, wyłączenie, offset, zaokrąglanie rogów), szyki, tekst na ścieżce, rozkładanie i pomiary. Prawy przycisk otwiera te same polecenia przy wskaźniku.",
        ],
    },
    Section {
        title: "Tekst, obrazy i import",
        lines: &[
            "Plik > Importuj wczytuje obrazy, SVG, Illustrator / PDF, DXF, projekty LightBurn i projekty UGS. Plik > Eksport zapisuje SVG i PLT / DXF. Karta Clipart ma gotowe obrazki, Wstaw > Cliparty z internetu… szuka ich w sieci, a Wstaw > Generuj kształt… używa generatorów z wtyczek.",
            "Widok > Obraz tła… kładzie pod rysunkiem zdjęcie albo plan; przeciągnij jego cztery narożniki na narożniki strony. Jest zapisywany w projekcie.",
        ],
    },
    Section {
        title: "Skróty klawiszowe",
        lines: &[
            "Ctrl+N nowy, Ctrl+O otwórz, Ctrl+S zapisz, Ctrl+Shift+S zapisz jako, Ctrl+P drukuj, Ctrl+Z cofnij, Ctrl+Y ponów, Ctrl+C kopiuj, Ctrl+V wklej, Ctrl+D powiel, Delete usuń, Ctrl+A zaznacz wszystko, Ctrl+G grupuj, Ctrl+Shift+G rozgrupuj, Ctrl+L zablokuj, Ctrl+Shift+L odblokuj, F1 ten przewodnik.",
        ],
    },
];

/// The guide in the language of the interface.
pub fn sections() -> &'static [Section] {
    if lang() == Lang::Pl {
        PL
    } else {
        EN
    }
}
