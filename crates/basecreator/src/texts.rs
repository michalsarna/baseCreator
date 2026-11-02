//! The texts of BaseCreator that the shared interface does not have: English, six languages in the table, and the other 23 in the files
//! of `lang`. The shared texts are in base-ui; the application's own are asked first.
use base_ui::i18n::{self, Texts};
use std::sync::Once;

/// The texts of the other languages, in the order of `Lang::ALL` after the first seven. One line each: `id|translation`, where the id is
/// the number of the English text in `lang/keys.txt`.
const EXTRA_FILES: [&str; 23] = [
    include_str!("../lang/bg.txt"),
    include_str!("../lang/ca.txt"),
    include_str!("../lang/hr.txt"),
    include_str!("../lang/cs.txt"),
    include_str!("../lang/da.txt"),
    include_str!("../lang/nl.txt"),
    include_str!("../lang/et.txt"),
    include_str!("../lang/fr.txt"),
    include_str!("../lang/el.txt"),
    include_str!("../lang/hu.txt"),
    include_str!("../lang/is.txt"),
    include_str!("../lang/ga.txt"),
    include_str!("../lang/lv.txt"),
    include_str!("../lang/lt.txt"),
    include_str!("../lang/mt.txt"),
    include_str!("../lang/nb.txt"),
    include_str!("../lang/pt.txt"),
    include_str!("../lang/ro.txt"),
    include_str!("../lang/sk.txt"),
    include_str!("../lang/sl.txt"),
    include_str!("../lang/es.txt"),
    include_str!("../lang/sv.txt"),
    include_str!("../lang/uk.txt"),
];
const KEYS: &str = include_str!("../lang/keys.txt");

pub static TEXTS: Texts = Texts::new(TABLE, KEYS, EXTRA_FILES);

/// Make the texts known to the translation layer (once).
pub fn register() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| i18n::register(&TEXTS));
}

