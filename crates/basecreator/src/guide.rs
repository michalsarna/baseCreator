//! The quick guide of BaseCreator: what is its own. The rest is the shared guide (`base_ui::guide`).
use base_ui::dialogs::Section;
use base_ui::guide::{Own, Picture};
use base_ui::i18n::{lang, Lang};

const EN: &[Section] = &[
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
];

const PL: &[Section] = &[
    Section {
        title: "Pierwsze kroki",
        lines: &[
            "Ekran startowy ma Nowy, Otwórz… i ostatnie pliki, a pod nimi język i schemat kolorów. Nie ma urządzenia do wybrania: Nowy zaczyna rysunek na stronie, a Plik > Ustawienia strony… (albo kliknięcie rozmiaru na pasku stanu) ustawiają jej rozmiar i kierunek.",
            "Otwórz projekt .cre w LightCreatorze, MillCreatorze albo PenCreatorze, żeby przygotować go dla maszyny, albo zapisz go dla innych programów przez Plik > Eksportuj.",
        ],
    },
    Section {
        title: "Warstwy",
        lines: &[
            "Rząd kolorów pod obszarem roboczym to 64 warstwy. Zaznacz kształty i kliknij kolor, żeby przenieść je na tę warstwę; nowy kształt trafia na warstwę w użyciu. Karta Warstwy nadaje warstwie nazwę i kolor oraz ją ukrywa. Ukrytą warstwę można pominąć przy drukowaniu.",
        ],
    },
];

/// The guide in English or Polish.
pub fn sections_in(polish: bool) -> Vec<Section> {
    let first = if polish { PL } else { EN };
    base_ui::guide::sections(Own { first, last: &[], machine: false, picture: Picture::Background }, polish)
}

/// The guide in the language of the interface.
pub fn sections() -> Vec<Section> {
    sections_in(lang() == Lang::Pl)
}
