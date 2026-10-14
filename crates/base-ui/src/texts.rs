//! The texts of the shared interface (node bar, selection commands): English, six languages in the table, and the
//! other 23 in the files next to this one. Generated from the texts of LightCreator; the test of this file checks that
//! every text used by `tr` in this crate has all its translations.
use crate::i18n::{Texts, TABLE_LANGS};

#[rustfmt::skip]
pub const TABLE: &[[&str; TABLE_LANGS]] = &[
    ["(empty)", "(pusto)", "(leer)", "(vuoto)", "(tyhjä)", "（空）", "(खाली)"],
    ["A newer version is available: {}", "Dostępna jest nowsza wersja: {}", "Eine neuere Version ist verfügbar: {}", "È disponibile una versione più recente: {}", "Uudempi versio on saatavilla: {}", "有新版本可用：{}", "नया संस्करण उपलब्ध है: {}"],
    ["Add SVG files from your computer to your clipart", "Dodaj pliki SVG z komputera do swoich clipartów", "SVG-Dateien von deinem Computer zu deinen Clipart hinzufügen", "Aggiungi file SVG dal computer alle tue clipart", "Lisää SVG-tiedostoja tietokoneelta omiin clipart-kuviisi", "将电脑上的 SVG 文件添加到你的剪贴画", "अपने कंप्यूटर से SVG फ़ाइलें अपनी क्लिपआर्ट में जोड़ें"],
    ["Add file…", "Dodaj plik…", "Datei hinzufügen…", "Aggiungi file…", "Lisää tiedosto…", "添加文件…", "फ़ाइल जोड़ें…"],
    ["Add node", "Dodaj węzeł", "Knoten hinzufügen", "Aggiungi nodo", "Lisää solmu", "添加节点", "नोड जोड़ें"],
    ["Added {} picture(s) to {}", "Dodano obrazków: {} do kategorii {}", "{} Bild(er) zu {} hinzugefügt", "Aggiunte {} immagini a {}", "Lisätty {} kuvaa luokkaan {}", "已将 {} 张图片添加到 {}", "{} चित्र {} में जोड़े गए"],
    ["Adjust image", "Dostosuj obraz", "Bild anpassen", "Regola immagine", "Säädä kuvaa", "调整图像", "छवि समायोजन"],
    ["Align bottom", "Wyrównaj do dołu", "Unten ausrichten", "Allinea in basso", "Tasaa alas", "底部对齐", "नीचे संरेखित करें"],
    ["Align centre", "Wyśrodkuj", "Zentrieren", "Centra", "Keskitä", "居中对齐", "केंद्र संरेखित करें"],
    ["Align centre (H)", "Wyśrodkuj w poziomie", "Horizontal zentrieren", "Centra orizzontalmente", "Keskitä vaakasuunnassa", "水平居中", "क्षैतिज केंद्रित करें"],
    ["Align centre (V)", "Wyśrodkuj w pionie", "Vertikal zentrieren", "Centra verticalmente", "Keskitä pystysuunnassa", "垂直居中", "ऊर्ध्वाधर केंद्रित करें"],
    ["Align left", "Wyrównaj do lewej", "Links ausrichten", "Allinea a sinistra", "Tasaa vasemmalle", "左对齐", "बाएँ संरेखित करें"],
    ["Align middle", "Wyrównaj do środka", "Mittig ausrichten", "Allinea al centro", "Tasaa keskelle", "居中对齐（垂直）", "मध्य संरेखित करें"],
    ["Align right", "Wyrównaj do prawej", "Rechts ausrichten", "Allinea a destra", "Tasaa oikealle", "右对齐", "दाएँ संरेखित करें"],
    ["Align top", "Wyrównaj do góry", "Oben ausrichten", "Allinea in alto", "Tasaa ylös", "顶部对齐", "ऊपर संरेखित करें"],
    ["Alignment", "Wyrównanie", "Ausrichtung", "Allineamento", "Tasaus", "对齐", "संरेखण"],
    ["All categories", "Wszystkie kategorie", "Alle Kategorien", "Tutte le categorie", "Kaikki luokat", "全部类别", "सभी श्रेणियाँ"],
    ["Angle", "Kąt", "Winkel", "Angolo", "Kulma", "角度", "कोण"],
    ["Animals", "Zwierzęta", "Tiere", "Animali", "Eläimet", "动物", "जानवर"],
    ["Apply", "Zastosuj", "Anwenden", "Applica", "Käytä", "应用", "लागू करें"],
    ["Author", "Autor", "Autor", "Autore", "Tekijä", "作者", "लेखक"],
    ["Auto levels", "Automatyczne poziomy", "Automatische Tonwertkorrektur", "Livelli automatici", "Automaattiset tasot", "自动色阶", "ऑटो लेवल"],
    ["Auto-fit work area to window", "Automatycznie dopasuj obszar roboczy do okna", "Arbeitsbereich automatisch an Fenster anpassen", "Adatta automaticamente l'area di lavoro alla finestra", "Sovita työalue ikkunaan automaattisesti", "自动使工作区域适应窗口", "कार्य क्षेत्र को विंडो में स्वतः फ़िट करें"],
    ["Automatic", "Automatyczny", "Automatisch", "Automatico", "Automaattinen", "自动", "स्वचालित"],
    ["Background colour", "Kolor tła", "Hintergrundfarbe", "Colore di sfondo", "Taustaväri", "背景颜色", "पृष्ठभूमि रंग"],
    ["Bottom", "Dół", "Unten", "Basso", "Ala", "下", "नीचे"],
    ["Brightness", "Jasność", "Helligkeit", "Luminosità", "Kirkkaus", "亮度", "चमक"],
    ["Bring to front", "Przesuń na wierzch", "In den Vordergrund", "Porta in primo piano", "Tuo eteen", "置于顶层", "सबसे आगे लाएँ"],
    ["Cancel", "Anuluj", "Abbrechen", "Annulla", "Peruuta", "取消", "रद्द करें"],
    ["Category", "Kategoria", "Kategorie", "Categoria", "Luokka", "类别", "श्रेणी"],
    ["Celebrations", "Święta i okazje", "Feste", "Feste", "Juhlat", "庆典", "उत्सव"],
    ["Centre", "Środek", "Mitte", "Centro", "Keskitä", "居中", "केंद्र"],
    ["Centre on bed", "Wyśrodkuj na stole", "Auf Arbeitsfläche zentrieren", "Centra sul piano", "Keskitä alustalle", "在工作台居中", "बेड पर केंद्रित करें"],
    ["Centre on each other", "Wyśrodkuj względem siebie", "Aufeinander zentrieren", "Centra l'uno sull'altro", "Keskitä toistensa suhteen", "相互居中", "एक-दूसरे के केंद्र पर रखें"],
    ["Change category", "Zmień kategorię", "Kategorie ändern", "Cambia categoria", "Vaihda luokkaa", "更改类别", "श्रेणी बदलें"],
    ["Change category…", "Zmień kategorię…", "Kategorie ändern…", "Cambia categoria…", "Vaihda luokkaa…", "更改类别…", "श्रेणी बदलें…"],
    ["Clear list", "Wyczyść listę", "Liste leeren", "Svuota elenco", "Tyhjennä luettelo", "清除列表", "सूची साफ़ करें"],
    ["Click a picture to see its licence and save it.", "Kliknij obrazek, aby zobaczyć jego licencję i go zapisać.", "Klicke auf ein Bild, um die Lizenz zu sehen und es zu speichern.", "Fai clic su un'immagine per vedere la licenza e salvarla.", "Napsauta kuvaa nähdäksesi lisenssin ja tallentaaksesi sen.", "点击图片查看其许可证并保存。", "लाइसेंस देखने और सहेजने के लिए चित्र पर क्लिक करें।"],
    ["Click to place on the work area", "Kliknij, aby umieścić na obszarze roboczym", "Klicken, um im Arbeitsbereich zu platzieren", "Clic per posizionare sull'area di lavoro", "Napsauta sijoittaaksesi työalueelle", "点击放置到工作区域", "कार्य क्षेत्र पर रखने के लिए क्लिक करें"],
    ["Clipart", "Clipart", "Clipart", "Clipart", "Clipart", "剪贴画", "क्लिपआर्ट"],
    ["Close", "Zamknij", "Schließen", "Chiudi", "Sulje", "关闭", "बंद करें"],
    ["Close an open path, or open a closed one at the selected node", "Zamknij otwartą ścieżkę lub otwórz zamkniętą w zaznaczonym węźle", "Offenen Pfad schließen oder geschlossenen am ausgewählten Knoten öffnen", "Chiudi un tracciato aperto o apri uno chiuso al nodo selezionato", "Sulje avoin polku tai avaa suljettu valitun solmun kohdalta", "闭合开放路径，或在所选节点处断开闭合路径", "खुले पाथ को बंद करें या बंद पाथ को चयनित नोड पर खोलें"],
    ["Collection", "Kolekcja", "Sammlung", "Raccolta", "Kokoelma", "图标集", "संग्रह"],
    ["Colour", "Kolor", "Farbe", "Colore", "Väri", "颜色", "रंग"],
    ["Colour scheme", "Schemat kolorów", "Farbschema", "Combinazione colori", "Värimaailma", "配色方案", "रंग योजना"],
    ["Contrast", "Kontrast", "Kontrast", "Contrasto", "Kontrasti", "对比度", "कंट्रास्ट"],
    ["Convert text to curves, or trace the image, to edit its nodes.", "Zamień tekst na krzywe albo wektoryzuj obraz, aby edytować jego węzły.", "Text in Kurven umwandeln oder das Bild nachzeichnen, um die Knoten zu bearbeiten.", "Converti il testo in curve o vettorializza l'immagine per modificarne i nodi.", "Muunna teksti käyriksi tai jäljitä kuva muokataksesi sen solmuja.", "将文字转换为曲线或描摹图像后才能编辑其节点。", "नोड संपादित करने के लिए टेक्स्ट को वक्रों में बदलें या छवि को ट्रेस करें।"],
    ["Convert to path", "Zamień na ścieżkę", "In Pfad umwandeln", "Converti in tracciato", "Muunna poluksi", "转换为路径", "पाथ में बदलें"],
    ["Copy", "Kopiuj", "Kopieren", "Copia", "Kopioi", "复制", "कॉपी करें"],
    ["Corner", "Narożnik", "Ecke", "Angolo", "Kulma", "尖角", "कोना"],
    ["Could not check for updates.", "Nie udało się sprawdzić aktualizacji.", "Die Suche nach Updates ist fehlgeschlagen.", "Impossibile controllare gli aggiornamenti.", "Päivitysten tarkistus epäonnistui.", "无法检查更新。", "अपडेट जाँचना संभव नहीं हुआ।"],
    ["Delete", "Usuń", "Löschen", "Elimina", "Poista", "删除", "हटाएँ"],
    ["Delete node", "Usuń węzeł", "Knoten löschen", "Elimina nodo", "Poista solmu", "删除节点", "नोड हटाएँ"],
    ["Delete the image afterwards", "Usuń obraz po wektoryzacji", "Bild danach löschen", "Elimina l'immagine dopo", "Poista kuva jälkeenpäin", "完成后删除图像", "बाद में छवि हटाएँ"],
    ["Delete…", "Usuń…", "Löschen…", "Elimina…", "Poista…", "删除…", "हटाएँ…"],
    ["Describe the project: what it is, the settings that worked, notes for next time.", "Opisz projekt: co to jest, ustawienia, które zadziałały, notatki na następny raz.", "Beschreibe das Projekt: was es ist, welche Einstellungen funktionierten, Notizen fürs nächste Mal.", "Descrivi il progetto: cos'è, le impostazioni che hanno funzionato, appunti per la prossima volta.", "Kuvaile projekti: mikä se on, toimivat asetukset, muistiinpanoja seuraavaa kertaa varten.", "描述项目：它是什么、有效的设置、给下次的备注。", "परियोजना का वर्णन करें: यह क्या है, कौन-सी सेटिंग काम आईं, अगली बार के लिए नोट्स।"],
    ["Description", "Opis", "Beschreibung", "Descrizione", "Kuvaus", "描述", "विवरण"],
    ["Details", "Szczegóły", "Details", "Dettagli", "Tiedot", "详细信息", "विवरण"],
    ["Details…", "Szczegóły…", "Details…", "Dettagli…", "Tiedot…", "详细信息…", "विवरण…"],
    ["Distance between lines", "Odległość między liniami", "Abstand zwischen den Linien", "Distanza tra le linee", "Viivojen väli", "线间距", "रेखाओं के बीच दूरी"],
    ["Download", "Pobierz", "Herunterladen", "Scarica", "Lataa", "下载", "डाउनलोड"],
    ["Download failed: {}", "Pobieranie nie powiodło się: {}", "Download fehlgeschlagen: {}", "Download non riuscito: {}", "Lataus epäonnistui: {}", "下载失败：{}", "डाउनलोड विफल: {}"],
    ["Downloading…", "Pobieranie…", "Wird heruntergeladen…", "Download in corso…", "Ladataan…", "正在下载…", "डाउनलोड हो रहा है…"],
    ["Duplicate", "Powiel", "Duplizieren", "Duplica", "Monista", "复制副本", "डुप्लिकेट"],
    ["Edit nodes", "Edytuj węzły", "Knoten bearbeiten", "Modifica nodi", "Muokkaa solmuja", "编辑节点", "नोड संपादित करें"],
    ["Ellipse (E)", "Elipsa (E)", "Ellipse (E)", "Ellisse (E)", "Ellipsi (E)", "椭圆 (E)", "दीर्घवृत्त (E)"],
    ["Engrave the light parts instead of the dark parts", "Grawieruj jasne części zamiast ciemnych", "Helle statt dunkle Bereiche gravieren", "Incidi le parti chiare invece di quelle scure", "Kaiverra vaaleat osat tummien sijaan", "雕刻浅色部分而非深色部分", "गहरे के बजाय हल्के भागों को उकेरें"],
    ["Fit all objects", "Dopasuj do wszystkich obiektów", "Alle Objekte einpassen", "Adatta a tutti gli oggetti", "Sovita kaikki kohteet", "适应所有对象", "सभी ऑब्जेक्ट फ़िट करें"],
    ["Fit all objects (Ctrl+9)", "Dopasuj do wszystkich obiektów (Ctrl+9)", "Alle Objekte einpassen (Strg+9)", "Adatta a tutti gli oggetti (Ctrl+9)", "Sovita kaikki kohteet (Ctrl+9)", "适应所有对象 (Ctrl+9)", "सभी ऑब्जेक्ट फ़िट करें (Ctrl+9)"],
    ["Fit bed to window", "Dopasuj stół do okna", "Arbeitsfläche einpassen", "Adatta il piano alla finestra", "Sovita alusta ikkunaan", "适应窗口", "बेड को विंडो में फिट करें"],
    ["Fit work area (Ctrl+0)", "Dopasuj obszar roboczy (Ctrl+0)", "Arbeitsbereich einpassen (Strg+0)", "Adatta l'area di lavoro (Ctrl+0)", "Sovita työalue (Ctrl+0)", "适应工作区域 (Ctrl+0)", "कार्य क्षेत्र फ़िट करें (Ctrl+0)"],
    ["Flip H", "Odbij H", "Spiegeln H", "Rifletti O", "Käännä V", "水平翻转", "पलटें H"],
    ["Flip V", "Odbij V", "Spiegeln V", "Rifletti V", "Käännä P", "垂直翻转", "पलटें V"],
    ["Flip horizontal", "Odbij w poziomie", "Horizontal spiegeln", "Rifletti orizzontalmente", "Käännä vaakasuunnassa", "水平翻转", "क्षैतिज पलटें"],
    ["Flip vertical", "Odbij w pionie", "Vertikal spiegeln", "Rifletti verticalmente", "Käännä pystysuunnassa", "垂直翻转", "ऊर्ध्वाधर पलटें"],
    ["Food", "Jedzenie", "Essen", "Cibo", "Ruoka", "食物", "भोजन"],
    ["Gamma", "Gamma", "Gamma", "Gamma", "Gamma", "伽马", "गामा"],
    ["Grid", "Siatka", "Raster", "Griglia", "Ruudukko", "网格", "ग्रिड"],
    ["Group", "Grupuj", "Gruppieren", "Raggruppa", "Ryhmitä", "编组", "समूह बनाएँ"],
    ["Grouped. The objects keep their layers.", "Zgrupowano. Obiekty zachowują swoje warstwy.", "Gruppiert. Die Objekte behalten ihre Ebenen.", "Raggruppati. Gli oggetti mantengono i loro livelli.", "Ryhmitetty. Kohteet säilyttävät tasonsa.", "已编组。对象保留各自的图层。", "समूह बना। ऑब्जेक्ट अपनी लेयर बनाए रखते हैं।"],
    ["Heart (K)", "Serce (K)", "Herz (K)", "Cuore (K)", "Sydän (K)", "心形 (K)", "दिल (K)"],
    ["Ignore small areas", "Ignoruj małe obszary", "Kleine Bereiche ignorieren", "Ignora aree piccole", "Ohita pienet alueet", "忽略小区域", "छोटे क्षेत्र अनदेखा करें"],
    ["Import", "Importuj", "Importieren", "Importa", "Tuo", "导入", "आयात"],
    ["Import failed: {}", "Nie udało się zaimportować: {}", "Import fehlgeschlagen: {}", "Importazione non riuscita: {}", "Tuonti epäonnistui: {}", "导入失败：{}", "आयात विफल: {}"],
    ["Import image", "Import obrazu", "Bild importieren", "Importa immagine", "Tuo kuva", "导入图像", "छवि आयात"],
    ["Imported image {}", "Zaimportowano obraz {}", "Bild {} importiert", "Immagine {} importata", "Kuva {} tuotu", "已导入图像 {}", "छवि {} आयात की गई"],
    ["Included", "W zestawie", "Mitgeliefert", "Incluse", "Mukana", "自带", "साथ आए"],
    ["Insert a node (or double-click a segment)", "Wstaw węzeł (lub kliknij dwukrotnie odcinek)", "Knoten einfügen (oder Segment doppelklicken)", "Inserisci un nodo (o doppio clic su un segmento)", "Lisää solmu (tai kaksoisnapsauta janaa)", "插入节点（或双击线段）", "नोड डालें (या खंड पर डबल-क्लिक करें)"],
    ["Language", "Język", "Sprache", "Lingua", "Kieli", "语言", "भाषा"],
    ["Left", "Lewo", "Links", "Sinistra", "Vasen", "左", "बाएँ"],
    ["Licence", "Licencja", "Lizenz", "Licenza", "Lisenssi", "许可证", "लाइसेंस"],
    ["Line (L)", "Linia (L)", "Linie (L)", "Linea (L)", "Viiva (L)", "直线 (L)", "रेखा (L)"],
    ["Line thickness", "Grubość linii", "Linienstärke", "Spessore delle linee", "Viivan paksuus", "线条粗细", "रेखा की मोटाई"],
    ["Lock", "Zablokuj", "Sperren", "Blocca", "Lukitse", "锁定", "लॉक करें"],
    ["Lock aspect ratio", "Zablokuj proporcje", "Seitenverhältnis sperren", "Blocca proporzioni", "Lukitse kuvasuhde", "锁定宽高比", "आस्पेक्ट रेशियो लॉक करें"],
    ["Locked {} object(s).", "Zablokowano obiektów: {}.", "{} Objekt(e) gesperrt.", "Bloccati {} oggetti.", "Lukittu {} kohdetta.", "已锁定 {} 个对象。", "{} ऑब्जेक्ट लॉक किए गए।"],
    ["Main grid", "Siatka główna", "Hauptraster", "Griglia principale", "Pääruudukko", "主网格", "मुख्य ग्रिड"],
    ["Make the segment after each selected node a curve", "Zamień odcinek za każdym zaznaczonym węzłem na krzywą", "Das Segment nach jedem ausgewählten Knoten zur Kurve machen", "Rendi curvo il segmento dopo ogni nodo selezionato", "Tee jokaisen valitun solmun jälkeinen jana käyräksi", "将每个所选节点之后的线段设为曲线", "प्रत्येक चयनित नोड के बाद का खंड वक्र करें"],
    ["Make the segment after each selected node straight", "Zamień odcinek za każdym zaznaczonym węzłem na prosty", "Das Segment nach jedem ausgewählten Knoten gerade machen", "Rendi retto il segmento dopo ogni nodo selezionato", "Tee jokaisen valitun solmun jälkeinen jana suoraksi", "将每个所选节点之后的线段设为直线", "प्रत्येक चयनित नोड के बाद का खंड सीधा करें"],
    ["Make the selected nodes corners", "Zamień zaznaczone węzły na narożniki", "Ausgewählte Knoten zu Ecken machen", "Rendi angoli i nodi selezionati", "Muuta valitut solmut kulmiksi", "将所选节点设为尖角", "चयनित नोड को कोना बनाएँ"],
    ["Make the selected nodes smooth", "Zamień zaznaczone węzły na gładkie", "Ausgewählte Knoten glatt machen", "Rendi morbidi i nodi selezionati", "Muuta valitut solmut pehmeiksi", "将所选节点设为平滑", "चयनित नोड को चिकना बनाएँ"],
    ["Maximise", "Maksymalizuj", "Maximieren", "Ingrandisci", "Suurenna", "最大化", "बड़ा करें"],
    ["Mid-H", "Śr.-H", "Mitte-H", "Centro-O", "Kesk.-V", "水平中", "मध्य-H"],
    ["Mid-V", "Śr.-V", "Mitte-V", "Centro-V", "Kesk.-P", "垂直中", "मध्य-V"],
    ["Mine", "Moje", "Meine", "Mie", "Omat", "我的", "मेरे"],
    ["Minimise", "Minimalizuj", "Minimieren", "Riduci a icona", "Pienennä", "最小化", "छोटा करें"],
    ["Mirror and centre", "Odbicie i środek", "Spiegeln und zentrieren", "Specchio e centro", "Peilaus ja keskitys", "镜像和居中", "प्रतिबिंब और केंद्र"],
    ["Music and fun", "Muzyka i zabawa", "Musik und Spaß", "Musica e giochi", "Musiikki ja hauskuus", "音乐与娱乐", "संगीत और मनोरंजन"],
    ["My clipart", "Moje cliparty", "Meine Clipart", "Le mie clipart", "Omat clipartit", "我的剪贴画", "मेरी क्लिपआर्ट"],
    ["Name", "Nazwa", "Name", "Nome", "Nimi", "名称", "नाम"],
    ["Nature", "Natura", "Natur", "Natura", "Luonto", "自然", "प्रकृति"],
    ["Negative image", "Obraz negatywowy", "Negativbild", "Immagine negativa", "Negatiivikuva", "负片图像", "नेगेटिव छवि"],
    ["Newest release: {}", "Najnowsze wydanie: {}", "Neueste Version: {}", "Ultima versione: {}", "Uusin julkaisu: {}", "最新版本：{}", "नवीनतम संस्करण: {}"],
    ["No pictures here. Use Online… to find some.", "Brak obrazków. Użyj „Z internetu…”, aby je znaleźć.", "Keine Bilder hier. Mit „Online…“ findest du welche.", "Nessuna immagine qui. Usa «Online…» per trovarne.", "Ei kuvia. Etsi niitä kohdasta ”Verkosta…”.", "这里没有图片。使用“在线…”查找。", "यहाँ कोई चित्र नहीं। खोजने के लिए “ऑनलाइन…” उपयोग करें।"],
    ["Node edit (N)", "Edycja węzłów (N)", "Knotenbearbeitung (N)", "Modifica nodi (N)", "Solmujen muokkaus (N)", "节点编辑 (N)", "नोड संपादन (N)"],
    ["Nothing found.", "Nic nie znaleziono.", "Nichts gefunden.", "Nessun risultato.", "Mitään ei löytynyt.", "未找到结果。", "कुछ नहीं मिला।"],
    ["OK", "OK", "OK", "OK", "OK", "确定", "ठीक है"],
    ["Objects", "Przedmioty", "Gegenstände", "Oggetti", "Esineet", "物品", "वस्तुएँ"],
    ["Online clipart", "Cliparty z internetu", "Clipart online", "Clipart online", "Clipartit verkosta", "在线剪贴画", "ऑनलाइन क्लिपआर्ट"],
    ["Online…", "Z internetu…", "Online…", "Online…", "Verkosta…", "在线…", "ऑनलाइन…"],
    ["Only save pictures you may use. Check the licence of the source.", "Zapisuj tylko obrazki, z których wolno Ci korzystać. Sprawdź licencję źródła.", "Speichere nur Bilder, die du verwenden darfst. Prüfe die Lizenz der Quelle.", "Salva solo immagini che puoi usare. Controlla la licenza della fonte.", "Tallenna vain kuvia, joita saat käyttää. Tarkista lähteen lisenssi.", "仅保存你有权使用的图片。请检查来源的许可证。", "केवल वे चित्र सहेजें जिनका आप उपयोग कर सकते हैं। स्रोत का लाइसेंस जाँचें।"],
    ["Open / close", "Otwórz / zamknij", "Öffnen / schließen", "Apri / chiudi", "Avaa / sulje", "断开 / 闭合", "खोलें / बंद करें"],
    ["Open recent", "Otwórz ostatnie", "Zuletzt geöffnet", "Apri recenti", "Avaa viimeisimmät", "最近打开", "हाल की फ़ाइलें खोलें"],
    ["Open the download page", "Otwórz stronę pobierania", "Download-Seite öffnen", "Apri la pagina di download", "Avaa lataussivu", "打开下载页面", "डाउनलोड पृष्ठ खोलें"],
    ["Or download a picture from an address", "Albo pobierz obrazek spod adresu", "Oder ein Bild von einer Adresse herunterladen", "Oppure scarica un'immagine da un indirizzo", "Tai lataa kuva osoitteesta", "或从网址下载图片", "या किसी पते से चित्र डाउनलोड करें"],
    ["Pan (H)", "Przesuwanie widoku (H)", "Verschieben (H)", "Sposta vista (H)", "Panoroi (H)", "平移 (H)", "पैन (H)"],
    ["Paste", "Wklej", "Einfügen", "Incolla", "Liitä", "粘贴", "चिपकाएँ"],
    ["Pictures that come with the program", "Obrazki dostarczone z programem", "Bilder, die mit dem Programm kommen", "Immagini fornite con il programma", "Ohjelman mukana tulevat kuvat", "随程序提供的图片", "प्रोग्राम के साथ आने वाले चित्र"],
    ["Pictures you downloaded or added", "Obrazki pobrane lub dodane przez Ciebie", "Von dir heruntergeladene oder hinzugefügte Bilder", "Immagini scaricate o aggiunte da te", "Lataamasi tai lisäämäsi kuvat", "你下载或添加的图片", "आपके द्वारा डाउनलोड या जोड़े गए चित्र"],
    ["Placed {}", "Umieszczono: {}", "{} platziert", "Posizionato: {}", "Sijoitettu: {}", "已放置：{}", "{} रखा गया"],
    ["Polygon (G)", "Wielokąt (G)", "Vieleck (G)", "Poligono (G)", "Monikulmio (G)", "多边形 (G)", "बहुभुज (G)"],
    ["Polyline / pen (P)", "Linia łamana / pióro (P)", "Polylinie / Stift (P)", "Polilinea / penna (P)", "Murtoviiva / kynä (P)", "折线 / 钢笔 (P)", "पॉलीलाइन / पेन (P)"],
    ["Position and size", "Położenie i rozmiar", "Position und Größe", "Posizione e dimensioni", "Sijainti ja koko", "位置和大小", "स्थिति और आकार"],
    ["Project page on GitHub", "Strona projektu na GitHubie", "Projektseite auf GitHub", "Pagina del progetto su GitHub", "Projektin sivu GitHubissa", "GitHub 上的项目页面", "GitHub पर प्रोजेक्ट पेज"],
    ["Quick guide", "Krótki przewodnik", "Kurzanleitung", "Guida rapida", "Pikaopas", "快速指南", "त्वरित मार्गदर्शिका"],
    ["Quit", "Zakończ", "Beenden", "Esci", "Lopeta", "退出", "बाहर निकलें"],
    ["Radius for rounding", "Promień zaokrąglenia", "Radius zum Abrunden", "Raggio di arrotondamento", "Pyöristyksen säde", "圆角半径", "गोल करने की त्रिज्या"],
    ["Rectangle (R)", "Prostokąt (R)", "Rechteck (R)", "Rettangolo (R)", "Suorakulmio (R)", "矩形 (R)", "आयत (R)"],
    ["Redo", "Ponów", "Wiederholen", "Ripeti", "Tee uudelleen", "重做", "फिर से करें"],
    ["Remove this picture from your clipart? The file on your disk is deleted.", "Usunąć ten obrazek ze swoich clipartów? Plik na dysku zostanie skasowany.", "Dieses Bild aus deinen Clipart entfernen? Die Datei auf der Festplatte wird gelöscht.", "Rimuovere questa immagine dalle tue clipart? Il file sul disco viene eliminato.", "Poistetaanko tämä kuva omista clipart-kuvistasi? Tiedosto levyltä poistetaan.", "从你的剪贴画中移除此图片？磁盘上的文件将被删除。", "इस चित्र को अपनी क्लिपआर्ट से हटाएँ? डिस्क की फ़ाइल हटा दी जाएगी।"],
    ["Rename", "Zmień nazwę", "Umbenennen", "Rinomina", "Nimeä uudelleen", "重命名", "नाम बदलें"],
    ["Rename…", "Zmień nazwę…", "Umbenennen…", "Rinomina…", "Nimeä uudelleen…", "重命名…", "नाम बदलें…"],
    ["Reset", "Resetuj", "Zurücksetzen", "Reimposta", "Palauta", "重置", "रीसेट"],
    ["Reset grid", "Przywróć siatkę", "Raster zurücksetzen", "Ripristina griglia", "Palauta ruudukko", "重置网格", "ग्रिड रीसेट करें"],
    ["Restore", "Przywróć", "Wiederherstellen", "Ripristina", "Palauta", "还原", "पूर्ववत करें"],
    ["Right", "Prawo", "Rechts", "Destra", "Oikea", "右", "दाएँ"],
    ["Rotate", "Obróć", "Drehen", "Ruota", "Kierrä", "旋转", "घुमाएँ"],
    ["Rotate 90° CCW", "Obróć 90° w lewo", "90° gegen den Uhrzeigersinn drehen", "Ruota di 90° antiorari", "Kierrä 90° vastapäivään", "逆时针旋转 90°", "90° वामावर्त घुमाएँ"],
    ["Rotate 90° CW", "Obróć 90° w prawo", "90° im Uhrzeigersinn drehen", "Ruota di 90° orari", "Kierrä 90° myötäpäivään", "顺时针旋转 90°", "90° दक्षिणावर्त घुमाएँ"],
    ["Rotate and flip", "Obrót i odbicie", "Drehen und spiegeln", "Ruota e rifletti", "Kierrä ja käännä", "旋转与翻转", "घुमाएँ और पलटें"],
    ["Rotation", "Obrót", "Drehung", "Rotazione", "Kierto", "旋转", "घुमाव"],
    ["Round", "Zaokrąglij", "Abrunden", "Arrotonda", "Pyöristä", "圆角", "गोल करें"],
    ["Round the selected corners with this radius", "Zaokrąglij zaznaczone rogi tym promieniem", "Ausgewählte Ecken mit diesem Radius abrunden", "Arrotonda gli angoli selezionati con questo raggio", "Pyöristä valitut kulmat tällä säteellä", "用此半径为所选角倒圆", "चयनित कोनों को इस त्रिज्या से गोल करें"],
    ["Rounded {} corner(s).", "Zaokrąglono rogów: {}.", "{} Ecke(n) abgerundet.", "Arrotondati {} angoli.", "Pyöristetty {} kulmaa.", "已圆角处理 {} 个角。", "{} कोने गोल किए गए।"],
    ["Save a copy in my clipart", "Zapisz kopię w moich clipartach", "Kopie in meinen Clipart speichern", "Salva una copia nelle mie clipart", "Tallenna kopio omiin clipart-kuviini", "在我的剪贴画中保存副本", "मेरी क्लिपआर्ट में प्रति सहेजें"],
    ["Save to my clipart", "Zapisz w moich clipartach", "In meinen Clipart speichern", "Salva nelle mie clipart", "Tallenna omiin clipart-kuviini", "保存到我的剪贴画", "मेरी क्लिपआर्ट में सहेजें"],
    ["Saved {} in {}", "Zapisano {} w kategorii {}", "{} in {} gespeichert", "Salvato {} in {}", "Tallennettu {} luokkaan {}", "已将 {} 保存到 {}", "{} को {} में सहेजा गया"],
    ["Search", "Szukaj", "Suchen", "Cerca", "Haku", "搜索", "खोजें"],
    ["Search failed: {}", "Wyszukiwanie nie powiodło się: {}", "Suche fehlgeschlagen: {}", "Ricerca non riuscita: {}", "Haku epäonnistui: {}", "搜索失败：{}", "खोज विफल: {}"],
    ["Search open icon collections (Iconify). Pictures are saved on your disk, in a category of your choice.", "Przeszukaj otwarte kolekcje ikon (Iconify). Obrazki zapisują się na Twoim dysku, w wybranej kategorii.", "Offene Symbolsammlungen durchsuchen (Iconify). Bilder werden auf deiner Festplatte in einer Kategorie deiner Wahl gespeichert.", "Cerca nelle raccolte di icone aperte (Iconify). Le immagini vengono salvate sul disco in una categoria a scelta.", "Hae avoimista kuvakekokoelmista (Iconify). Kuvat tallennetaan levyllesi valitsemaasi luokkaan.", "搜索开放图标集（Iconify）。图片保存在你的磁盘上，类别由你选择。", "खुले आइकन संग्रह खोजें (Iconify)। चित्र आपकी डिस्क पर चुनी गई श्रेणी में सहेजे जाते हैं।"],
    ["Search the internet for more pictures and save them here", "Szukaj w internecie kolejnych obrazków i zapisuj je tutaj", "Im Internet nach weiteren Bildern suchen und hier speichern", "Cerca altre immagini su internet e salvale qui", "Etsi lisää kuvia netistä ja tallenna ne tänne", "在互联网上搜索更多图片并保存到这里", "इंटरनेट पर और चित्र खोजें और यहाँ सहेजें"],
    ["Searching…", "Szukanie…", "Suche läuft…", "Ricerca in corso…", "Haetaan…", "正在搜索…", "खोज जारी…"],
    ["Secondary grid", "Siatka pomocnicza", "Hilfsraster", "Griglia secondaria", "Apuruudukko", "辅助网格", "सहायक ग्रिड"],
    ["Select (V)", "Zaznaczanie (V)", "Auswahl (V)", "Selezione (V)", "Valinta (V)", "选择 (V)", "चयन (V)"],
    ["Select a shape first, then edit its nodes.", "Najpierw zaznacz kształt, potem edytuj jego węzły.", "Zuerst eine Form auswählen, dann ihre Knoten bearbeiten.", "Seleziona prima una forma, poi modifica i suoi nodi.", "Valitse ensin muoto ja muokkaa sitten sen solmuja.", "请先选择形状，再编辑其节点。", "पहले एक आकृति चुनें, फिर उसके नोड संपादित करें।"],
    ["Select all", "Zaznacz wszystko", "Alles auswählen", "Seleziona tutto", "Valitse kaikki", "全选", "सभी चुनें"],
    ["Select at least two objects to group.", "Zaznacz co najmniej dwa obiekty do zgrupowania.", "Wähle mindestens zwei Objekte zum Gruppieren.", "Seleziona almeno due oggetti da raggruppare.", "Valitse vähintään kaksi ryhmitettävää kohdetta.", "请至少选择两个对象进行编组。", "समूह बनाने के लिए कम से कम दो ऑब्जेक्ट चुनें।"],
    ["Send to back", "Przesuń na spód", "In den Hintergrund", "Porta in secondo piano", "Vie taakse", "置于底层", "सबसे पीछे भेजें"],
    ["Shapes", "Kształty", "Formen", "Forme", "Muodot", "形状", "आकार"],
    ["Show", "Pokaż", "Anzeigen", "Mostra", "Näytä", "显示", "दिखाएँ"],
    ["Show or hide the grid", "Pokaż lub ukryj siatkę", "Raster ein- oder ausblenden", "Mostra o nascondi la griglia", "Näytä tai piilota ruudukko", "显示或隐藏网格", "ग्रिड दिखाएँ या छिपाएँ"],
    ["Show or hide the secondary grid", "Pokaż lub ukryj siatkę pomocniczą", "Feines Raster ein- oder ausblenden", "Mostra o nascondi la griglia secondaria", "Näytä tai piilota apuruudukko", "显示或隐藏辅助网格", "सहायक ग्रिड दिखाएँ या छिपाएँ"],
    ["Simplify", "Upraszczanie", "Vereinfachen", "Semplifica", "Yksinkertaista", "简化", "सरल करें"],
    ["Size", "Rozmiar", "Größe", "Dimensione", "Koko", "大小", "आकार"],
    ["Size when placed", "Rozmiar po wstawieniu", "Größe beim Einfügen", "Dimensione all'inserimento", "Koko lisättäessä", "放置时的大小", "रखते समय आकार"],
    ["Smooth", "Gładki", "Glatt", "Morbido", "Pehmeä", "平滑", "चिकना"],
    ["Smooth curves", "Gładkie krzywe", "Glatte Kurven", "Curve morbide", "Pehmeät käyrät", "平滑曲线", "चिकने वक्र"],
    ["Snap to grid", "Przyciągaj do siatki", "Am Raster einrasten", "Aggancia alla griglia", "Kohdista ruudukkoon", "对齐网格", "ग्रिड पर स्नैप करें"],
    ["Snap to objects", "Przyciągaj do obiektów", "An Objekten einrasten", "Aggancia agli oggetti", "Kohdista objekteihin", "对齐对象", "ऑब्जेक्ट पर स्नैप करें"],
    ["Source", "Źródło", "Quelle", "Fonte", "Lähde", "来源", "स्रोत"],
    ["Spiral (I)", "Spirala (I)", "Spirale (I)", "Spirale (I)", "Spiraali (I)", "螺旋 (I)", "सर्पिल (I)"],
    ["Star (S)", "Gwiazda (S)", "Stern (S)", "Stella (S)", "Tähti (S)", "星形 (S)", "तारा (S)"],
    ["Symbols", "Symbole", "Symbole", "Simboli", "Symbolit", "符号", "प्रतीक"],
    ["Text (T)", "Tekst (T)", "Text (T)", "Testo (T)", "Teksti (T)", "文字 (T)", "टेक्स्ट (T)"],
    ["The result is empty.", "Wynik jest pusty.", "Das Ergebnis ist leer.", "Il risultato è vuoto.", "Tulos on tyhjä.", "结果为空。", "परिणाम खाली है।"],
    ["The secondary grid must be finer than the main grid.", "Siatka pomocnicza musi być gęstsza niż główna.", "Das Hilfsraster muss feiner als das Hauptraster sein.", "La griglia secondaria deve essere più fine di quella principale.", "Apuruudukon on oltava tiheämpi kuin pääruudukon.", "辅助网格必须比主网格更细。", "सहायक ग्रिड मुख्य ग्रिड से बारीक होनी चाहिए।"],
    ["The space around the work area", "Przestrzeń wokół pola roboczego", "Der Raum um die Arbeitsfläche", "Lo spazio intorno al piano di lavoro", "Työalueen ympärillä oleva tila", "工作区域周围的空间", "कार्य क्षेत्र के चारों ओर की जगह"],
    ["There are no straight corners to round.", "Brak prostych narożników do zaokrąglenia.", "Es gibt keine geraden Ecken zum Abrunden.", "Non ci sono angoli retti da arrotondare.", "Pyöristettäviä suoria kulmia ei ole.", "没有可倒圆的直线角。", "गोल करने के लिए कोई सीधा कोना नहीं है।"],
    ["Threshold", "Próg", "Schwellwert", "Soglia", "Kynnys", "阈值", "थ्रेशोल्ड"],
    ["To curve", "Na krzywą", "Zu Kurve", "In curva", "Käyräksi", "转为曲线", "वक्र में"],
    ["To line", "Na linię", "Zu Linie", "In linea", "Viivaksi", "转为直线", "रेखा में"],
    ["Tone", "Tony", "Tonwerte", "Tonalità", "Sävy", "色调", "टोन"],
    ["Tools", "Narzędzia", "Werkzeuge", "Strumenti", "Työkalut", "工具", "औज़ार"],
    ["Top", "Góra", "Oben", "Alto", "Ylä", "上", "ऊपर"],
    ["Trace", "Wektoryzuj", "Nachzeichnen", "Vettorializza", "Jäljitä", "描摹", "ट्रेस"],
    ["Trace image", "Wektoryzacja obrazu", "Bild nachzeichnen", "Vettorializzazione immagine", "Kuvan jäljitys", "描摹图像", "छवि ट्रेस"],
    ["Trace the light areas", "Wektoryzuj jasne obszary", "Helle Bereiche nachzeichnen", "Vettorializza le aree chiare", "Jäljitä vaaleat alueet", "描摹浅色区域", "हल्के क्षेत्र ट्रेस करें"],
    ["Traced {} outlines", "Zwektoryzowano konturów: {}", "{} Konturen nachgezeichnet", "Tracciati {} contorni", "Jäljitetty {} ääriviivaa", "已描摹 {} 条轮廓", "{} रूपरेखाएँ ट्रेस की गईं"],
    ["Triangle (Y)", "Trójkąt (Y)", "Dreieck (Y)", "Triangolo (Y)", "Kolmio (Y)", "三角形 (Y)", "त्रिभुज (Y)"],
    ["Undo", "Cofnij", "Rückgängig", "Annulla", "Kumoa", "撤销", "पूर्ववत करें"],
    ["Ungroup", "Rozgrupuj", "Gruppierung aufheben", "Separa", "Pura ryhmitys", "取消编组", "समूह हटाएँ"],
    ["Unlock", "Odblokuj", "Entsperren", "Sblocca", "Avaa lukitus", "解锁", "अनलॉक"],
    ["Unlocked {} object(s).", "Odblokowano obiektów: {}.", "{} Objekt(e) entsperrt.", "Sbloccati {} oggetti.", "Lukitus avattu {} kohteelta.", "已解锁 {} 个对象。", "{} ऑब्जेक्ट अनलॉक किए गए।"],
    ["Updates", "Aktualizacje", "Updates", "Aggiornamenti", "Päivitykset", "更新", "अपडेट"],
    ["Version {}", "Wersja {}", "Version {}", "Versione {}", "Versio {}", "版本 {}", "संस्करण {}"],
    ["View options", "Opcje widoku", "Ansichtsoptionen", "Opzioni di visualizzazione", "Näkymän asetukset", "视图选项", "दृश्य विकल्प"],
    ["Width", "Szerokość", "Breite", "Larghezza", "Leveys", "宽度", "चौड़ाई"],
    ["Work area", "Obszar roboczy", "Arbeitsbereich", "Area di lavoro", "Työalue", "工作区域", "कार्य क्षेत्र"],
    ["Workspace colour", "Kolor obszaru roboczego", "Farbe des Arbeitsbereichs", "Colore dell'area di lavoro", "Työtilan väri", "工作区颜色", "कार्यक्षेत्र का रंग"],
    ["You have the newest version.", "Masz najnowszą wersję.", "Sie haben die neueste Version.", "Hai l'ultima versione.", "Sinulla on uusin versio.", "您使用的是最新版本。", "आपके पास नवीनतम संस्करण है।"],
    ["You have {}.", "Masz wersję {}.", "Sie haben {}.", "Hai la versione {}.", "Sinulla on {}.", "您当前使用的是 {}。", "आपके पास {} है।"],
    ["Zoom in", "Powiększ", "Vergrößern", "Ingrandisci", "Lähennä", "放大", "ज़ूम इन"],
    ["Zoom in (Z)", "Powiększ (Z)", "Vergrößern (Z)", "Ingrandisci (Z)", "Lähennä (Z)", "放大 (Z)", "ज़ूम इन (Z)"],
    ["Zoom out", "Pomniejsz", "Verkleinern", "Riduci", "Loitonna", "缩小", "ज़ूम आउट"],
    ["Zoom out (X)", "Pomniejsz (X)", "Verkleinern (X)", "Riduci (X)", "Loitonna (X)", "缩小 (X)", "ज़ूम आउट (X)"],
    ["comes with the program", "dostarczony z programem", "mit dem Programm geliefert", "fornita con il programma", "tulee ohjelman mukana", "随程序提供", "प्रोग्राम के साथ आया"],
    ["credit the author", "podaj autora", "Urheber nennen", "cita l'autore", "mainitse tekijä", "需署名作者", "लेखक का श्रेय दें"],
    ["e.g. cat, flower, gear", "np. kot, kwiat, koło zębate", "z. B. Katze, Blume, Zahnrad", "es. gatto, fiore, ingranaggio", "esim. kissa, kukka, hammaspyörä", "例如：猫、花、齿轮", "जैसे: बिल्ली, फूल, गियर"],
    ["no credit needed", "bez obowiązku podania autora", "keine Namensnennung nötig", "nessun credito richiesto", "ei tekijämainintaa", "无需署名", "श्रेय आवश्यक नहीं"],
    ["or a new category", "lub nowa kategoria", "oder eine neue Kategorie", "oppure una nuova categoria", "tai uusi luokka", "或新类别", "या नई श्रेणी"],
    ["yours", "Twój", "von dir", "tua", "oma", "你的", "आपका"],
    ["{} outlines", "Liczba konturów: {}", "{} Konturen", "{} contorni", "{} ääriviivaa", "{} 条轮廓", "{} रूपरेखाएँ"],
    ["{} pictures found", "Znaleziono obrazków: {}", "{} Bilder gefunden", "Trovate {} immagini", "Löytyi {} kuvaa", "找到 {} 张图片", "{} चित्र मिले"],
    ["{} × {} px", "{} × {} px", "{} × {} px", "{} × {} px", "{} × {} px", "{} × {} 像素", "{} × {} पिक्सेल"],
    ["Aborted (soft reset)", "Przerwano (reset programowy)", "Abgebrochen (Soft-Reset)", "Interrotto (soft reset)", "Keskeytetty (ohjelmallinen nollaus)", "已中止（软复位）", "रोका गया (सॉफ़्ट रीसेट)"],
    ["Animation speed", "Szybkość animacji", "Geschwindigkeit der Animation", "Velocità dell'animazione", "Animaation nopeus", "动画速度", "एनिमेशन की गति"],
    ["Back to the start", "Z powrotem na początek", "Zurück zum Anfang", "Torna all'inizio", "Takaisin alkuun", "回到开头", "शुरुआत पर वापस"],
    ["Cannot open {}: {}", "Nie można otworzyć {}: {}", "{} kann nicht geöffnet werden: {}", "Impossibile aprire {}: {}", "Kohdetta {} ei voi avata: {}", "无法打开 {}：{}", "{} नहीं खुल सका: {}"],
    ["Connected to {}", "Połączono z {}", "Verbunden mit {}", "Connesso a {}", "Yhdistetty: {}", "已连接到 {}", "{} से कनेक्ट हुआ"],
    ["Connection lost", "Utracono połączenie", "Verbindung verloren", "Connessione persa", "Yhteys katkesi", "连接丢失", "कनेक्शन टूट गया"],
    ["Disconnected", "Rozłączono", "Getrennt", "Disconnesso", "Ei yhteyttä", "未连接", "डिस्कनेक्टेड"],
    ["No answer from the controller for {} s while a job is running (state: {}).", "Brak odpowiedzi sterownika od {} s podczas zadania (stan: {}).", "Seit {} s keine Antwort der Steuerung während eines Auftrags (Zustand: {}).", "Nessuna risposta dal controller da {} s durante un lavoro (stato: {}).", "Ohjain ei ole vastannut {} s työn aikana (tila: {}).", "作业运行期间控制器已 {} 秒没有应答（状态：{}）。", "काम चलते समय कंट्रोलर से {} s से कोई जवाब नहीं (स्थिति: {})।"],
    ["Pause", "Pauza", "Pause", "Pausa", "Tauko", "暂停", "रोकें"],
    ["Play", "Odtwarzaj", "Abspielen", "Riproduci", "Toista", "播放", "चलाएँ"],
    ["Repeat", "Powtarzaj", "Wiederholen", "Ripeti", "Toista", "重复", "दोहराएँ"],
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
    const SOURCES: [&str; 13] = [
        include_str!("editor/host.rs"),
        include_str!("editor/commands.rs"),
        include_str!("editor/canvas.rs"),
        include_str!("editor/bars.rs"),
        include_str!("menu.rs"),
        include_str!("window.rs"),
        include_str!("widgets.rs"),
        include_str!("dialogs.rs"),
        include_str!("editor/prefs.rs"),
        include_str!("editor/props.rs"),
        include_str!("start.rs"),
        include_str!("editor/image_ui.rs"),
        include_str!("clipart_ui.rs"),
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