// Columns: English, Polski, Deutsch, Italiano, Suomi, 中文, हिन्दी
#[rustfmt::skip]
const TABLE: &[[&str; 7]] = &[
    ["A usual sheet size…", "Zwykły rozmiar arkusza…", "Eine übliche Blattgröße…", "Una dimensione di foglio comune…", "Tavallinen arkin koko…", "常见的纸张尺寸…", "एक सामान्य कागज़ आकार…"],
    ["About", "O programie", "Über", "Informazioni", "Tietoja", "关于", "के बारे में"],
    ["Arrange", "Rozmieść", "Anordnen", "Disponi", "Järjestä", "排列", "व्यवस्थित करें"],
    ["Device settings…", "Ustawienia urządzenia…", "Geräteeinstellungen…", "Impostazioni dispositivo…", "Laitteen asetukset…", "设备设置…", "डिवाइस सेटिंग्स…"],
    ["Edit", "Edycja", "Bearbeiten", "Modifica", "Muokkaa", "编辑", "संपादन"],
    ["Export G-code…", "Eksportuj G-code…", "G-Code exportieren…", "Esporta G-code…", "Vie G-koodi…", "导出 G-code…", "G-code निर्यात करें…"],
    ["Export drawing as DXF…", "Eksportuj rysunek jako DXF…", "Zeichnung als DXF exportieren…", "Esporta il disegno come DXF…", "Vie piirros DXF-muodossa…", "将图形导出为 DXF…", "ड्रॉइंग को DXF के रूप में निर्यात करें…"],
    ["Export drawing as SVG…", "Eksportuj rysunek jako SVG…", "Zeichnung als SVG exportieren…", "Esporta il disegno come SVG…", "Vie piirros SVG-muodossa…", "将图形导出为 SVG…", "ड्रॉइंग को SVG के रूप में निर्यात करें…"],
    ["Help", "Pomoc", "Hilfe", "Aiuto", "Ohje", "帮助", "सहायता"],
    ["Import DXF…", "Importuj DXF…", "DXF importieren…", "Importa DXF…", "Tuo DXF…", "导入 DXF…", "DXF आयात करें…"],
    ["Import LightBurn project…", "Importuj projekt LightBurn…", "LightBurn-Projekt importieren…", "Importa progetto LightBurn…", "Tuo LightBurn-projekti…", "导入 LightBurn 项目…", "LightBurn प्रोजेक्ट आयात करें…"],
    ["Inches (in)", "Cale (in)", "Zoll (in)", "Pollici (in)", "Tuumat (in)", "英寸 (in)", "इंच (in)"],
    ["Insert", "Wstawianie", "Einfügen", "Inserisci", "Lisää", "插入", "सम्मिलित करें"],
    ["Landscape", "Poziomo", "Querformat", "Orizzontale", "Vaaka", "横向", "लैंडस्केप"],
    ["Millimetres (mm)", "Milimetry (mm)", "Millimeter (mm)", "Millimetri (mm)", "Millimetrit (mm)", "毫米 (mm)", "मिलीमीटर (mm)"],
    ["New", "Nowy", "Neu", "Nuovo", "Uusi", "新建", "नया"],
    ["Nothing selected", "Nic nie zaznaczono", "Nichts ausgewählt", "Nessuna selezione", "Ei valintaa", "未选择任何内容", "कुछ चयनित नहीं"],
    ["Open a drawing or start an empty design.", "Otwórz rysunek albo zacznij pusty projekt.", "Öffne eine Zeichnung oder beginne mit einem leeren Entwurf.", "Apri un disegno o inizia un progetto vuoto.", "Avaa piirros tai aloita tyhjä suunnitelma.", "打开一幅图，或开始一个空白设计。", "कोई ड्रॉइंग खोलें या खाली डिज़ाइन शुरू करें।"],
    ["Open…", "Otwórz…", "Öffnen…", "Apri…", "Avaa…", "打开…", "खोलें…"],
    ["Page setup…", "Ustawienia strony…", "Seite einrichten…", "Imposta pagina…", "Sivun asetukset…", "页面设置…", "पृष्ठ सेटअप…"],
    ["Portrait", "Pionowo", "Hochformat", "Verticale", "Pysty", "纵向", "पोर्ट्रेट"],
    ["Properties", "Właściwości", "Eigenschaften", "Proprietà", "Ominaisuudet", "属性", "गुण"],
    ["Put the selected shapes on this layer", "Umieść zaznaczone kształty na tej warstwie", "Die ausgewählten Formen auf diese Ebene legen", "Metti le forme selezionate su questo livello", "Siirrä valitut muodot tälle tasolle", "将所选图形放到此图层", "चुने हुए आकार इस लेयर पर रखें"],
    ["Ready", "Gotowe", "Bereit", "Pronto", "Valmis", "就绪", "तैयार"],
    ["Save as…", "Zapisz jako…", "Speichern unter…", "Salva come…", "Tallenna nimellä…", "另存为…", "इस रूप में सहेजें…"],
    ["Settings", "Ustawienia", "Einstellungen", "Impostazioni", "Asetukset", "设置", "सेटिंग्स"],
    ["Shapes on this layer", "Kształty na tej warstwie", "Formen auf dieser Ebene", "Forme su questo livello", "Muodot tällä tasolla", "此图层上的图形", "इस लेयर पर आकार"],
    ["Shown in the work area", "Pokazana w obszarze roboczym", "Im Arbeitsbereich angezeigt", "Mostrato nell'area di lavoro", "Näytetään työalueella", "显示在工作区域", "कार्य क्षेत्र में दिखाया गया"],
    ["Units", "Jednostki", "Einheiten", "Unità", "Yksiköt", "单位", "इकाइयाँ"],
    ["View", "Widok", "Ansicht", "Vista", "Näytä", "视图", "दृश्य"],
    ["View options…", "Opcje widoku…", "Ansichtsoptionen…", "Opzioni di visualizzazione…", "Näkymän asetukset…", "视图选项…", "दृश्य विकल्प…"],
];
