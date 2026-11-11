//! The quick guide, the same in every program: what every program has (the window, drawing, import, the gallery, the machine, the keys)
//! is here, and a program adds its own sections around it. Written in English and Polish; other interface languages show the English
//! text. `tool_check::check_guide_paths` checks each `Menu > Entry` in both languages in every program.
use crate::dialogs::Section;

/// What the program says about the picture under the drawing.
#[derive(Clone, Copy, PartialEq)]
pub enum Picture {
    /// A photo or the live picture of a camera.
    Camera,
    /// A photo or a plan.
    Background,
}

/// What a program adds to the guide, in one language.
#[derive(Clone, Copy)]
pub struct Own {
    /// Before the shared sections: getting started, the layers, the output.
    pub first: &'static [Section],
    /// After the shared sections, before the keys.
    pub last: &'static [Section],
    /// The section about the machine, for a program that controls one.
    pub machine: bool,
    pub picture: Picture,
}

/// The guide of a program, with the shared sections in place.
pub fn sections(own: Own, polish: bool) -> Vec<Section> {
    let s = if polish { &PL } else { &EN };
    let mut out = own.first.to_vec();
    out.push(s.window);
    out.push(s.drawing);
    out.push(if own.picture == Picture::Camera { s.text_camera } else { s.text_background });
    out.push(s.clipart);
    if own.machine {
        out.push(s.machine);
    }
    out.extend_from_slice(own.last);
    out.push(s.keys);
    out
}

struct Shared {
    window: Section,
    drawing: Section,
    text_camera: Section,
    text_background: Section,
    clipart: Section,
    machine: Section,
    keys: Section,
}

