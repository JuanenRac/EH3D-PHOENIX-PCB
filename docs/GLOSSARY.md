# Glossary

The terms of the domain as this program uses them. The table at the end of the English page gives each term in the eight languages.

| Term | Meaning |
|---|---|
| **nm (nanometre)** | The unit of every length in the model, kept as a whole number: 1 mm is 1 000 000 nm, so sums and comparisons have no rounding error. |
| **net** | All the pads, tracks and vias that must be connected electrically; it has a name such as GND. |
| **net class** | A group of nets that share rules: track width, clearance and via size. The rule file of the second format holds the classes. |
| **via** | A plated hole that joins the copper of two layers. |
| **annular ring** | The ring of copper that is left around a drilled hole; if it is too thin, the drill can break it. |
| **footprint** | The copper pads, holes and drawings of a component on the board. |
| **courtyard** | The area around a footprint that no other part may enter. |
| **copper zone** | A filled area of copper, usually a ground plane, that keeps its clearance from other nets. |
| **stack-up** | The order, thickness and material of the copper and insulating layers of a board; the impedance of a line depends on it. |
| **DRC (design rule check)** | The check of a board against limits such as the narrowest track, the clearance, the drill and the annular ring. Command: `phoenix check`. |
| **ERC (electrical rule check)** | The check of a schematic: pins on no net, inputs nobody drives, supplies without a source and so on. Command: `phoenix erc`. |
| **ratsnest** | The straight lines that show which connections are still to be routed. Command: `phoenix ratsnest`. |
| **Gerber X2** | The format of the files of copper, solder mask, paste and silkscreen that a manufacturer reads; X2 adds attributes such as the function of the layer. |
| **Excellon** | The format of the drill files: the tools and the positions of the holes, plated or not. |
| **DSN / SES** | The Specctra files for an external autorouter: DSN is the design that goes out, SES the session with the routes that comes back. Command: `phoenix autoroute`. |
| **V-score** | A groove cut in a straight line between two boards of a panel, so that they can be broken apart. |
| **panel** | Several boards laid out together on one sheet, with a frame, tabs, fiducials and tooling holes, to be made and assembled at once. Command: `phoenix panel`. |
| **mouse bites** | A tab between two boards crossed by a row of small holes, so that it breaks cleanly. |
| **fiducial** | A small round mark of bare copper that the assembly machine sees to find the position of the board. |
| **pick and place** | The file with the reference, position, rotation and face of each part, for the assembly machine. Command: `phoenix pnp`. |
| **stencil** | The thin sheet with openings through which the solder paste is spread on the pads. Command: `phoenix stencil`. |
| **BOM (bill of materials)** | The list of the parts of a board, grouped by value and footprint, with quantities and prices. Command: `phoenix bom`. |
| **differential pair** | Two tracks that carry one signal as the difference of their voltages; they must keep a gap, equal lengths and a symmetry. Command: `phoenix pairs`. |
| **crosstalk (NEXT, FEXT)** | The part of a signal that appears on a neighbouring track; near-end (NEXT) is seen at the driver, far-end (FEXT) at the other end. Command: `phoenix si crosstalk`. |
| **S parameters / Touchstone** | The reflection and transmission of a network at each frequency; Touchstone (.s1p, .s2p) is the file that stores them. Command: `phoenix rf touchstone`. |
| **VSWR** | A number from 1 upwards that says how badly a load is matched to a line: 1 is a perfect match. |
| **Monte Carlo** | Many runs of a circuit in which the values vary at random within their tolerances; a fixed seed gives the same table every time. |
| **netlist** | The text that lists the elements of a circuit and the nodes they join; it is what the simulator reads. Command: `phoenix netlist`. |
| **dry run** | A run that does everything except writing: it shows what would be written or changed. Almost every command is a dry run unless `--out` names a new file. |

## The terms in the eight languages

