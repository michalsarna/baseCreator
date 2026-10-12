//! The texts of the shared interface (node bar, selection commands): English, six languages in the table, and the
//! other 23 in the files next to this one. Generated from the texts of LightCreator; the test of this file checks that
//! every text used by `tr` in this crate has all its translations.
use crate::i18n::{Texts, TABLE_LANGS};

#[rustfmt::skip]
pub const TABLE: &[[&str; TABLE_LANGS]] = &[
    ["(empty)", "(pusto)", "(leer)", "(vuoto)", "(tyhjä)", "（空）", "(खाली)"],
    ["Add node", "Dodaj węzeł", "Knoten hinzufügen", "Aggiungi nodo", "Lisää solmu", "添加节点", "नोड जोड़ें"],
    ["Align bottom", "Wyrównaj do dołu", "Unten ausrichten", "Allinea in basso", "Tasaa alas", "底部对齐", "नीचे संरेखित करें"],
    ["Align centre (H)", "Wyśrodkuj w poziomie", "Horizontal zentrieren", "Centra orizzontalmente", "Keskitä vaakasuunnassa", "水平居中", "क्षैतिज केंद्रित करें"],
    ["Align centre (V)", "Wyśrodkuj w pionie", "Vertikal zentrieren", "Centra verticalmente", "Keskitä pystysuunnassa", "垂直居中", "ऊर्ध्वाधर केंद्रित करें"],
    ["Align left", "Wyrównaj do lewej", "Links ausrichten", "Allinea a sinistra", "Tasaa vasemmalle", "左对齐", "बाएँ संरेखित करें"],
    ["Align right", "Wyrównaj do prawej", "Rechts ausrichten", "Allinea a destra", "Tasaa oikealle", "右对齐", "दाएँ संरेखित करें"],
    ["Align top", "Wyrównaj do góry", "Oben ausrichten", "Allinea in alto", "Tasaa ylös", "顶部对齐", "ऊपर संरेखित करें"],
    ["Auto-fit work area to window", "Automatycznie dopasuj obszar roboczy do okna", "Arbeitsbereich automatisch an Fenster anpassen", "Adatta automaticamente l'area di lavoro alla finestra", "Sovita työalue ikkunaan automaattisesti", "自动使工作区域适应窗口", "कार्य क्षेत्र को विंडो में स्वतः फ़िट करें"],
    ["Bring to front", "Przesuń na wierzch", "In den Vordergrund", "Porta in primo piano", "Tuo eteen", "置于顶层", "सबसे आगे लाएँ"],
    ["Centre on bed", "Wyśrodkuj na stole", "Auf Arbeitsfläche zentrieren", "Centra sul piano", "Keskitä alustalle", "在工作台居中", "बेड पर केंद्रित करें"],
    ["Centre on each other", "Wyśrodkuj względem siebie", "Aufeinander zentrieren", "Centra l'uno sull'altro", "Keskitä toistensa suhteen", "相互居中", "एक-दूसरे के केंद्र पर रखें"],
    ["Clear list", "Wyczyść listę", "Liste leeren", "Svuota elenco", "Tyhjennä luettelo", "清除列表", "सूची साफ़ करें"],
    ["Close", "Zamknij", "Schließen", "Chiudi", "Sulje", "关闭", "बंद करें"],
    ["Close an open path, or open a closed one at the selected node", "Zamknij otwartą ścieżkę lub otwórz zamkniętą w zaznaczonym węźle", "Offenen Pfad schließen oder geschlossenen am ausgewählten Knoten öffnen", "Chiudi un tracciato aperto o apri uno chiuso al nodo selezionato", "Sulje avoin polku tai avaa suljettu valitun solmun kohdalta", "闭合开放路径，或在所选节点处断开闭合路径", "खुले पाथ को बंद करें या बंद पाथ को चयनित नोड पर खोलें"],
    ["Convert text to curves, or trace the image, to edit its nodes.", "Zamień tekst na krzywe albo wektoryzuj obraz, aby edytować jego węzły.", "Text in Kurven umwandeln oder das Bild nachzeichnen, um die Knoten zu bearbeiten.", "Converti il testo in curve o vettorializza l'immagine per modificarne i nodi.", "Muunna teksti käyriksi tai jäljitä kuva muokataksesi sen solmuja.", "将文字转换为曲线或描摹图像后才能编辑其节点。", "नोड संपादित करने के लिए टेक्स्ट को वक्रों में बदलें या छवि को ट्रेस करें।"],
    ["Convert to path", "Zamień na ścieżkę", "In Pfad umwandeln", "Converti in tracciato", "Muunna poluksi", "转换为路径", "पाथ में बदलें"],
    ["Copy", "Kopiuj", "Kopieren", "Copia", "Kopioi", "复制", "कॉपी करें"],
    ["Corner", "Narożnik", "Ecke", "Angolo", "Kulma", "尖角", "कोना"],
    ["Delete", "Usuń", "Löschen", "Elimina", "Poista", "删除", "हटाएँ"],
    ["Delete node", "Usuń węzeł", "Knoten löschen", "Elimina nodo", "Poista solmu", "删除节点", "नोड हटाएँ"],
    ["Duplicate", "Powiel", "Duplizieren", "Duplica", "Monista", "复制副本", "डुप्लिकेट"],
    ["Edit nodes", "Edytuj węzły", "Knoten bearbeiten", "Modifica nodi", "Muokkaa solmuja", "编辑节点", "नोड संपादित करें"],
    ["Ellipse (E)", "Elipsa (E)", "Ellipse (E)", "Ellisse (E)", "Ellipsi (E)", "椭圆 (E)", "दीर्घवृत्त (E)"],
    ["Fit all objects", "Dopasuj do wszystkich obiektów", "Alle Objekte einpassen", "Adatta a tutti gli oggetti", "Sovita kaikki kohteet", "适应所有对象", "सभी ऑब्जेक्ट फ़िट करें"],
    ["Fit all objects (Ctrl+9)", "Dopasuj do wszystkich obiektów (Ctrl+9)", "Alle Objekte einpassen (Strg+9)", "Adatta a tutti gli oggetti (Ctrl+9)", "Sovita kaikki kohteet (Ctrl+9)", "适应所有对象 (Ctrl+9)", "सभी ऑब्जेक्ट फ़िट करें (Ctrl+9)"],
    ["Fit bed to window", "Dopasuj stół do okna", "Arbeitsfläche einpassen", "Adatta il piano alla finestra", "Sovita alusta ikkunaan", "适应窗口", "बेड को विंडो में फिट करें"],
    ["Fit work area (Ctrl+0)", "Dopasuj obszar roboczy (Ctrl+0)", "Arbeitsbereich einpassen (Strg+0)", "Adatta l'area di lavoro (Ctrl+0)", "Sovita työalue (Ctrl+0)", "适应工作区域 (Ctrl+0)", "कार्य क्षेत्र फ़िट करें (Ctrl+0)"],
    ["Flip horizontal", "Odbij w poziomie", "Horizontal spiegeln", "Rifletti orizzontalmente", "Käännä vaakasuunnassa", "水平翻转", "क्षैतिज पलटें"],
    ["Flip vertical", "Odbij w pionie", "Vertikal spiegeln", "Rifletti verticalmente", "Käännä pystysuunnassa", "垂直翻转", "ऊर्ध्वाधर पलटें"],
    ["Grid", "Siatka", "Raster", "Griglia", "Ruudukko", "网格", "ग्रिड"],
    ["Group", "Grupuj", "Gruppieren", "Raggruppa", "Ryhmitä", "编组", "समूह बनाएँ"],
    ["Grouped. The objects keep their layers.", "Zgrupowano. Obiekty zachowują swoje warstwy.", "Gruppiert. Die Objekte behalten ihre Ebenen.", "Raggruppati. Gli oggetti mantengono i loro livelli.", "Ryhmitetty. Kohteet säilyttävät tasonsa.", "已编组。对象保留各自的图层。", "समूह बना। ऑब्जेक्ट अपनी लेयर बनाए रखते हैं।"],
    ["Heart (K)", "Serce (K)", "Herz (K)", "Cuore (K)", "Sydän (K)", "心形 (K)", "दिल (K)"],
    ["Insert a node (or double-click a segment)", "Wstaw węzeł (lub kliknij dwukrotnie odcinek)", "Knoten einfügen (oder Segment doppelklicken)", "Inserisci un nodo (o doppio clic su un segmento)", "Lisää solmu (tai kaksoisnapsauta janaa)", "插入节点（或双击线段）", "नोड डालें (या खंड पर डबल-क्लिक करें)"],
    ["Line (L)", "Linia (L)", "Linie (L)", "Linea (L)", "Viiva (L)", "直线 (L)", "रेखा (L)"],
    ["Lock", "Zablokuj", "Sperren", "Blocca", "Lukitse", "锁定", "लॉक करें"],
    ["Locked {} object(s).", "Zablokowano obiektów: {}.", "{} Objekt(e) gesperrt.", "Bloccati {} oggetti.", "Lukittu {} kohdetta.", "已锁定 {} 个对象。", "{} ऑब्जेक्ट लॉक किए गए।"],
    ["Make the segment after each selected node a curve", "Zamień odcinek za każdym zaznaczonym węzłem na krzywą", "Das Segment nach jedem ausgewählten Knoten zur Kurve machen", "Rendi curvo il segmento dopo ogni nodo selezionato", "Tee jokaisen valitun solmun jälkeinen jana käyräksi", "将每个所选节点之后的线段设为曲线", "प्रत्येक चयनित नोड के बाद का खंड वक्र करें"],
    ["Make the segment after each selected node straight", "Zamień odcinek za każdym zaznaczonym węzłem na prosty", "Das Segment nach jedem ausgewählten Knoten gerade machen", "Rendi retto il segmento dopo ogni nodo selezionato", "Tee jokaisen valitun solmun jälkeinen jana suoraksi", "将每个所选节点之后的线段设为直线", "प्रत्येक चयनित नोड के बाद का खंड सीधा करें"],
    ["Make the selected nodes corners", "Zamień zaznaczone węzły na narożniki", "Ausgewählte Knoten zu Ecken machen", "Rendi angoli i nodi selezionati", "Muuta valitut solmut kulmiksi", "将所选节点设为尖角", "चयनित नोड को कोना बनाएँ"],
    ["Make the selected nodes smooth", "Zamień zaznaczone węzły na gładkie", "Ausgewählte Knoten glatt machen", "Rendi morbidi i nodi selezionati", "Muuta valitut solmut pehmeiksi", "将所选节点设为平滑", "चयनित नोड को चिकना बनाएँ"],
    ["Maximise", "Maksymalizuj", "Maximieren", "Ingrandisci", "Suurenna", "最大化", "बड़ा करें"],
    ["Minimise", "Minimalizuj", "Minimieren", "Riduci a icona", "Pienennä", "最小化", "छोटा करें"],
    ["Node edit (N)", "Edycja węzłów (N)", "Knotenbearbeitung (N)", "Modifica nodi (N)", "Solmujen muokkaus (N)", "节点编辑 (N)", "नोड संपादन (N)"],
    ["Open / close", "Otwórz / zamknij", "Öffnen / schließen", "Apri / chiudi", "Avaa / sulje", "断开 / 闭合", "खोलें / बंद करें"],
    ["Open recent", "Otwórz ostatnie", "Zuletzt geöffnet", "Apri recenti", "Avaa viimeisimmät", "最近打开", "हाल की फ़ाइलें खोलें"],
    ["Pan (H)", "Przesuwanie widoku (H)", "Verschieben (H)", "Sposta vista (H)", "Panoroi (H)", "平移 (H)", "पैन (H)"],
    ["Paste", "Wklej", "Einfügen", "Incolla", "Liitä", "粘贴", "चिपकाएँ"],
    ["Polygon (G)", "Wielokąt (G)", "Vieleck (G)", "Poligono (G)", "Monikulmio (G)", "多边形 (G)", "बहुभुज (G)"],
    ["Polyline / pen (P)", "Linia łamana / pióro (P)", "Polylinie / Stift (P)", "Polilinea / penna (P)", "Murtoviiva / kynä (P)", "折线 / 钢笔 (P)", "पॉलीलाइन / पेन (P)"],
    ["Quit", "Zakończ", "Beenden", "Esci", "Lopeta", "退出", "बाहर निकलें"],
    ["Radius for rounding", "Promień zaokrąglenia", "Radius zum Abrunden", "Raggio di arrotondamento", "Pyöristyksen säde", "圆角半径", "गोल करने की त्रिज्या"],
    ["Rectangle (R)", "Prostokąt (R)", "Rechteck (R)", "Rettangolo (R)", "Suorakulmio (R)", "矩形 (R)", "आयत (R)"],
    ["Redo", "Ponów", "Wiederholen", "Ripeti", "Tee uudelleen", "重做", "फिर से करें"],
    ["Restore", "Przywróć", "Wiederherstellen", "Ripristina", "Palauta", "还原", "पूर्ववत करें"],
    ["Rotate 90° CCW", "Obróć 90° w lewo", "90° gegen den Uhrzeigersinn drehen", "Ruota di 90° antiorari", "Kierrä 90° vastapäivään", "逆时针旋转 90°", "90° वामावर्त घुमाएँ"],
    ["Rotate 90° CW", "Obróć 90° w prawo", "90° im Uhrzeigersinn drehen", "Ruota di 90° orari", "Kierrä 90° myötäpäivään", "顺时针旋转 90°", "90° दक्षिणावर्त घुमाएँ"],
    ["Round", "Zaokrąglij", "Abrunden", "Arrotonda", "Pyöristä", "圆角", "गोल करें"],
    ["Round the selected corners with this radius", "Zaokrąglij zaznaczone rogi tym promieniem", "Ausgewählte Ecken mit diesem Radius abrunden", "Arrotonda gli angoli selezionati con questo raggio", "Pyöristä valitut kulmat tällä säteellä", "用此半径为所选角倒圆", "चयनित कोनों को इस त्रिज्या से गोल करें"],
    ["Rounded {} corner(s).", "Zaokrąglono rogów: {}.", "{} Ecke(n) abgerundet.", "Arrotondati {} angoli.", "Pyöristetty {} kulmaa.", "已圆角处理 {} 个角。", "{} कोने गोल किए गए।"],
    ["Select (V)", "Zaznaczanie (V)", "Auswahl (V)", "Selezione (V)", "Valinta (V)", "选择 (V)", "चयन (V)"],
    ["Select a shape first, then edit its nodes.", "Najpierw zaznacz kształt, potem edytuj jego węzły.", "Zuerst eine Form auswählen, dann ihre Knoten bearbeiten.", "Seleziona prima una forma, poi modifica i suoi nodi.", "Valitse ensin muoto ja muokkaa sitten sen solmuja.", "请先选择形状，再编辑其节点。", "पहले एक आकृति चुनें, फिर उसके नोड संपादित करें।"],
    ["Select all", "Zaznacz wszystko", "Alles auswählen", "Seleziona tutto", "Valitse kaikki", "全选", "सभी चुनें"],
    ["Select at least two objects to group.", "Zaznacz co najmniej dwa obiekty do zgrupowania.", "Wähle mindestens zwei Objekte zum Gruppieren.", "Seleziona almeno due oggetti da raggruppare.", "Valitse vähintään kaksi ryhmitettävää kohdetta.", "请至少选择两个对象进行编组。", "समूह बनाने के लिए कम से कम दो ऑब्जेक्ट चुनें।"],
    ["Send to back", "Przesuń na spód", "In den Hintergrund", "Porta in secondo piano", "Vie taakse", "置于底层", "सबसे पीछे भेजें"],
    ["Smooth", "Gładki", "Glatt", "Morbido", "Pehmeä", "平滑", "चिकना"],
    ["Snap to grid", "Przyciągaj do siatki", "Am Raster einrasten", "Aggancia alla griglia", "Kohdista ruudukkoon", "对齐网格", "ग्रिड पर स्नैप करें"],
    ["Snap to objects", "Przyciągaj do obiektów", "An Objekten einrasten", "Aggancia agli oggetti", "Kohdista objekteihin", "对齐对象", "ऑब्जेक्ट पर स्नैप करें"],
    ["Spiral (I)", "Spirala (I)", "Spirale (I)", "Spirale (I)", "Spiraali (I)", "螺旋 (I)", "सर्पिल (I)"],
    ["Star (S)", "Gwiazda (S)", "Stern (S)", "Stella (S)", "Tähti (S)", "星形 (S)", "तारा (S)"],
    ["Text (T)", "Tekst (T)", "Text (T)", "Testo (T)", "Teksti (T)", "文字 (T)", "टेक्स्ट (T)"],
    ["There are no straight corners to round.", "Brak prostych narożników do zaokrąglenia.", "Es gibt keine geraden Ecken zum Abrunden.", "Non ci sono angoli retti da arrotondare.", "Pyöristettäviä suoria kulmia ei ole.", "没有可倒圆的直线角。", "गोल करने के लिए कोई सीधा कोना नहीं है।"],
    ["To curve", "Na krzywą", "Zu Kurve", "In curva", "Käyräksi", "转为曲线", "वक्र में"],
    ["To line", "Na linię", "Zu Linie", "In linea", "Viivaksi", "转为直线", "रेखा में"],
    ["Triangle (Y)", "Trójkąt (Y)", "Dreieck (Y)", "Triangolo (Y)", "Kolmio (Y)", "三角形 (Y)", "त्रिभुज (Y)"],
    ["Undo", "Cofnij", "Rückgängig", "Annulla", "Kumoa", "撤销", "पूर्ववत करें"],
    ["Ungroup", "Rozgrupuj", "Gruppierung aufheben", "Separa", "Pura ryhmitys", "取消编组", "समूह हटाएँ"],
    ["Unlock", "Odblokuj", "Entsperren", "Sblocca", "Avaa lukitus", "解锁", "अनलॉक"],
    ["Unlocked {} object(s).", "Odblokowano obiektów: {}.", "{} Objekt(e) entsperrt.", "Sbloccati {} oggetti.", "Lukitus avattu {} kohteelta.", "已解锁 {} 个对象。", "{} ऑब्जेक्ट अनलॉक किए गए।"],
    ["Zoom in", "Powiększ", "Vergrößern", "Ingrandisci", "Lähennä", "放大", "ज़ूम इन"],
    ["Zoom in (Z)", "Powiększ (Z)", "Vergrößern (Z)", "Ingrandisci (Z)", "Lähennä (Z)", "放大 (Z)", "ज़ूम इन (Z)"],
    ["Zoom out", "Pomniejsz", "Verkleinern", "Riduci", "Loitonna", "缩小", "ज़ूम आउट"],
    ["Zoom out (X)", "Pomniejsz (X)", "Verkleinern (X)", "Riduci (X)", "Loitonna (X)", "缩小 (X)", "ज़ूम आउट (X)"],
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
    const SOURCES: [&str; 7] = [
        include_str!("editor/host.rs"),
        include_str!("editor/commands.rs"),
        include_str!("editor/canvas.rs"),
        include_str!("editor/bars.rs"),
        include_str!("menu.rs"),
        include_str!("window.rs"),
        include_str!("widgets.rs"),
    ];

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
        // The names of the tools.
        for (_, label, _) in crate::editor::Tool::ALL {
            out.push(label.to_string());
        }
        // The labels of the standard menu entries.
        use crate::menu::{std_entries as e, Entry};
        for entry in [e::edit(), e::align(), e::place(), e::group(), e::order(), e::nodes(), e::grid(), e::zoom()].concat() {
            if let Entry::Item(_, label, _) = entry {
                out.push(label.to_string());
            }
        }
        out
    }

    #[test]
    fn every_text_the_shared_ui_shows_is_in_the_table_with_every_language() {
        let lits = literals();
        assert!(lits.len() >= 60, "the scan finds the texts: {}", lits.len());
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