const EN: Shared = Shared {
    window: Section {
        title: "Window and tabs",
        lines: &[
            "The window has its own flat frame: drag the title bar to move it, double-click it to maximise, use the buttons at its right end to minimise, maximise or close, and resize from the edges.",
            "Each open drawing has a tab above the work area (File > New and File > Open… make one, up to 10); a dot marks unsaved work. Ctrl+1 .. Ctrl+9 and Ctrl+0 show the tabs, Ctrl+W closes one. Closing with unsaved work asks, tab by tab. File > Open recent lists the last files, and File > Print… prints the design on paper or saves it as a PDF.",
            "Settings > Program options… sets auto-save and the check for updates. Settings > Language and Settings > Colour scheme change the interface; Settings > View options… sets the grids, the background colour of the work area and the line thickness.",
            "Help > Download backup… saves your settings and your clipart in a zip file; Help > Restore from backup… reads it back. Help > Check for updates… looks for a newer version.",
            "The arrange bar between the work area and the side panel holds align, centre, turn, mirror, group, lock and order commands for the selection; hover an icon for its name. A double arrow at the edge of a bar means more icons are out of sight: scroll to reach them.",
        ],
    },
    drawing: Section {
        title: "Drawing and editing",
        lines: &[
            "The palette on the left: select (V), node edit (N), rectangle (R), ellipse (E), triangle (Y), star (S), polygon (G, asks for the number of sides), heart (K), spiral (I, asks for the number of turns), line (L), polyline (P), text (T), pan (H), zoom in (Z) and zoom out (X). Drag to draw; Shift keeps the proportions. The Insert menu has the same tools.",
            "Click to select, Shift+click to add. A frame from left to right selects what is wholly inside, from right to left also what it touches. The handles resize; a click on a handle changes them to rotate (Shift: every 15°) and slant. The arrows move the selection (Shift: 10 mm). Escape clears the selection, twice gives the select tool. Hold Space to pan with any tool.",
            "The Arrange menu has align, flip, rotate, group, lock, order, the operations on shapes (union, intersection, subtract, exclusive or, offset, round corners), arrays, text on a path, nesting and measuring. The right button opens the same commands at the pointer.",
            "Arrange > Nest shapes… packs the selected shapes into the work area. With By the real outlines a small part goes into the hole of a big one and an L into the corner of another. In a program for a machine, with What does not fit goes on new sheets the rest opens as new drawings in tabs of their own, with the same layers, to save and cut one after another.",
            "Node edit: the palette button (N), Arrange > Edit nodes or a double click on a shape; pictures from the gallery can be edited too, text must be converted to curves first. Drag nodes and Bézier handles, double-click a segment to add a node, Delete removes nodes. The bar on the work area switches nodes between corner and smooth, makes segments straight or curved, opens or closes a path and rounds the selected corners. Keys of the node tool: D delete, I insert a node, L to line, S to curve, C corner, R round, B break the path at the node. A node dropped onto another one joins them: the end of a path on its other end closes it, a node on its neighbour merges with it, an end on the end of another path makes one path.",
            "Round corners replaces sharp straight corners with arcs of the radius you give; with two straight lines selected it joins them with a rounded corner. Too large a radius is limited to what fits.",
            "Locked objects cannot be moved, edited or deleted, but they can be selected and copied; a copy is not locked.",
        ],
    },
    text_camera: Section {
        title: "Text, pictures and import",
        lines: &[
            "Text tool: click on the work area, then type in the Properties tab. Choose the font, size, style and spacing; Convert to curves turns text into editable shapes.",
            "File > Import brings in pictures, SVG, Illustrator / PDF, DXF, LightBurn projects and UGS designs. File > Export writes SVG and PLT / DXF. A picture opens a window with rotate, flip, brightness, contrast, gamma, auto levels, negative and size; Arrange > Adjust image… changes it later, and Arrange > Trace image… turns it into outlines. Negative image is also in Properties of a selected picture.",
            "View > Overlay from… puts a photo under the drawing, or the live picture of the camera when the device has one; drag its four corners onto the material. View > Show camera overlay shows or hides it. It is saved with the project.",
        ],
    },
    text_background: Section {
        title: "Text, pictures and import",
        lines: &[
            "Text tool: click on the work area, then type in the Properties tab. Choose the font, size, style and spacing; Convert to curves turns text into editable shapes.",
            "File > Import brings in pictures, SVG, Illustrator / PDF, DXF, LightBurn projects and UGS designs. File > Export writes SVG and PLT / DXF. A picture opens a window with rotate, flip, brightness, contrast, gamma, auto levels, negative and size; Arrange > Adjust image… changes it later, and Arrange > Trace image… turns it into outlines. Negative image is also in Properties of a selected picture.",
            "View > Background picture… puts a photo or a plan under the drawing; drag its four corners onto the corners of the page. It is saved with the project.",
        ],
    },
    clipart: Section {
        title: "Clipart gallery",
        lines: &[
            "The Clipart tab holds ready-made pictures in categories. A blue dot marks pictures that come with the program, a green dot those you saved or downloaded. Click a picture to place it on the active layer, in the middle of the work area, at the size set in the tab. A picture with several parts is grouped. The arrow on a category's title line folds it up or opens it again.",
            "Online… and Insert > Online clipart… search open icon collections on the internet (Iconify). Click a result to see its collection, licence and author, give it a name and a category and save it; the file is stored on your disk. Orange dots mark pictures whose licence asks you to credit the author. You can also download a picture from a web address, or add SVG files from your computer with Add file…. Insert > Generate shape… uses the generators of plugins.",
            "Right-click a picture of yours to rename it, move it to another category, see its details or delete it. Right-click a shipped picture to save a copy in your own category.",
            "Only save pictures you are allowed to use and check the licence of the source before you sell what you make.",
        ],
    },
    machine: Section {
        title: "Machine",
        lines: &[
            "Machine > Device settings… sets the connection, and Connect on the Device tab opens it. Machine > Switch device… goes back to the start screen. Machine > Firmware settings ($$)… reads and writes the settings of GRBL, and Machine > Calibration… measures the steps of the axes.",
            "Machine > Start from line… starts a job in the middle, and Machine > Check the program runs it in the check mode of GRBL: nothing moves and the lines GRBL does not take are reported.",
            "Machine > Diagnostics and statistics… shows the inputs, the jobs, the alarms and the maintenance reminders; Help > Export diagnostics (zip)… makes one file to attach to a bug report.",
            "The No-go zones page of Machine > Device settings… keeps parts of the work area the tool must never enter, such as a clamp: moves through the air go round them, and a cut that touches one is reported before the job. With the Acceleration of X, Y, Z and the Junction deviation of the machine filled in, the time of a job counts the speeding up, the slowing down and the slower corners, as the controller does.",
        ],
    },
    keys: Section {
        title: "Keyboard shortcuts",
        lines: &[
            "Ctrl+N new, Ctrl+O open, Ctrl+S save, Ctrl+Shift+S save as, Ctrl+I import SVG, Ctrl+P print, Ctrl+Z undo, Ctrl+Y redo, Ctrl+C copy, Ctrl+V paste, Ctrl+D duplicate, Delete delete, Ctrl+A select all, Ctrl+G group, Ctrl+Shift+G ungroup, Ctrl+L lock, Ctrl+Shift+L unlock, F1 this guide.",
            "The group under the tools holds pan (H), zoom in (Z), zoom out (X) and two fit buttons, the work area (Ctrl+Shift+0) and all objects (Ctrl+Shift+9). The mouse wheel zooms and the middle button drags the view; the work area is fitted to the window until you zoom by hand.",
        ],
    },
};

