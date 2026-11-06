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
  <img src="https://img.shields.io/badge/Tests-3598-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** un workspace Rust il cui comando `phoenix inventory` conta i file di progetto di una cartella privata senza mostrare alcun nome né valore: file per estensione, tipo e versione dichiarata, e un'impronta del contenuto. Ha 3598 test propri e passano su un computer. È stato eseguito solo su cartelle create dai suoi stessi test, mai su veri file di progetto. Ha anche le fondamenta di un modello di progetto (valori interi in nanometri e microgradi con intervallo verificato, id di oggetto e JSON canonico), il file di progetto `.phoenixproj` (manifesto, domini, migrazioni) e un modello di scheda con operazioni reversibili (livelli, reti, componenti, piste, vie, zone), il tutto provato solo su dati generati. La libreria della scheda trova anche quale rame tocca quale e verifica dimensioni e distanze con un file di regole; una quinta libreria legge librerie, schede e schemi EAGLE XML in un riepilogo e riscrive l'albero XML in modo identico. Tutto è stato provato solo su file generati o fatti a mano. Esistono anche un modello di schema e il suo controllo di coerenza con la scheda. Non ci sono importazione di progetti altrui nel modello, editor, e le regole coprono solo una parte dei controlli previsti: gran parte della funzione EDA è ancora pianificata. Il modello della scheda contiene anche testo e tratti di serigrafia (con un carattere a tratti incorporato) che il controllo delle regole e l'esportazione Gerber usano, e lo schema può essere disegnato su fogli con parti posizionate e fili che vengono confrontati con l'elenco delle reti e con la scheda; una scheda può essere scritta come file XML di EAGLE e riletta, cosa verificata solo con file scritti da questo codice e mai aperta in EAGLE; il contratto per le coppie di file binari EAGLE e del loro XML è scritto, ma non esiste alcun lettore dei file binari. Un calcolatore di stratificazione e linee di trasmissione (`phoenix stackup`: impedenza, permittività effettiva, perdite, ritardo e larghezza della pista di microstriscia, stripline, guida coplanare e coppie accoppiate con formule pubblicate) è un prototipo, mai confrontato con un simulatore di campo né con una misura. Con la base dati dei materiali di EH3D-COMMON (`phoenix materials`, `phoenix stackup --material ID`) Dk e Df di uno strato derivano dai punti pubblicati da una scheda tecnica, mai estrapolati; nessuno di questi valori è stato confrontato con la misura di una scheda. Una libreria di connettività e regole (`eh3d-phoenix-rules`) legge i limiti da un file di regole o dalle regole di progetto di una scheda EAGLE, controlla una scheda in un ordine fisso (larghezza, via, foro, anello, distanza, bordo, distanza tra i fori, solder mask, zone, serigrafia), trova i cortocircuiti e le reti non instradate e confronta le reti di una scheda con quelle di uno schema, e i comandi `phoenix check` e `phoenix connectivity` stampano il rapporto con i risultati confermati e non conclusivi separati; è stata provata solo su schede fatte a mano e non è mai stata confrontata con lo strumento di riferimento. Altri controlli delle regole di progetto (`phoenix check --advanced`: anello anulare, distanza dal bordo della scheda, trappole d'acido, distanza in base alla tensione, isole di rame, piste cieche, via duplicate, pad senza rete e componenti fuori dal contorno, con limiti e livelli in `config/drc_advanced.cfg`) sono anch'essi un prototipo, provato solo su schede fatte a mano e mai confrontato con le regole di un produttore né con un altro strumento; i valori della tabella delle distanze non sono verificati con la norma. L'esportazione di fabbricazione (`phoenix export`) produce file Gerber X2 per ogni strato, file di foratura Excellon metallizzati e non metallizzati, un file di posizionamento e una distinta base con profili letti da un file JSON, un manifesto con somme SHA-256 e un controllo che rilegge i file e li confronta con la scheda; è un prototipo i cui file sono stati letti solo dai parser di questo repository, mai aperti in un visualizzatore indipendente né da un produttore. Sopra di essa, `phoenix pnp`, `phoenix stencil` e `phoenix fabpack` scrivono file di posizionamento, le aperture di uno stencil per pasta saldante e un pacchetto zip deterministico con un manifesto; sono prototipi mai caricati in una macchina, mai trasformati in stencil né inviati a un produttore. Un set di strumenti RF (`eh3d-phoenix-rf`, comando `phoenix rf`) legge e scrive file Touchstone, converte i parametri S, Z, Y e ABCD, calcola ROS, perdite, ritardo di gruppo e stabilità, progetta reti a L, adattatori a stub singolo e sezioni a quarto d'onda per impedenze complesse, fornisce le coordinate della carta di Smith e progetta filtri a scala di Butterworth, Chebyshev e Bessel con componenti ideali; è un prototipo verificato solo con valori da manuale, mai confrontato con un analizzatore di reti, un simulatore di campo né un altro programma RF. Un lettore KiCad (`eh3d-phoenix-kicad`, comando `phoenix kicad-read`) legge le librerie di simboli, le impronte, gli schemi con i loro fogli, le schede e i file di progetto di KiCad da 6 a 9 nello stesso modello dei file EAGLE, con un rapporto di importazione e un livello di fedeltà; è un prototipo provato solo su file scritti a mano per i suoi test, mai confrontato con KiCad né eseguito su una raccolta pubblica di progetti. Un sanificatore di librerie (`eh3d-phoenix-sanitizer`, comando `phoenix sanitize`) legge una cartella di vecchie librerie EAGLE e segnala le impronte, i simboli e i dispositivi duplicati o quasi duplicati, con un hash geometrico canonico, un punteggio rispetto a una soglia, un rapporto HTML e JSON e un piano della libreria principale; si limita ad analizzare, non riscrive mai una libreria e non applica mai il piano, ed è un prototipo provato solo su librerie fatte dai suoi test, mai su librerie reali. Un controllo elettrico (`eh3d-phoenix-erc`, comando `phoenix erc`) giudica uno schema: pin su nessuna rete, reti con un solo pin, tipi di pin che una matrice di un file di impostazioni non consente insieme, ingressi senza pilota, alimentazioni senza sorgente, nomi di rete simili, riferimenti doppi, valori vuoti, componenti senza impronta, fili sciolti, reti che nulla pilota, pin di alimentazione dei chip senza condensatore di disaccoppiamento, nomi di rete che possono essere errori di battitura ed etichette che non toccano nulla, con i risultati confermati e non conclusivi separati; una sincronizzazione (`eh3d-phoenix-sync`, comando `phoenix sync`) confronta uno schema e una scheda tramite l'identità dei loro componenti ed esegue le modifiche del trasferimento alla scheda, o dell'annotazione all'indietro nello schema, come un pacchetto provato prima a vuoto che si può annullare e ripetere, con un piano dell'annotazione all'indietro che dà a ogni modifica uno stato (applicabile, conflitto o ignorata) e un codice di conferma (`phoenix annotate`) e un confronto dei due documenti (`phoenix sync --diff`); il file delle regole del secondo formato contiene anche classi di rete con limiti propri per piste e via. Tutto è un prototipo provato solo su schemi e schede fatti a mano e dai test, mai confrontato con lo strumento di riferimento, e anche l'editor di schemi (il comando `phoenix schedit` e una modalità di modifica della finestra) è un prototipo, provato solo senza schermo: operazioni che si possono annullare, una netlist che segue il disegno, il controllo delle regole dopo ogni modifica e il trasferimento alla scheda. Un simulatore di circuiti (`eh3d-phoenix-sim`, comando `phoenix sim`) legge e scrive netlist in stile SPICE, traduce uno schema in una netlist e risolve circuiti lineari, con diodi (con resistenza serie, capacità di giunzione e breakdown), con transistor bipolari e MOS, con interruttori controllati in tensione e con macromodelli di amplificatore operazionale con un'analisi nodale modificata propria (punto di lavoro in continua e scansione in continua di una sorgente, scansioni in alternata con ritardo di gruppo, rumore e analisi transitoria con un passo che si ferma agli spigoli delle sorgenti), con un controllo rispetto ai filtri a scala degli strumenti RF; legge anche le righe `.op`, `.dc`, `.tran` e `.ac` di una netlist e l'`.include` di file nella stessa cartella; è un prototipo provato con valori da manuale e soluzioni in forma chiusa, mai confrontato con SPICE né con una misura, e i suoi modelli dei dispositivi sono semplici. Fa anche scansioni di liste di valori, misure sui risultati, Monte Carlo sulle tolleranze con seme fisso, e trova rumore, poli e zeri da un adattamento della risposta AC; è anch'esso un prototipo, verificato su casi a soluzione chiusa e mai confrontato con SPICE. Una libreria per la logica di modifica e instradamento (`eh3d-phoenix-route`, comandi `phoenix route` e `phoenix ratsnest`) elenca i collegamenti ancora da instradare e instrada tra due elementi di una rete con tratti a 0, 45 e 90 gradi, fermandosi davanti a un ostacolo o aggirandolo, con la larghezza della classe di rete e via dimensionate dalle regole; le sue modifiche sono operazioni reversibili che segnalano ciò che aggiungono al controllo delle regole. È un prototipo provato su schede fatte a mano e dai test, senza archi né zone di rame; la finestra che modifica con esso (`phoenix view`) non è mai stata vista su uno schermo. L'instradamento con un autorouter esterno passa per file (`phoenix autoroute`): la scheda viene scritta come file Specctra DSN, una sessione Specctra viene letta come un piano le cui piste sono controllate con le regole e applicate tutte o nessuna, e un adattatore che avvia un programma è spento per impostazione predefinita e non avvia nulla senza un interruttore in un file di impostazioni e un'opzione sulla riga di comando; è un prototipo provato solo con sessioni fatte a mano, mai con un vero autorouter. Un interprete di script (`eh3d-phoenix-script`, comandi `phoenix script run`, `phoenix script report` e `phoenix cam-import`) legge file di script del linguaggio di comandi del flusso di lavoro precedente, esegue `WIRE`, `VIA`, `MOVE`, `CHANGE`, `DELETE`, `DRC` e gli altri comandi che hanno un equivalente come operazioni reversibili della scheda, tutto o niente e con un annulla che restituisce l'originale byte per byte, elenca ogni comando di ogni script come equivalente, adattato o in sospeso con la sua causa e legge un job di uscita come profilo di esportazione; è stato costruito dalla descrizione pubblica del linguaggio, è un prototipo provato solo su script scritti per i suoi test e non è mai stato confrontato con lo strumento originale né eseguito sugli script propri dell'autore. Un secondo linguaggio di script (`phoenix-script/2`: variabili, cicli limitati, interrogazioni e modifiche di una scheda, di uno schema o di una libreria come un unico passo reversibile, in un ambiente isolato con limiti; `phoenix script check`, `list` e `help`) è un prototipo provato solo su documenti fatti a mano. Un generatore di strutture RF (`eh3d-phoenix-rfgen`, comando `phoenix rfgen`) disegna, a partire dallo stackup e dalla frequenza, linee a microstriscia, curve smussate e ad arco, transizioni di larghezza, stub, reti di adattamento, filtri passa-basso a salti di impedenza e passa-banda a linee accoppiate, un divisore di Wilkinson, un accoppiatore a rami, antenne patch e dipolo e una transizione SMA con recinzione di via, con formule pubblicate, e le inserisce nella scheda come operazioni reversibili, tutto o niente, con un rapporto delle dimensioni, delle formule usate e del controllo delle regole sul risultato; è un prototipo, provato solo con valori dei libri di testo e con schede create dai test, e non è mai stato confrontato con un simulatore elettromagnetico né con una misura. Un modulo di scrittura KiCad (`phoenix kicad-export`) scrive una scheda, uno schema, impronte e simboli come file KiCad 8, con un rapporto su ciò che il modello non può esprimere; è un prototipo i cui file non sono mai stati aperti in KiCad. Le coppie differenziali (`phoenix pairs` e le operazioni `route_pair`, `tune_pair` e `fix_pair_gap` di `phoenix boardedit`) tracciano le due piste di una coppia una accanto all'altra, uguagliano le loro lunghezze con meandri, correggono la loro distanza e verificano distanza, simmetria, tratto non accoppiato, lunghezze, via, larghezza e, con uno stackup, l'impedenza; sono un prototipo provato solo su schede fatte a mano e dai test, e lo strumento della finestra per esse non è mai stato visto su uno schermo. La spinta (l'operazione `route_shove` di `phoenix boardedit`) sposta in parallelo le piste di altre reti che una nuova pista stringerebbe, entro una profondità e un numero di piste fissati in un file, tutto o niente e rifiutata se lasciasse un errore di regole; con essa vengono gli archi nelle piste (`route_arc`, `round_corners`) e i meandri sui tratti a 45 gradi; sono un prototipo provato solo su schede fatte a mano e dai test, e lo strumento di spinta della finestra non è mai stato visto su uno schermo. La stampa (`phoenix print`, `eh3d-phoenix-print`) scrive i fogli di uno schema, il piano di montaggio e gli strati di una scheda e un pannello come SVG o come PDF scritto a mano; è un prototipo il cui PDF è stato letto solo dal verificatore del proprio crate e mai aperto in un vero lettore di PDF. Le stime di integrità del segnale (`phoenix si`, `eh3d-phoenix-si`) danno, da formule chiuse e dallo stackup, la diafonia tra piste parallele, le riflessioni e le terminazioni delle reti, un diagramma ad occhio semplificato e il percorso di ritorno delle piste; sono prototipi quasi statici provati solo su schede fatte dai test, senza nulla confrontato con un risolutore di campo, un simulatore o una misura.

