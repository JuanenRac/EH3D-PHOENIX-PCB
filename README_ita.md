<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-PCB
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | 🇮🇹 <b>Italiano</b> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D-PHOENIX-PCB: pianificato come un ambiente EDA con acquisizione di schemi, layout di PCB, strumenti di verifica, simulazione di circuiti e moduli specializzati per alimentatori a commutazione e RF. Finora è scritto solo l'inventario del corpus.

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20Python-blue.svg" alt="Rust and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-431-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** un workspace Rust il cui comando `phoenix inventory` conta i file di progetto di una cartella privata senza mostrare alcun nome né valore: file per estensione, tipo e versione dichiarata, e un'impronta del contenuto. Ha 952 test propri e passano su un computer. È stato eseguito solo su cartelle create dai suoi stessi test, mai su veri file di progetto. Ha anche le fondamenta di un modello di progetto (valori interi in nanometri e microgradi con intervallo verificato, id di oggetto e JSON canonico), il file di progetto `.phoenixproj` (manifesto, domini, migrazioni) e un modello di scheda con operazioni reversibili (livelli, reti, componenti, piste, vie, zone), il tutto provato solo su dati generati. La libreria della scheda trova anche quale rame tocca quale e verifica dimensioni e distanze con un file di regole; una quinta libreria legge librerie, schede e schemi EAGLE XML in un riepilogo e riscrive l'albero XML in modo identico. Tutto è stato provato solo su file generati o fatti a mano. Esistono anche un modello di schema e il suo controllo di coerenza con la scheda. Non ci sono importazione di progetti altrui nel modello, editor, e le regole coprono solo una parte dei controlli previsti: gran parte della funzione EDA è ancora pianificata. Il modello della scheda contiene anche testo e tratti di serigrafia (con un carattere a tratti incorporato) che il controllo delle regole e l'esportazione Gerber usano, e lo schema può essere disegnato su fogli con parti posizionate e fili che vengono confrontati con l'elenco delle reti e con la scheda; una scheda può essere scritta come file XML di EAGLE e riletta, cosa verificata solo con file scritti da questo codice e mai aperta in EAGLE; il contratto per le coppie di file binari EAGLE e del loro XML è scritto, ma non esiste alcun lettore dei file binari. Un calcolatore di stratificazione e linee di trasmissione (`phoenix stackup`: impedenza, permittività effettiva, perdite, ritardo e larghezza della pista di microstriscia, stripline, guida coplanare e coppie accoppiate con formule pubblicate) è un prototipo, mai confrontato con un simulatore di campo né con una misura. Una libreria di connettività e regole (`eh3d-phoenix-rules`) legge i limiti da un file di regole o dalle regole di progetto di una scheda EAGLE, controlla una scheda in un ordine fisso (larghezza, via, foro, anello, distanza, bordo, distanza tra i fori, solder mask, zone, serigrafia), trova i cortocircuiti e le reti non instradate e confronta le reti di una scheda con quelle di uno schema, e i comandi `phoenix check` e `phoenix connectivity` stampano il rapporto con i risultati confermati e non conclusivi separati; è stata provata solo su schede fatte a mano e non è mai stata confrontata con lo strumento di riferimento. L'esportazione di fabbricazione (`phoenix export`) produce file Gerber X2 per ogni strato, file di foratura Excellon metallizzati e non metallizzati, un file di posizionamento e una distinta base con profili letti da un file JSON, un manifesto con somme SHA-256 e un controllo che rilegge i file e li confronta con la scheda; è un prototipo i cui file sono stati letti solo dai parser di questo repository, mai aperti in un visualizzatore indipendente né da un produttore. Un set di strumenti RF (`eh3d-phoenix-rf`, comando `phoenix rf`) legge e scrive file Touchstone, converte i parametri S, Z, Y e ABCD, calcola ROS, perdite, ritardo di gruppo e stabilità, progetta reti a L, adattatori a stub singolo e sezioni a quarto d'onda per impedenze complesse, fornisce le coordinate della carta di Smith e progetta filtri a scala di Butterworth, Chebyshev e Bessel con componenti ideali; è un prototipo verificato solo con valori da manuale, mai confrontato con un analizzatore di reti, un simulatore di campo né un altro programma RF. Un lettore KiCad (`eh3d-phoenix-kicad`, comando `phoenix kicad-read`) legge le librerie di simboli, le impronte, gli schemi con i loro fogli, le schede e i file di progetto di KiCad da 6 a 9 nello stesso modello dei file EAGLE, con un rapporto di importazione e un livello di fedeltà; è un prototipo provato solo su file scritti a mano per i suoi test, mai confrontato con KiCad né eseguito su una raccolta pubblica di progetti. Un sanificatore di librerie (`eh3d-phoenix-sanitizer`, comando `phoenix sanitize`) legge una cartella di vecchie librerie EAGLE e segnala le impronte, i simboli e i dispositivi duplicati o quasi duplicati, con un hash geometrico canonico, un punteggio rispetto a una soglia, un rapporto HTML e JSON e un piano della libreria principale; si limita ad analizzare, non riscrive mai una libreria e non applica mai il piano, ed è un prototipo provato solo su librerie fatte dai suoi test, mai su librerie reali. Un controllo elettrico (`eh3d-phoenix-erc`, comando `phoenix erc`) giudica uno schema: pin su nessuna rete, reti con un solo pin, tipi di pin che una matrice di un file di impostazioni non consente insieme, ingressi senza pilota, alimentazioni senza sorgente, nomi di rete simili, riferimenti doppi, valori vuoti, componenti senza impronta e fili sciolti, con i risultati confermati e non conclusivi separati; una sincronizzazione (`eh3d-phoenix-sync`, comando `phoenix sync`) confronta uno schema e una scheda tramite l'identità dei loro componenti ed esegue le modifiche del trasferimento alla scheda, o dell'annotazione all'indietro nello schema, come un pacchetto provato prima a vuoto che si può annullare e ripetere; il file delle regole del secondo formato contiene anche classi di rete con limiti propri per piste e via. Tutto è un prototipo provato solo su schemi e schede fatti a mano e dai test, mai confrontato con lo strumento di riferimento, e non esiste ancora un editor di schemi. Un simulatore di circuiti (`eh3d-phoenix-sim`, comando `phoenix sim`) legge e scrive netlist in stile SPICE, traduce uno schema in una netlist e risolve circuiti lineari e con diodi con un'analisi nodale modificata propria (punto di lavoro in continua, scansioni in alternata con ritardo di gruppo e analisi transitoria), con un controllo rispetto ai filtri a scala degli strumenti RF; è un prototipo provato con valori da manuale, mai confrontato con SPICE né con una misura.