| en | es | de | fr | it | ja | zh | pt |
|---|---|---|---|---|---|---|---|
| nm (nanometre) | nm (nanómetro) | nm (Nanometer) | nm (nanomètre) | nm (nanometro) | nm（ナノメートル） | nm（纳米） | nm (nanómetro) |
| net | red (eléctrica) | Netz | réseau (net) | rete | ネット | 网络 | rede |
| net class | clase de red | Netzklasse | classe de réseau | classe di rete | ネットクラス | 网络类 | classe de rede |
| via | vía | Via (Durchkontaktierung) | via | via | ビア | 过孔 | via |
| annular ring | anillo anular | Restring | anneau annulaire | anello anulare | アニュラリング | 焊环 | anel anular |
| footprint | huella | Footprint (Bauform) | empreinte | impronta | フットプリント | 封装 | footprint (pegada) |
| courtyard | área de cortesía (courtyard) | Courtyard (Platzbedarf) | zone d'exclusion (courtyard) | area di ingombro (courtyard) | コートヤード | 占位区（courtyard） | área de exclusão (courtyard) |
| copper zone | zona de cobre | Kupferfläche | zone de cuivre | zona di rame | 銅箔ゾーン | 铜区 | zona de cobre |
| stack-up | apilado (stack-up) | Lagenaufbau | empilage | stack-up (struttura a strati) | 層構成 | 叠层 | empilhamento |
| DRC (design rule check) | DRC (comprobación de reglas de diseño) | DRC (Entwurfsregelprüfung) | DRC (vérification des règles de conception) | DRC (controllo delle regole di progetto) | DRC（デザインルールチェック） | DRC（设计规则检查） | DRC (verificação das regras de projeto) |
| ERC (electrical rule check) | ERC (comprobación de reglas eléctricas) | ERC (elektrische Regelprüfung) | ERC (vérification des règles électriques) | ERC (controllo delle regole elettriche) | ERC（電気ルールチェック） | ERC（电气规则检查） | ERC (verificação das regras elétricas) |
| ratsnest | líneas de conexión pendientes (ratsnest) | Luftlinien (Ratsnest) | chevelu (ratsnest) | linee di collegamento (ratsnest) | ラッツネスト（未配線ライン） | 飞线（ratsnest） | linhas por ligar (ratsnest) |
| Gerber X2 | Gerber X2 | Gerber X2 | Gerber X2 | Gerber X2 | Gerber X2 | Gerber X2 | Gerber X2 |
| Excellon | Excellon | Excellon | Excellon | Excellon | Excellon | Excellon | Excellon |
| DSN / SES | DSN / SES | DSN / SES | DSN / SES | DSN / SES | DSN / SES | DSN / SES | DSN / SES |
| V-score | corte en V (V-score) | Ritzen (V-Nut) | rainure en V (V-score) | taglio a V (V-score) | Vカット | V 形槽（V-cut） | corte em V (V-score) |
| panel | panel | Nutzen | panneau | pannello | パネル（面付け） | 拼板 | painel |
| mouse bites | puentes con taladros (mouse bites) | Stege mit Perforation (Mouse Bites) | ponts à trous (mouse bites) | ponticelli forati (mouse bites) | ミシン目タブ（マウスバイト） | 邮票孔（mouse bites） | pontes com furos (mouse bites) |
| fiducial | marca fiducial | Passmarke (Fiducial) | mire de repérage (fiducial) | fiducial | フィデューシャルマーク | 光学定位点（fiducial） | marca fiducial |
| pick and place | posicionamiento de componentes (pick and place) | Bestückungsdaten (Pick and Place) | fichier de placement (pick and place) | posizionamento componenti (pick and place) | 実装位置データ（ピックアンドプレース） | 贴装坐标（pick and place） | posicionamento de componentes (pick and place) |
| stencil | plantilla de pasta (stencil) | Schablone (Stencil) | pochoir (stencil) | stencil per pasta saldante | メタルマスク（ステンシル） | 钢网（stencil） | estêncil |
| BOM (bill of materials) | BOM (lista de materiales) | BOM (Stückliste) | BOM (nomenclature) | BOM (distinta base) | BOM（部品表） | BOM（物料清单） | BOM (lista de materiais) |
| differential pair | par diferencial | Differenzialpaar | paire différentielle | coppia differenziale | 差動ペア | 差分对 | par diferencial |
| crosstalk (NEXT, FEXT) | diafonía (NEXT, FEXT) | Übersprechen (NEXT, FEXT) | diaphonie (NEXT, FEXT) | diafonia (NEXT, FEXT) | クロストーク（NEXT、FEXT） | 串扰（NEXT、FEXT） | diafonia (NEXT, FEXT) |
| S parameters / Touchstone | parámetros S / Touchstone | S-Parameter / Touchstone | paramètres S / Touchstone | parametri S / Touchstone | S パラメータ／Touchstone | S 参数／Touchstone | parâmetros S / Touchstone |
| VSWR | ROE (relación de onda estacionaria) | Stehwellenverhältnis (VSWR) | TOS (taux d'onde stationnaire) | ROS (rapporto d'onda stazionaria) | VSWR（電圧定在波比） | 驻波比（VSWR） | ROE (razão de onda estacionária) |
| Monte Carlo | Monte Carlo | Monte Carlo | Monte Carlo | Monte Carlo | モンテカルロ | 蒙特卡洛 | Monte Carlo |
| netlist | netlist (lista de conexiones) | Netzliste | netlist (liste de connexions) | netlist | ネットリスト | 网表 | netlist (lista de ligações) |
| dry run | simulacro | Probelauf | essai à blanc | prova a secco | ドライラン | 试运行 | simulacro |

## Languages

* [Español](glossary/es/GLOSSARY.md)
* [Deutsch](glossary/de/GLOSSARY.md)
* [Français](glossary/fr/GLOSSARY.md)
* [Italiano](glossary/it/GLOSSARY.md)
* [日本語](glossary/ja/GLOSSARY.md)
* [中文](glossary/zh/GLOSSARY.md)
* [Português (Portugal)](glossary/pt/GLOSSARY.md)