---

> **Stato di sicurezza funzionale e informatica:** questo repository è ai primi passi e non è un sistema di sicurezza funzionale. Nulla di quanto scritto qui sostituisce i limiti, gli interblocchi e l'arresto di emergenza della macchina e del suo controllore.

## 📋 In breve

| | |
|---|---|
| Versione | `0.0.4` |
| Maturità | Impalcatura |
| Test | 3598 |
| Stack tecnologico | Rust, Python 3.10+ |
| Su hardware reale | Non ancora eseguito su hardware |

**Vai a:** [Panoramica](#-panoramica) · [Architettura](#-architettura) · [Avvio rapido](#-avvio-rapido) · [Test](#-test) · [Struttura del repository](#-struttura-del-repository) · [Progetti correlati](#-progetti-correlati)

## 🚦 Stato del progetto

Ogni riga è una parte di questo repository e il semaforo dice a che punto è oggi. È scritta in base a ciò che il repository può mostrare (test, compilazioni, esecuzioni), non a ciò che è previsto.

🟢 Fatto e verificato · 🟡 Funziona, con limiti noti · 🟠 Fatto a metà o non ancora verificato · 🔴 Non fatto o non funzionante

| Area | Stato | Cosa significa qui |
|---|---|---|
| Controlli del repository | 🟡 | Il controllo del repository, quello delle immagini e i 33 test della base girano nella CI e passano. |
| Test | 🟡 | I 3598 test del codice Rust e i 33 della base passano su un computer. Usano solo cartelle create dai test stessi; nessuno è stato eseguito su veri file di progetto. |
| Hardware reale | 🟠 | Non eseguito su nessuna macchina né scheda: nulla qui è stato provato su hardware. |
| Funzione pianificata | 🟠 | Scritto: l'inventario oscurato di un corpus, le fondamenta del modello (unità, id, JSON canonico), il file di progetto `.phoenixproj`, il modello di scheda con annulla e ripeti, la sua connettività e un primo controllo delle regole, e un lettore di riepilogo di EAGLE XML. Non iniziato: l'importazione di formati diversi da KiCad nel modello, gli scrittori di altri formati, gran parte delle regole, gli editor e il visualizzatore definitivo, il confronto del simulatore con SPICE e tutto il resto del pianificato. Il modello della scheda contiene anche testo e tratti di serigrafia (con un carattere a tratti incorporato) che il controllo delle regole e l'esportazione Gerber usano, e lo schema può essere disegnato su fogli con parti posizionate e fili che vengono confrontati con l'elenco delle reti e con la scheda; una scheda può essere scritta come file XML di EAGLE e riletta, cosa verificata solo con file scritti da questo codice e mai aperta in EAGLE; il contratto per le coppie di file binari EAGLE e del loro XML è scritto, ma non esiste alcun lettore dei file binari. Un calcolatore di stratificazione e linee di trasmissione (`phoenix stackup`: impedenza, permittività effettiva, perdite, ritardo e larghezza della pista di microstriscia, stripline, guida coplanare e coppie accoppiate con formule pubblicate) è un prototipo, mai confrontato con un simulatore di campo né con una misura. Con la base dati dei materiali di EH3D-COMMON (`phoenix materials`, `phoenix stackup --material ID`) Dk e Df di uno strato derivano dai punti pubblicati da una scheda tecnica, mai estrapolati; nessuno di questi valori è stato confrontato con la misura di una scheda. Un set di strumenti RF (`phoenix rf`: file Touchstone, parametri a due porte, adattamento, filtri a scala ideali e, come stime, reti pi e T, valori normalizzati, maschere e cerchi di stabilità) è un prototipo, verificato solo con valori da manuale e mai confrontato con una misura. Un lettore KiCad (`phoenix kicad-read`: simboli, impronte, schemi con fogli, schede e file di progetto) è un prototipo, provato solo su file scritti a mano. Un sanificatore di librerie (`phoenix sanitize`: inventario di una cartella di librerie EAGLE, hash geometrico, confronto approssimato con soglia, gruppi di duplicati, rapporto HTML e JSON e un piano della libreria principale che non viene mai applicato) è un prototipo, provato solo su librerie fatte dai suoi test. Un controllo elettrico (`phoenix erc`) e la sincronizzazione di uno schema e di una scheda (`phoenix sync`: trasferimento alla scheda e annotazione all'indietro, come un pacchetto che si può annullare) sono prototipi, provati solo su schemi e schede fatti a mano. Un simulatore di circuiti proprio (`phoenix sim`: netlist in stile SPICE, la netlist di uno schema e analisi in continua, in alternata e transitoria di circuiti lineari e con diodi) è un prototipo, verificato solo con valori da manuale e mai confrontato con SPICE. La logica di modifica e instradamento (`phoenix route`, `phoenix ratsnest`: i collegamenti rimasti, un router di tratti a 0, 45 e 90 gradi e modifiche reversibili con il controllo delle regole in tempo reale) è un prototipo, provato solo su schede fatte a mano e dai test. Un interprete di script (`phoenix script run`, `phoenix script report`, `phoenix cam-import`: script del linguaggio di comandi del flusso di lavoro precedente eseguiti come operazioni reversibili della scheda, tutto o niente, un rapporto di compatibilità per comando e per script, e job di uscita letti come profilo di esportazione) è un prototipo, costruito dalla descrizione pubblica del linguaggio e provato solo su script scritti per i suoi test. Un secondo linguaggio di script (`phoenix-script/2`: variabili, cicli limitati, interrogazioni e modifiche di una scheda, di uno schema o di una libreria come un unico passo reversibile, in un ambiente isolato con limiti; `phoenix script check`, `list` e `help`) è un prototipo provato solo su documenti fatti a mano. Un generatore di strutture RF (`phoenix rfgen`: linee a microstriscia, curve, stub, reti di adattamento, filtri, un divisore di Wilkinson, un accoppiatore a rami, antenne patch e dipolo e una transizione SMA con recinzione di via, disegnati dallo stackup e dalla frequenza come operazioni reversibili della scheda con un rapporto delle formule e del controllo delle regole) è un prototipo, verificato solo con valori dei libri di testo e mai confrontato con un simulatore elettromagnetico né con una misura. Una distinta base con costi (`phoenix bom`: componenti raggruppati per valore, impronta e produttore, prezzi per scaglione di quantità da un file locale, il costo per scheda e per lotto, e una stima del costo delle schede con tariffe di esempio) e un assemblaggio 3D della scheda con i modelli dei suoi componenti (`phoenix assembly3d`: un modello per impronta da una tabella, il parallelepipedo che racchiude ogni solido STEP, collisioni e altezza verificate sui parallelepipedi, e un'esportazione OBJ e STEP semplificata) sono prototipi, provati solo su schede e file creati dai test; le tariffe sono un esempio e non un preventivo, nessun programma CAD ha aperto i file esportati, e la finestra 3D che disegna la scena è un prototipo mai visto su uno schermo. Un insieme di casi di riferimento (`phoenix oracle`) verifica i calcoli con valori pubblicati, la loro fonte e la loro tolleranza; non è un confronto con EAGLE, KiCad, un risolutore di campo né una misura, e nessuno stato qui sopra cambia per questo. Un corpus proprio (`phoenix corpus`) di piccole schede, schemi, librerie e file di produzione scritti a mano, con numeri calcolati a mano, file che devono essere rifiutati, andate e ritorni tra formati e proprietà su schede generate, è un prototipo; non contiene file di un produttore o di un utente e non è un confronto con EAGLE, KiCad o un programma CAM. |
| Finestra | 🟠 | Il visualizzatore ed editor di schede e schemi (`phoenix view`, egui, dietro la funzionalità `gui`) è scritto e la sua logica è collaudata: una scheda si modifica con annulla e ripeti e con i rilievi delle regole di ogni modifica, uno schema viene controllato, sincronizzato con una scheda e modificato (posiziona, sposta, ruota, specchia, elimina, copia e incolla, fili, etichette, porte di alimentazione e nomi di bus, con il controllo delle regole dopo ogni modifica), e nulla viene mai salvato sopra il file letto; ma la finestra non è mai stata vista su uno schermo: il disegno, il mouse e la fluidità a schermo non sono verificati. La stessa finestra ha una modalità librerie che modifica i simboli, i footprint e i dispositivi delle librerie EAGLE (`phoenix libedit`, `phoenix footprint`, `docs/LIBRARY_EDIT.md`) con annullamento, un controllo della libreria e generatori di footprint; anch'essa non è mai stata vista su uno schermo. Una modalità di modifica delle schede (`phoenix boardedit`, `docs/BOARD_EDIT.md`) posiziona componenti, traccia piste a 45 gradi e via, incolla reti e regola le lunghezze con meandri, e ogni operazione è verificata dalle regole di progetto; non è mai stata vista su uno schermo né usata con il mouse. Una finestra per i file Gerber ed Excellon di una cartella (`docs/VIEWER.md`: uno strato per file, opacità, misura e confronto con la scheda) e una finestra 3D della scheda assemblata (telecamera orbitale, un clic sceglie un componente, collisioni e altezze segnalate) sono prototipi; nessuna è mai stata vista su uno schermo né usata con il mouse. Il programma `phoenix-gui` la apre con una schermata iniziale (aprire un file, una nuova libreria, file recenti, trascinare un file) e, su Windows, senza finestra di console. Anche le parti comuni della finestra (ricerca, misure, preset dei livelli, elenco della cronologia, temi, scala e tasti modificabili, aiuto e preferenze ricordate) sono prototipi e non sono mai state viste su uno schermo. |
| Requisiti | 🟠 | 21 requisiti su 31 (la modalità oscurata, le fondamenta del modello, le operazioni reversibili, il file di progetto, i codici di diagnostica, i calcolatori RF analitici, i controlli di connettività e di regole e l'esportazione di fabbricazione con il suo pacchetto, il visualizzatore ed editor di schede e schemi i lettori e gli scrittori KiCad, il sanificatore di librerie, i controlli e la sincronizzazione dello schema il simulatore di circuiti, la logica di instradamento e l'interprete di script) sono a livello di prototipo, 10 sono solo pianificati e nessuno è convalidato. |

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

**Scaricare e aprire la finestra.** L'archivio di una versione (Windows a 64 e 32 bit, Linux e macOS) contiene due programmi: `phoenix`, la riga di comando, e `phoenix-gui`, la finestra del visualizzatore ed editor. Fai doppio clic su `phoenix-gui` (`phoenix-gui.exe` su Windows): apre una schermata iniziale in cui apri una scheda, uno schema o una libreria, scegli uno dei file recenti o trascini un file sulla finestra; `phoenix-gui <file>` apre subito un file. Su Windows non ha una finestra di console: se non si apre, il motivo è scritto in `last_error.txt`, nella cartella delle impostazioni dell'utente (`%APPDATA%\EH3D-PHOENIX-PCB` su Windows, `~/.config/eh3d-phoenix-pcb` sugli altri sistemi), dove si trova anche l'elenco dei file recenti (`recent.cfg`); `config/viewer.cfg` stabilisce quanti ne conserva. `phoenix` da solo stampa solo testo e non apre mai una finestra. La finestra è un prototipo che non è stato provato con il mouse (vedi lo stato qui sopra). Per compilare i due programmi: `cargo build --release -p eh3d-phoenix-cli --features gui`.

## 🧪 Test

Il codice ha 3598 test (`cargo test`) e la base ne ha 33 (`python -I -m unittest discover -s tools -p "test_*.py"`); tutti passano su un computer. La CI esegue il controllo del repository, quello delle immagini, i test della base e, per il codice Rust, il controllo del formato, clippy con gli avvisi trattati come errori e i test su Linux e Windows.

## 🏗️ Compilazione e rilascio

La versione sta nel file `VERSION` e in `eh3d.project.json`, e ogni versione ha la sua voce in [CHANGELOG.md](CHANGELOG.md). `cargo build --release` compila la libreria e il programma `phoenix`; le versioni vengono pubblicate su GitHub. Gli archivi di una versione contengono `phoenix` e, accanto, `phoenix-gui`, la finestra; `cargo build --release -p eh3d-phoenix-cli --features gui` compila entrambi.

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
│   ├── SECURITY_REVIEW.md                       la revisione di sicurezza e i limiti in vigore
│   ├── DECISIONS.md STATUS.md                   decisioni applicabili qui e stato onesto
│   ├── NOT_YET.md RULES.md                      idee per dopo e regole per scrivere il codice
│   ├── TESTING.md I18N_AUDIT.md                 come si prova, e il controllo delle otto lingue
│   ├── ROADMAP.md                               la mappa delle fasi, dalle fondamenta alla suite tutto in uno
│   ├── GLOSSARY.md MIGRATION.md                 i termini e la guida per chi arriva da EAGLE o KiCad
│   ├── tutorials/ glossary/ migration/          tutorial eseguiti da un test, e le traduzioni
│   ├── ERROR_CODES.md                           i codici di diagnostica PHOENIX-<AREA>-<NNN>
│   ├── STACKUP.md                               calcoli di stratificazione e linee, formule
│   ├── RF.md                                    strumenti RF: Touchstone, doppi bipoli, adattamento, filtri
│   ├── RF_ADVANCED.md                           più RF: pi/T, valori standard, maschere, stabilità
│   ├── RFGEN.md                                 strutture RF sulla scheda: formule, fonti e limiti
│   ├── KICAD.md                                 i lettori KiCad: cosa si legge da ogni formato
│   ├── KICAD_IMPORT.md                          leggere un intero progetto KiCad: comando, rapporto
│   ├── KICAD_EXPORT.md                          scrivere file KiCad: cosa si scrive, il rapporto, il comando
│   ├── VIEWER.md                                il prototipo di visualizzatore ed editor e il suo benchmark
│   ├── SHORTCUTS.md                             scorciatoie da tastiera della finestra e come cambiarle
│   ├── SANITIZER.md                             il sanificatore di librerie: duplicati, hash, rapporto e piano
│   ├── SCRIPT.md                                l'interprete di script: comandi, rapporto e job di uscita
│   ├── SCRIPT_API.md                            il secondo linguaggio di script: riferimento e limiti
│   ├── SCRIPT_EXAMPLES.md                       i suoi script di esempio con il loro output
│   ├── ERC.md                                   controllo elettrico e sincronizzazione
│   ├── ANNOTATE.md                              annotazione all'indietro nello schema e confronto
│   ├── SCHEMATIC_EDIT.md                        l'editor di schemi: operazioni, reti, finestra
│   ├── HIERARCHY.md BUSES.md                    simboli di foglio, fogli ripetuti e bus
│   ├── LIBRARY_EDIT.md                          l'editor di librerie: modifiche, controlli, generatori
│   ├── IPC_LIBRARY.md                           la libreria propria di footprint: famiglie, densità, controllo
│   ├── LIBINDEX.md                              l'importazione di massa di librerie: rapporto di qualità, indice
│   ├── BOARD_EDIT.md                            l'editor di schede: operazioni, regole, finestra
│   ├── SHOVE.md                                 spinta delle piste e archi nelle piste: operazioni, impostazioni, finestra
│   ├── BOARD_ZONES.md                           zone di rame nell'editor di schede: riempimento, priorità, incolla
│   ├── SIM.md                                   simulazione di circuiti: netlist, analisi, esportazione dello schema
│   ├── SIM_NONLINEAR.md                         simulatore: dispositivi non lineari, scansione in continua, convergenza
│   ├── SIM_ADVANCED.md                          simulatore: scansioni, misure, Monte Carlo, poli
│   ├── NETLIST.md                               la netlist di uno schema: tabella dei modelli, rapporto
│   ├── ROUTE.md                                 modifica e instradamento: connessioni, router, modifiche reversibili
│   ├── DRC_ADVANCED.md                          controlli avanzati delle regole e relative impostazioni
│   ├── AUTOROUTE.md                             instradamento con un autorouter esterno: DSN, sessione, adattatore
│   ├── ZONES.md                                 riempimento delle zone di rame: ritaglio, rilievi, isole, verifica
│   ├── LENGTHS.md                               lunghezze, gruppi da pareggiare e coppie differenziali
│   ├── DIFFPAIRS.md                             coppie differenziali: definizione, operazioni, verifiche, il comando
│   ├── SI.md                                    integrità del segnale: diafonia, terminazioni, occhio, percorso di ritorno
│   ├── EAGLE_DEEP.md                            EAGLE in profondità: regole, lavori CAM, varianti, livelli, rapporto di perdita
│   ├── GERBER_IMPORT.md                         importazione di file Gerber ed Excellon per revisione, e l'andata e ritorno
│   ├── GERBER_X2.md                             attributi Gerber X2 nell'esportazione e la loro verifica
│   ├── PANEL.md                                 panelizzazione: schede, cornice, linguette, V-score, fiduciali, fori, strati
│   ├── PRINT.md                                 stampa: pagine SVG e PDF con cartiglio
│   ├── BOM.md                                   distinta base con costi, CSV, JSON, HTML
│   ├── PARTS.md                                 base locale dei componenti: ricerca, controllo, legame alla distinta
│   ├── PNP.md STENCIL.md FABPACK.md             posizionamento, stencil e il pacchetto di fabbricazione
│   ├── ASSEMBLY3D.md                            assemblaggio 3D: modelli STEP, collisioni, OBJ, STEP
│   ├── DXF.md                                   DXF: contorno, esportazione schede e pannelli
│   ├── ENCLOSURE.md                             scheda contro involucro, solido STEP
│   ├── SPATIAL.md                               indice spaziale dei controlli, rapporto perf
│   ├── ORACLE.md                                casi di riferimento e piano di confronto con EAGLE e KiCad
│   ├── CORPUS.md                                il corpus proprio: pezzi scritti a mano, andate e ritorni, proprietà
│   ├── GATE_F1.md                               cosa verifica il varco della prima fase e cosa no
│   └── adr/                                     i registri delle decisioni
├── AGENTS.md                                    regole per chi modifica il repository
├── deny.toml                                    politica di licenze e avvisi delle dipendenze
├── crates/                                      inventario, modello, progetto, scheda, schema, EAGLE, KiCad, regole, ERC, sincronizzazione, hub, esportazione, importazione Gerber, pannello, stratificazione, RF, simulatore, router, zone, lunghezze, distinta base, assemblaggio 3D, sanificatore, script, visualizzatore, CLI
├── config/                                      il file delle impostazioni (ogni valore regolato)
├── examples/                                    script di esempio, il loro output e i file dei tutorial
├── Cargo.toml Cargo.lock                        il workspace Rust e le sue dipendenze bloccate
├── tools/                                       il controllo del repository, i suoi test e gli strumenti delle immagini
├── tests/                                       casi di riferimento e corpus proprio di pezzi scritti a mano
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
* [Tutorial](docs/tutorials/it/README.md): nove percorsi con comandi che un test esegue, in otto lingue; `phoenix help` spiega ogni tema e comando nel programma
* [Test](docs/TESTING.md): cosa si prova e come, dove stanno i file di esempio e come aggiungere un caso; [le otto lingue](docs/I18N_AUDIT.md): cosa si controlla in ogni testo e in ogni pagina
* [Glossario](docs/glossary/it/GLOSSARY.md) e [guida alla migrazione per chi arriva da EAGLE o KiCad](docs/migration/it/MIGRATION.md)

## 🤝 Contribuire e comunità

* Prima di una pull request esegui `python -I tools/ci_validate.py` e i test della base: tutto deve passare. Vedi [CONTRIBUTING.md](CONTRIBUTING.md) e il [codice di condotta](CODE_OF_CONDUCT.md).
* Il README esiste in otto lingue: una modifica al testo si fa in tutti e otto i file, e le due immagini sono disegnate da `docs/architecture.json` e `docs/structure.json` con `tools/make_architecture.py` e `tools/make_structure.py`.
* Segnala una vulnerabilità in privato: [SECURITY.md](SECURITY.md). Domande, idee e segnalazioni: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)). La revisione di sicurezza che il progetto ha fatto da sé (senza audit esterno): [docs/SECURITY_REVIEW.md](docs/SECURITY_REVIEW.md).

## 👤 AUTORE

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENZA

Software: GPL-3.0-or-later. Vedi [LICENSE](LICENSE). I progetti hardware della suite usano la CERN OHL-S.