const PL: Shared = Shared {
    window: Section {
        title: "Okno i karty",
        lines: &[
            "Okno ma własną, płaską ramkę: przeciągaj pasek tytułu, by je przesunąć, dwuklik maksymalizuje, przyciski na jego prawym końcu minimalizują, maksymalizują i zamykają, a rozmiar zmieniasz za krawędzie.",
            "Każdy otwarty rysunek ma kartę nad obszarem roboczym (Plik > Nowy i Plik > Otwórz… tworzą kartę, do 10); kropka oznacza niezapisaną pracę. Ctrl+1 … Ctrl+9 i Ctrl+0 pokazują karty, Ctrl+W zamyka kartę. Zamknięcie z niezapisaną pracą pyta, karta po karcie. Plik > Otwórz ostatnie pokazuje ostatnie pliki, a Plik > Drukuj… drukuje projekt na papierze albo zapisuje go jako PDF.",
            "Ustawienia > Opcje programu… ustawiają automatyczny zapis i sprawdzanie aktualizacji. Ustawienia > Język i Ustawienia > Schemat kolorów zmieniają interfejs; Ustawienia > Opcje widoku… ustawiają siatki, kolor tła obszaru roboczego i grubość linii.",
            "Pomoc > Pobierz kopię zapasową… zapisuje Twoje ustawienia i cliparty w pliku zip, a przywracanie z kopii w menu Pomoc wczytuje je z powrotem. Pomoc > Sprawdź aktualizacje… szuka nowszej wersji.",
            "Pasek wyrównania między obszarem roboczym a panelem bocznym zawiera polecenia wyrównania, środkowania, obrotu, odbicia, grupowania, blokowania i kolejności dla zaznaczenia; nazwę ikony pokaże podpowiedź. Podwójna strzałka na brzegu paska oznacza, że dalsze ikony są poza widokiem: przewiń, by je zobaczyć.",
        ],
    },
    drawing: Section {
        title: "Rysowanie i edycja",
        lines: &[
            "Paleta po lewej: zaznaczanie (V), edycja węzłów (N), prostokąt (R), elipsa (E), trójkąt (Y), gwiazda (S), wielokąt (G, pyta o liczbę boków), serce (K), spirala (I, pyta o liczbę zwojów), linia (L), łamana (P), tekst (T), rączka (H), powiększanie (Z) i pomniejszanie (X). Rysuj przeciągając; Shift utrzymuje proporcje. Menu Wstaw ma te same narzędzia.",
            "Kliknięcie zaznacza, Shift+kliknięcie dodaje. Ramka od lewej do prawej zaznacza to, co jest w niej w całości, od prawej do lewej także to, czego dotyka. Uchwyty zmieniają rozmiar; kliknięcie uchwytu zamienia je na obrót (Shift: co 15°) i pochylenie. Strzałki przesuwają zaznaczenie (Shift: 10 mm). Escape czyści zaznaczenie, dwa razy wraca do zaznaczania. Trzymana Spacja daje rączkę przy każdym narzędziu.",
            "Menu Rozmieść ma wyrównanie, odbicia, obroty, grupowanie, blokadę, kolejność, działania na kształtach (suma, część wspólna, różnica, wyłączenie, offset, zaokrąglanie rogów), szyki, tekst na ścieżce, rozkładanie i pomiary. Prawy przycisk otwiera te same polecenia przy wskaźniku.",
            "Rozmieść > Rozmieść kształty (nesting)… układa zaznaczone kształty w obszarze roboczym. Z opcją Według prawdziwych obrysów mała część trafia w otwór dużej, a L w róg innej. W programie dla maszyny z opcją Co się nie mieści, idzie na nowe arkusze reszta otwiera się jako nowe rysunki na osobnych kartach, z tymi samymi warstwami, do zapisania i wycięcia po kolei.",
            "Edycja węzłów: przycisk na palecie (N) albo dwukrotne kliknięcie kształtu; cliparty z galerii też można edytować, a tekst trzeba najpierw zamienić na krzywe. Przeciągaj węzły i uchwyty Béziera, dwukrotne kliknięcie odcinka dodaje węzeł, Delete usuwa węzły. Pasek na obszarze roboczym przełącza węzły między narożnikiem a gładkim, robi odcinki prostymi lub krzywymi, otwiera lub zamyka ścieżkę i zaokrągla zaznaczone rogi. Klawisze edycji węzłów: D usuwa, I wstawia węzeł, L na linię, S na krzywą, C narożnik, R zaokrągla, B rozdziela ścieżkę w węźle. Węzeł upuszczony na inny łączy się z nim: koniec ścieżki na jej drugim końcu ją zamyka, węzeł na sąsiednim zlewa się z nim, a koniec na końcu innej ścieżki tworzy jedną ścieżkę.",
            "Zaokrąglanie rogów zamienia ostre proste narożniki na łuki o podanym promieniu; przy zaznaczonych dwóch prostych łączy je zaokrąglonym narożnikiem. Zbyt duży promień jest ograniczany do tego, co się mieści.",
            "Zablokowanych obiektów nie można przesuwać, edytować ani usuwać, ale można je zaznaczać i kopiować; kopia nie jest zablokowana.",
        ],
    },
    text_camera: Section {
        title: "Tekst, obrazy i import",
        lines: &[
            "Narzędzie tekstu: kliknij na obszarze roboczym i wpisz tekst na karcie Właściwości. Wybierz czcionkę, rozmiar, styl i odstępy; Zamień na krzywe robi z tekstu edytowalne kształty.",
            "Plik > Importuj wczytuje obrazy, SVG, Illustrator / PDF, DXF, projekty LightBurn i projekty UGS. Plik > Eksportuj zapisuje SVG i PLT / DXF. Obraz otwiera okno z obrotem, odbiciem, jasnością, kontrastem, gammą, automatycznymi poziomami, negatywem i rozmiarem; dostosowanie i trasowanie obrazu są później w menu Rozmieść. Obraz negatywowy jest też we Właściwościach zaznaczonego obrazu.",
            "Widok > Nakładka z… kładzie pod rysunkiem zdjęcie albo obraz na żywo z kamery, gdy urządzenie ją ma; przeciągnij jego cztery narożniki na materiał. Widok > Pokaż nakładkę z kamery pokazuje ją lub ukrywa. Jest zapisywana w projekcie.",
        ],
    },
    text_background: Section {
        title: "Tekst, obrazy i import",
        lines: &[
            "Narzędzie tekstu: kliknij na obszarze roboczym i wpisz tekst na karcie Właściwości. Wybierz czcionkę, rozmiar, styl i odstępy; Zamień na krzywe robi z tekstu edytowalne kształty.",
            "Plik > Importuj wczytuje obrazy, SVG, Illustrator / PDF, DXF, projekty LightBurn i projekty UGS. Plik > Eksportuj zapisuje SVG i PLT / DXF. Obraz otwiera okno z obrotem, odbiciem, jasnością, kontrastem, gammą, automatycznymi poziomami, negatywem i rozmiarem; dostosowanie i trasowanie obrazu są później w menu Rozmieść. Obraz negatywowy jest też we Właściwościach zaznaczonego obrazu.",
            "Widok > Obraz w tle… kładzie zdjęcie albo plan pod rysunkiem; przeciągnij jego cztery narożniki na narożniki strony. Jest zapisywany w projekcie.",
        ],
    },
    clipart: Section {
        title: "Galeria clipartów",
        lines: &[
            "Karta Clipart zawiera gotowe obrazki w kategoriach. Niebieska kropka oznacza obrazki dostarczone z programem, zielona obrazki zapisane lub pobrane przez Ciebie. Kliknij obrazek, aby umieścić go na aktywnej warstwie, na środku obszaru roboczego, w rozmiarze ustawionym na karcie. Obrazek z wieloma częściami jest grupowany. Strzałka w linii tytułu kategorii zwija ją lub rozwija.",
            "Z internetu… i Wstaw > Cliparty z internetu… przeszukują otwarte kolekcje ikon (Iconify). Kliknij wynik, aby zobaczyć kolekcję, licencję i autora, nadaj mu nazwę i kategorię i zapisz; plik trafia na Twój dysk. Pomarańczowa kropka oznacza obrazki, których licencja wymaga podania autora. Możesz też pobrać obrazek spod adresu internetowego albo dodać pliki SVG z komputera przyciskiem Dodaj plik…. Wstaw > Generuj kształt… używa generatorów z wtyczek.",
            "Kliknij prawym przyciskiem własny obrazek, aby zmienić jego nazwę, przenieść do innej kategorii, zobaczyć szczegóły lub go usunąć. Prawy przycisk na obrazku z zestawu zapisuje jego kopię w Twojej kategorii.",
            "Zapisuj tylko obrazki, z których wolno Ci korzystać, i sprawdź licencję źródła, zanim zaczniesz sprzedawać wykonane prace.",
        ],
    },
    machine: Section {
        title: "Maszyna",
        lines: &[
            "Maszyna > Ustawienia urządzenia… ustawiają połączenie, a Połącz na karcie Urządzenie je otwiera. Maszyna > Zmień urządzenie… wraca do ekranu startowego. Maszyna > Ustawienia firmware ($$)… czytają i zapisują ustawienia GRBL, a Maszyna > Kalibracja… mierzy kroki osi.",
            "Maszyna > Start od wiersza… zaczyna zadanie od środka, a Maszyna > Sprawdź program przepuszcza je przez tryb sprawdzania GRBL: nic się nie rusza, a wiersze, których GRBL nie przyjmuje, są zgłaszane.",
            "Maszyna > Diagnostyka i statystyki… pokazują wejścia, zadania, alarmy i przypomnienia o przeglądach; Pomoc > Eksportuj diagnostykę (zip)… tworzy jeden plik do dołączenia do zgłoszenia błędu.",
            "Strona Strefy zakazane w Maszyna > Ustawienia urządzenia… przechowuje części obszaru roboczego, w które narzędzie nie może wjechać, np. docisk: ruchy w powietrzu je omijają, a cięcie, które którejś dotyka, jest zgłaszane przed zadaniem. Gdy Przyspieszenie X, Y, Z i Odchyłka w narożnikach maszyny są wpisane, czas zadania uwzględnia rozpędzanie, hamowanie i wolniejsze narożniki, tak jak sterownik.",
        ],
    },
    keys: Section {
        title: "Skróty klawiszowe",
        lines: &[
            "Ctrl+N nowy, Ctrl+O otwórz, Ctrl+S zapisz, Ctrl+Shift+S zapisz jako, Ctrl+I import SVG, Ctrl+P drukuj, Ctrl+Z cofnij, Ctrl+Y ponów, Ctrl+C kopiuj, Ctrl+V wklej, Ctrl+D powiel, Delete usuń, Ctrl+A zaznacz wszystko, Ctrl+G grupuj, Ctrl+Shift+G rozgrupuj, Ctrl+L zablokuj, Ctrl+Shift+L odblokuj, F1 ten przewodnik.",
            "Grupa pod narzędziami zawiera rączkę (H), powiększanie (Z), pomniejszanie (X) oraz dwa przyciski dopasowania: do obszaru roboczego (Ctrl+Shift+0) i do wszystkich obiektów (Ctrl+Shift+9). Kółko myszy zbliża, środkowy przycisk przesuwa widok; obszar roboczy jest dopasowany do okna, dopóki nie zmienisz zbliżenia ręcznie.",
        ],
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    const FIRST: &[Section] = &[Section { title: "Own", lines: &["a"] }];
    const LAST: &[Section] = &[Section { title: "Safety", lines: &["b"] }];

    #[test]
    fn the_shared_sections_come_between_those_of_the_program_and_the_keys_are_last() {
        let own = Own { first: FIRST, last: LAST, machine: true, picture: Picture::Camera };
        let titles: Vec<&str> = sections(own, false).iter().map(|s| s.title).collect();
        assert_eq!(titles, ["Own", "Window and tabs", "Drawing and editing", "Text, pictures and import", "Clipart gallery", "Machine", "Safety", "Keyboard shortcuts"]);
        let without = Own { machine: false, picture: Picture::Background, ..own };
        let plain = sections(without, true);
        assert!(!plain.iter().any(|s| s.title == "Maszyna"), "no machine, no section");
        assert!(plain.iter().flat_map(|s| s.lines).any(|l| l.contains("Widok > Obraz w tle…")));
    }

    #[test]
    fn the_two_languages_have_the_same_number_of_lines_in_each_section() {
        for (e, p) in [(EN.window, PL.window), (EN.drawing, PL.drawing), (EN.text_camera, PL.text_camera), (EN.text_background, PL.text_background), (EN.clipart, PL.clipart), (EN.machine, PL.machine), (EN.keys, PL.keys)] {
            assert_eq!(e.lines.len(), p.lines.len(), "{}", e.title);
        }
    }
}
