//! The texts of the shared interface (node bar, selection commands): English, six languages in the table, and the
//! other 23 in the files next to this one. Generated from the texts of LightCreator; the test of this file checks that
//! every text used by `tr` in this crate has all its translations.
use crate::i18n::{Texts, TABLE_LANGS};

#[rustfmt::skip]
pub const TABLE: &[[&str; TABLE_LANGS]] = &[
    ["Add node", "Dodaj węzeł", "Knoten hinzufügen", "Aggiungi nodo", "Lisää solmu", "添加节点", "नोड जोड़ें"],
    ["Close an open path, or open a closed one at the selected node", "Zamknij otwartą ścieżkę lub otwórz zamkniętą w zaznaczonym węźle", "Offenen Pfad schließen oder geschlossenen am ausgewählten Knoten öffnen", "Chiudi un tracciato aperto o apri uno chiuso al nodo selezionato", "Sulje avoin polku tai avaa suljettu valitun solmun kohdalta", "闭合开放路径，或在所选节点处断开闭合路径", "खुले पाथ को बंद करें या बंद पाथ को चयनित नोड पर खोलें"],
    ["Convert text to curves, or trace the image, to edit its nodes.", "Zamień tekst na krzywe albo wektoryzuj obraz, aby edytować jego węzły.", "Text in Kurven umwandeln oder das Bild nachzeichnen, um die Knoten zu bearbeiten.", "Converti il testo in curve o vettorializza l'immagine per modificarne i nodi.", "Muunna teksti käyriksi tai jäljitä kuva muokataksesi sen solmuja.", "将文字转换为曲线或描摹图像后才能编辑其节点。", "नोड संपादित करने के लिए टेक्स्ट को वक्रों में बदलें या छवि को ट्रेस करें।"],
    ["Corner", "Narożnik", "Ecke", "Angolo", "Kulma", "尖角", "कोना"],
    ["Delete node", "Usuń węzeł", "Knoten löschen", "Elimina nodo", "Poista solmu", "删除节点", "नोड हटाएँ"],
    ["Grouped. The objects keep their layers.", "Zgrupowano. Obiekty zachowują swoje warstwy.", "Gruppiert. Die Objekte behalten ihre Ebenen.", "Raggruppati. Gli oggetti mantengono i loro livelli.", "Ryhmitetty. Kohteet säilyttävät tasonsa.", "已编组。对象保留各自的图层。", "समूह बना। ऑब्जेक्ट अपनी लेयर बनाए रखते हैं।"],
    ["Insert a node (or double-click a segment)", "Wstaw węzeł (lub kliknij dwukrotnie odcinek)", "Knoten einfügen (oder Segment doppelklicken)", "Inserisci un nodo (o doppio clic su un segmento)", "Lisää solmu (tai kaksoisnapsauta janaa)", "插入节点（或双击线段）", "नोड डालें (या खंड पर डबल-क्लिक करें)"],
    ["Locked {} object(s).", "Zablokowano obiektów: {}.", "{} Objekt(e) gesperrt.", "Bloccati {} oggetti.", "Lukittu {} kohdetta.", "已锁定 {} 个对象。", "{} ऑब्जेक्ट लॉक किए गए।"],
    ["Make the segment after each selected node a curve", "Zamień odcinek za każdym zaznaczonym węzłem na krzywą", "Das Segment nach jedem ausgewählten Knoten zur Kurve machen", "Rendi curvo il segmento dopo ogni nodo selezionato", "Tee jokaisen valitun solmun jälkeinen jana käyräksi", "将每个所选节点之后的线段设为曲线", "प्रत्येक चयनित नोड के बाद का खंड वक्र करें"],
    ["Make the segment after each selected node straight", "Zamień odcinek za każdym zaznaczonym węzłem na prosty", "Das Segment nach jedem ausgewählten Knoten gerade machen", "Rendi retto il segmento dopo ogni nodo selezionato", "Tee jokaisen valitun solmun jälkeinen jana suoraksi", "将每个所选节点之后的线段设为直线", "प्रत्येक चयनित नोड के बाद का खंड सीधा करें"],
    ["Make the selected nodes corners", "Zamień zaznaczone węzły na narożniki", "Ausgewählte Knoten zu Ecken machen", "Rendi angoli i nodi selezionati", "Muuta valitut solmut kulmiksi", "将所选节点设为尖角", "चयनित नोड को कोना बनाएँ"],
    ["Make the selected nodes smooth", "Zamień zaznaczone węzły na gładkie", "Ausgewählte Knoten glatt machen", "Rendi morbidi i nodi selezionati", "Muuta valitut solmut pehmeiksi", "将所选节点设为平滑", "चयनित नोड को चिकना बनाएँ"],
    ["Open / close", "Otwórz / zamknij", "Öffnen / schließen", "Apri / chiudi", "Avaa / sulje", "断开 / 闭合", "खोलें / बंद करें"],
    ["Radius for rounding", "Promień zaokrąglenia", "Radius zum Abrunden", "Raggio di arrotondamento", "Pyöristyksen säde", "圆角半径", "गोल करने की त्रिज्या"],
    ["Round", "Zaokrąglij", "Abrunden", "Arrotonda", "Pyöristä", "圆角", "गोल करें"],
    ["Round the selected corners with this radius", "Zaokrąglij zaznaczone rogi tym promieniem", "Ausgewählte Ecken mit diesem Radius abrunden", "Arrotonda gli angoli selezionati con questo raggio", "Pyöristä valitut kulmat tällä säteellä", "用此半径为所选角倒圆", "चयनित कोनों को इस त्रिज्या से गोल करें"],
    ["Rounded {} corner(s).", "Zaokrąglono rogów: {}.", "{} Ecke(n) abgerundet.", "Arrotondati {} angoli.", "Pyöristetty {} kulmaa.", "已圆角处理 {} 个角。", "{} कोने गोल किए गए।"],
    ["Select a shape first, then edit its nodes.", "Najpierw zaznacz kształt, potem edytuj jego węzły.", "Zuerst eine Form auswählen, dann ihre Knoten bearbeiten.", "Seleziona prima una forma, poi modifica i suoi nodi.", "Valitse ensin muoto ja muokkaa sitten sen solmuja.", "请先选择形状，再编辑其节点。", "पहले एक आकृति चुनें, फिर उसके नोड संपादित करें।"],
    ["Select at least two objects to group.", "Zaznacz co najmniej dwa obiekty do zgrupowania.", "Wähle mindestens zwei Objekte zum Gruppieren.", "Seleziona almeno due oggetti da raggruppare.", "Valitse vähintään kaksi ryhmitettävää kohdetta.", "请至少选择两个对象进行编组。", "समूह बनाने के लिए कम से कम दो ऑब्जेक्ट चुनें।"],
    ["Smooth", "Gładki", "Glatt", "Morbido", "Pehmeä", "平滑", "चिकना"],
    ["There are no straight corners to round.", "Brak prostych narożników do zaokrąglenia.", "Es gibt keine geraden Ecken zum Abrunden.", "Non ci sono angoli retti da arrotondare.", "Pyöristettäviä suoria kulmia ei ole.", "没有可倒圆的直线角。", "गोल करने के लिए कोई सीधा कोना नहीं है।"],
    ["To curve", "Na krzywą", "Zu Kurve", "In curva", "Käyräksi", "转为曲线", "वक्र में"],
    ["To line", "Na linię", "Zu Linie", "In linea", "Viivaksi", "转为直线", "रेखा में"],
    ["Unlocked {} object(s).", "Odblokowano obiektów: {}.", "{} Objekt(e) entsperrt.", "Sbloccati {} oggetti.", "Lukitus avattu {} kohteelta.", "已解锁 {} 个对象。", "{} ऑब्जेक्ट अनलॉक किए गए।"],
];