---

> **Stato di sicurezza funzionale e informatica:** questo repository è ai primi passi e non è un sistema di sicurezza funzionale. Nulla di quanto scritto qui sostituisce i limiti, gli interblocchi e l'arresto di emergenza della macchina e del suo controllore.

## 📋 In breve

| | |
|---|---|
| Versione | `0.0.2` |
| Maturità | Impalcatura |
| Test | 431 |
| Stack tecnologico | Rust, Python 3.10+ |
| Su hardware reale | Non ancora eseguito su hardware |

**Vai a:** [Panoramica](#-panoramica) · [Architettura](#-architettura) · [Avvio rapido](#-avvio-rapido) · [Test](#-test) · [Struttura del repository](#-struttura-del-repository) · [Progetti correlati](#-progetti-correlati)

## 🚦 Stato del progetto

Ogni riga è una parte di questo repository e il semaforo dice a che punto è oggi. È scritta in base a ciò che il repository può mostrare (test, compilazioni, esecuzioni), non a ciò che è previsto.

🟢 Fatto e verificato · 🟡 Funziona, con limiti noti · 🟠 Fatto a metà o non ancora verificato · 🔴 Non fatto o non funzionante

| Area | Stato | Cosa significa qui |
|---|---|---|
| Controlli del repository | 🟡 | Il controllo del repository, quello delle immagini e i 3 test della base girano nella CI e passano. |
| Test | 🟡 | I 952 test del codice Rust e i 3 della base passano su un computer. Usano solo cartelle create dai test stessi; nessuno è stato eseguito su veri file di progetto. |
| Hardware reale | 🟠 | Non eseguito su nessuna macchina né scheda: nulla qui è stato provato su hardware. |
| Funzione pianificata | 🟠 | Scritto: l'inventario oscurato di un corpus, le fondamenta del modello (unità, id, JSON canonico), il file di progetto `.phoenixproj`, il modello di scheda con annulla e ripeti, la sua connettività e un primo controllo delle regole, e un lettore di riepilogo di EAGLE XML. Non iniziato: l'editor di schemi, l'importazione di formati diversi da KiCad nel modello, gli scrittori di altri formati, gran parte delle regole, gli editor e il visualizzatore definitivo, il confronto del simulatore con SPICE e tutto il resto del pianificato. Il modello della scheda contiene anche testo e tratti di serigrafia (con un carattere a tratti incorporato) che il controllo delle regole e l'esportazione Gerber usano, e lo schema può essere disegnato su fogli con parti posizionate e fili che vengono confrontati con l'elenco delle reti e con la scheda; una scheda può essere scritta come file XML di EAGLE e riletta, cosa verificata solo con file scritti da questo codice e mai aperta in EAGLE; il contratto per le coppie di file binari EAGLE e del loro XML è scritto, ma non esiste alcun lettore dei file binari. Un calcolatore di stratificazione e linee di trasmissione (`phoenix stackup`: impedenza, permittività effettiva, perdite, ritardo e larghezza della pista di microstriscia, stripline, guida coplanare e coppie accoppiate con formule pubblicate) è un prototipo, mai confrontato con un simulatore di campo né con una misura. Un set di strumenti RF (`phoenix rf`: file Touchstone, parametri a due porte, adattamento e filtri a scala ideali) è un prototipo, verificato solo con valori da manuale. Un lettore KiCad (`phoenix kicad-read`: simboli, impronte, schemi con fogli, schede e file di progetto) è un prototipo, provato solo su file scritti a mano. Un sanificatore di librerie (`phoenix sanitize`: inventario di una cartella di librerie EAGLE, hash geometrico, confronto approssimato con soglia, gruppi di duplicati, rapporto HTML e JSON e un piano della libreria principale che non viene mai applicato) è un prototipo, provato solo su librerie fatte dai suoi test. Un controllo elettrico (`phoenix erc`) e la sincronizzazione di uno schema e di una scheda (`phoenix sync`: trasferimento alla scheda e annotazione all'indietro, come un pacchetto che si può annullare) sono prototipi, provati solo su schemi e schede fatti a mano. Un simulatore di circuiti proprio (`phoenix sim`: netlist in stile SPICE, la netlist di uno schema e analisi in continua, in alternata e transitoria di circuiti lineari e con diodi) è un prototipo, verificato solo con valori da manuale e mai confrontato con SPICE. |
| Finestra | 🟠 | Il visualizzatore di schede e schemi in sola lettura (`phoenix view`, egui, dietro la funzionalità `gui`) è scritto e la sua logica è collaudata, ma la finestra non è mai stata vista su uno schermo: il disegno, il mouse e la fluidità a schermo non sono verificati. |
| Requisiti | 🟠 | 15 requisiti su 30 (la modalità oscurata, le fondamenta del modello, le operazioni reversibili, il file di progetto, i codici di diagnostica, i calcolatori RF analitici, i controlli di connettività e di regole e l'esportazione di fabbricazione con il suo pacchetto, il visualizzatore di schede e schemi i lettori KiCad, il sanificatore di librerie, i controlli e la sincronizzazione dello schema e il simulatore di circuiti) sono a livello di prototipo, 15 sono solo pianificati e nessuno è convalidato. |

## 🎯 Panoramica

* **Cosa esiste:** un workspace con una libreria che conta un corpus privato di file di progetto in modalità oscurata (solo conteggi, tipi, versioni dichiarate e un'impronta, mai un nome né un valore) e una riga di comando, `phoenix inventory`. Guarda i primi byte di un file e non legge i progetti. Una seconda libreria contiene le fondamenta del modello di progetto: lunghezze e angoli come interi con intervallo verificato, id di oggetto (UUID v7) e JSON canonico. Una terza libreria legge e scrive il progetto `.phoenixproj` sul contenitore condiviso di EH3D-COMMON: il manifesto con domini, collegamenti e migrazioni, i domini che non capisce conservati intatti e la regola `must_understand`. Una quarta libreria contiene il modello di scheda: livelli, reti, impronte, componenti, piste con linee e archi, vie e zone collegate da id stabili, con operazioni reversibili e una cronologia di annulla e ripeti. Ogni errore ha un codice stabile `PHOENIX-<AREA>-<NNN>`. La libreria della scheda calcola anche le isole di ogni rete e verifica larghezze, fori, anelli e distanze con un file di regole in millimetri, tenendo separati il confermato e l'inconcludente. Una quinta libreria legge librerie, schede e schemi EAGLE XML in un riepilogo, riscrive l'albero XML in modo identico e misura il tasso di lettura su copie di un corpus verificate con SHA-256, senza modificare gli originali; il comando `phoenix eagle-rate` lo esegue. Una sesta libreria scrive i file Gerber X2, Excellon e `.eh3dproj` 1.0.0 di una scheda con EH3D-CORE-LIBS ed EH3D-COMMON, con gli stessi byte per la stessa scheda; i file vengono riletti solo dai parser della stessa libreria. Una settima libreria contiene lo schema (simboli, parti e reti collegati da id stabili) e lo confronta con una scheda: parti senza componente e differenze di designatori, valori, impronte e reti, con i codici `PHOENIX-LINK-<NNN>`. Un'altra libreria, `eh3d-phoenix-rules`, ricava le reti dal rame (pad, piste, via e zone unite dalla geometria con una tolleranza), le confronta con quelle dello schema ed esegue le regole di progetto in un ordine fisso rispetto a limiti letti da un file di regole o dalle regole di progetto di una scheda EAGLE, con i risultati confermati e non conclusivi in due elenchi; i comandi `phoenix check` e `phoenix connectivity` stampano il suo rapporto come testo in otto lingue o come JSON.

* **Pianificato:** EH3D-PHOENIX-PCB vuole essere un ambiente EDA con acquisizione di schemi, layout di PCB, strumenti di verifica, simulazione di circuiti e moduli specializzati per alimentatori a commutazione e RF. È pensato come un programma in Rust con finestra per il desktop (un workspace di crate di motore, un'interfaccia a riga di comando e un'applicazione) che funziona da solo e può usare il server per attività di calcolo. Nulla di tutto questo esiste ancora.

## 🧩 Ruolo in EH3D

EH3D-PHOENIX-PCB è il progetto principale della famiglia PHOENIX della EH3D Fabrication Suite; gli altri repository della famiglia si basano su di esso. Cosa farà, cosa non farà ancora, i suoi requisiti e le sue relazioni con altri repository sono scritti nella cartella `docs/`.

## 🏛️ Architettura

<p align="center">
  <img src="images/architecture.svg" alt="Architettura di EH3D-PHOENIX-PCB: chi lo usa, che cosa contiene e da che cosa dipende" width="100%">
</p>

Come si incastrano le parti di questo repository: chi lo usa, che cosa contiene e da che cosa dipende. L'immagine è disegnata da `docs/architecture.json` e verificata rispetto al codice.

## 🚀 Avvio rapido

Servono Python 3.10 o successivo per i controlli e una toolchain Rust per il codice.

```bash
cargo build --release
cargo test
cargo run -p eh3d-phoenix-cli -- inventory <folder>
python -I tools/ci_validate.py
python -I tools/make_architecture.py --check
python -I tools/make_structure.py --check --readmes
python -I -m unittest discover -s tools -p "test_*.py"
```

## 🧪 Test

Il codice ha 952 test (`cargo test`) e la base ne ha 3 (`python -I -m unittest discover -s tools -p "test_*.py"`); tutti passano su un computer. La CI esegue il controllo del repository, quello delle immagini, i test della base e, per il codice Rust, il controllo del formato, clippy con gli avvisi trattati come errori e i test su Linux e Windows.

## 🏗️ Compilazione e rilascio

La versione sta nel file `VERSION` e in `eh3d.project.json`, e ogni versione ha la sua voce in [CHANGELOG.md](CHANGELOG.md). `cargo build --release` compila la libreria e il programma `phoenix`; le versioni vengono pubblicate su GitHub.

## 📂 Struttura del repository

<p align="center">
  <img src="images/repository-structure.svg" alt="Struttura del repository EH3D-PHOENIX-PCB" width="100%">
</p>

<details>
<summary>Versione testuale dell'albero</summary>

```text
EH3D-PHOENIX-PCB/
├── docs/
│   ├── architecture.json structure.json         i dati delle due immagini
│   ├── SCOPE.md REQUIREMENTS.md                 l'ambito e i requisiti
│   ├── DEPENDENCIES.md THREAT_MODEL.md          relazioni con altri repository e modello delle minacce
│   ├── DECISIONS.md STATUS.md                   decisioni applicabili qui e stato onesto
│   ├── NOT_YET.md RULES.md                      idee per dopo e regole per scrivere il codice
│   ├── ROADMAP.md                               la mappa delle fasi, dalle fondamenta alla suite tutto in uno
│   ├── ERROR_CODES.md                           i codici di diagnostica PHOENIX-<AREA>-<NNN>
│   ├── STACKUP.md                               calcoli di stratificazione e linee, formule
│   ├── RF.md                                    strumenti RF: Touchstone, doppi bipoli, adattamento, filtri
│   ├── KICAD.md                                 i lettori KiCad: cosa si legge da ogni formato
│   ├── VIEWER.md                                il prototipo di visualizzatore in sola lettura e il suo benchmark
│   ├── SANITIZER.md                             il sanificatore di librerie: duplicati, hash, rapporto e piano
│   ├── ERC.md                                   controllo elettrico e sincronizzazione
│   ├── SIM.md                                   simulazione di circuiti: netlist, analisi, esportazione dello schema
│   ├── GATE_F1.md                               cosa verifica il varco della prima fase e cosa no
│   └── adr/                                     i registri delle decisioni
├── AGENTS.md                                    regole per chi modifica il repository
├── deny.toml                                    politica di licenze e avvisi delle dipendenze
├── crates/                                      inventario, modello, progetto, scheda, schema, EAGLE, KiCad, regole, ERC, sincronizzazione, hub, esportazione, stratificazione, RF, simulatore, sanificatore, visualizzatore, CLI
├── config/                                      il file delle impostazioni (ogni valore regolato)
├── Cargo.toml Cargo.lock                        il workspace Rust e le sue dipendenze bloccate
├── tools/                                       il controllo del repository, i suoi test e gli strumenti delle immagini
├── images/                                      banner e le immagini di Architettura e Struttura del repository
├── .github/                                     workflow di CI e della dashboard, modelli di issue e pull request
├── eh3d.project.json                            manifesto: versione, ruolo, maturità e comando di compilazione
├── VERSION                                      la versione del repository
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

</details>

## 🗺️ Roadmap e limiti noti

**Limiti noti oggi**

* Codice iniziale: esistono solo l'inventario del corpus e le fondamenta del modello; l'inventario è stato eseguito solo su cartelle create dai suoi stessi test, mai su veri file di progetto.
* Non ci sono schema elettrico, lettori o scrittori di file di progetto di altri programmi, editor né regole; il file di progetto e il modello di scheda sono stati provati solo su dati generati, mai su un progetto reale.
* Lo stack finale dell'interfaccia utente resta aperto (una scelta provvisoria è registrata in `docs/adr/`), come la libreria di geometria (vedi `docs/STATUS.md`).

**Prossimi passi**

* Attendere che i contratti da cui dipende questo progetto siano accettati (vedi `docs/DEPENDENCIES.md`), poi scrivere il modello e i primi lettori con test e sostituire le righe di stato con risultati reali.

## 🌍 Lingue

Il README e i testi esistono in inglese, spagnolo, francese, italiano, tedesco, cinese, giapponese e portoghese europeo.

## 🔗 Progetti correlati

* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS): la mappa della suite, la sua architettura, le convenzioni e i controlli condivisi
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): costruito accanto a questo repository; il contenitore di progetto condiviso (`eh3d-container`) e la diagnostica comune (`eh3d-diagnostic`) che questo progetto usa
* Repository fratelli di PHOENIX: [EH3D-VALIDATION-LAB](https://github.com/JuanenRac/EH3D-VALIDATION-LAB) (corpus e oracolo di file di riferimento) ed [EH3D-PHOENIX-CAD](https://github.com/JuanenRac/EH3D-PHOENIX-CAD) (lato meccanico), entrambi in codice iniziale
* Relazioni pianificate, non ancora costruite (EH3D-CORE-LIBS, EH3D-SERVER, EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE, EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER, EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-SUITE-LAUNCHER, EH3D-HYDRA-BRIDGE): vedi [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md)

## 📚 Documentazione

* [Le convenzioni della suite](https://github.com/JuanenRac/EH3D-DOCS/blob/main/docs/CONVENTIONS.md)
* [Registro delle modifiche](CHANGELOG.md)
* [Ambito, requisiti, dipendenze, modello delle minacce e decisioni](docs/SCOPE.md)

## 🤝 Contribuire e comunità

* Prima di una pull request esegui `python -I tools/ci_validate.py` e i test della base: tutto deve passare. Vedi [CONTRIBUTING.md](CONTRIBUTING.md) e il [codice di condotta](CODE_OF_CONDUCT.md).
* Il README esiste in otto lingue: una modifica al testo si fa in tutti e otto i file, e le due immagini sono disegnate da `docs/architecture.json` e `docs/structure.json` con `tools/make_architecture.py` e `tools/make_structure.py`.
* Segnala una vulnerabilità in privato: [SECURITY.md](SECURITY.md). Domande, idee e segnalazioni: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTORE

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENZA

Software: GPL-3.0-or-later. Vedi [LICENSE](LICENSE). I progetti hardware della suite usano la CERN OHL-S.