pub const KEYS: &str = include_str!("texts/keys.txt");

pub static TEXTS: Texts = Texts::new(TABLE, KEYS, [
    include_str!("texts/bg.txt"),
    include_str!("texts/ca.txt"),
    include_str!("texts/hr.txt"),
    include_str!("texts/cs.txt"),
    include_str!("texts/da.txt"),
    include_str!("texts/nl.txt"),
    include_str!("texts/et.txt"),
    include_str!("texts/fr.txt"),
    include_str!("texts/el.txt"),
    include_str!("texts/hu.txt"),
    include_str!("texts/is.txt"),
    include_str!("texts/ga.txt"),
    include_str!("texts/lv.txt"),
    include_str!("texts/lt.txt"),
    include_str!("texts/mt.txt"),
    include_str!("texts/nb.txt"),
    include_str!("texts/pt.txt"),
    include_str!("texts/ro.txt"),
    include_str!("texts/sk.txt"),
    include_str!("texts/sl.txt"),
    include_str!("texts/es.txt"),
    include_str!("texts/sv.txt"),
    include_str!("texts/uk.txt"),

]);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;

    /// The sources that call `tr` or `trf`.
    const SOURCES: [&str; 3] = [include_str!("editor/host.rs"), include_str!("editor/commands.rs"), include_str!("editor/canvas.rs")];

    fn literals() -> Vec<String> {
        let mut out = vec![];
        for src in SOURCES {
            for call in ["tr(\"", "trf(\""] {
                for part in src.split(call).skip(1) {
                    let end = part.find('"').unwrap();
                    out.push(part[..end].to_string());
                }
            }
        }
        out
    }

    #[test]
    fn every_text_the_shared_ui_shows_is_in_the_table_with_every_language() {
        let lits = literals();
        assert!(lits.len() >= 20, "the scan finds the texts: {}", lits.len());
        for s in &lits {
            let row = TABLE.iter().find(|r| r[0] == s).unwrap_or_else(|| panic!("no row for {s:?}"));
            assert!(row.iter().all(|t| !t.is_empty()), "an empty translation of {s:?}");
            let n = s.matches("{}").count();
            assert!(row.iter().all(|t| t.matches("{}").count() == n), "placeholders of {s:?}");
        }
        let english: Vec<&str> = KEYS.lines().filter_map(|l| l.split_once('\t')).map(|(_, t)| t).collect();
        for s in &lits {
            assert!(english.contains(&s.as_str()), "{s:?} is missing in keys.txt");
        }
        for (n, file) in TEXTS.files().iter().enumerate() {
            let code = Lang::ALL[TABLE_LANGS + n].code();
            let ids: Vec<usize> = file.lines().filter_map(|l| l.split_once('|')).filter_map(|(i, _)| i.parse().ok()).collect();
            assert_eq!(ids.len(), english.len(), "{code}: every text needs a translation");
            for (line, key) in file.lines().zip(&english) {
                let text = line.split_once('|').unwrap().1;
                assert_eq!(text.matches("{}").count(), key.matches("{}").count(), "{code}: placeholders of {key:?}");
            }
        }
    }

    #[test]
    fn the_texts_translate() {
        let fr = Lang::Fr.index() as usize;
        assert_eq!(TEXTS.lookup(fr, "Smooth").map(|s| !s.is_empty()), Some(true));
        let pl = Lang::Pl.index() as usize;
        assert_eq!(TEXTS.lookup(pl, "Round"), Some(TABLE.iter().find(|r| r[0] == "Round").unwrap()[1]));
    }
}
