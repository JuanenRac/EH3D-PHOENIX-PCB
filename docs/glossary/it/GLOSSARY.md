# Glossario

I termini del dominio come li usa questo programma. La tabella in fondo alla pagina in inglese dà ogni termine nelle otto lingue.

| Termine | Significato |
|---|---|
| **nm (nanometro)** | L'unità di ogni lunghezza nel modello, conservata come intero: 1 mm sono 1 000 000 nm, quindi somme e confronti non hanno errori di arrotondamento. |
| **rete** | Tutti i pad, le piste e le via che devono essere collegati elettricamente; ha un nome come GND. |
| **classe di rete** | Un gruppo di reti che condividono regole: larghezza della pista, distanza e dimensione della via. Il file di regole del secondo formato contiene le classi. |
| **via** | Un foro metallizzato che unisce il rame di due livelli. |
| **anello anulare** | L'anello di rame che resta intorno a un foro; se è troppo sottile, la punta può romperlo. |
| **impronta** | I pad di rame, i fori e i disegni di un componente sulla scheda. |
| **area di ingombro (courtyard)** | L'area intorno a un'impronta in cui nessun altro componente può entrare. |
| **zona di rame** | Un'area di rame piena, di solito un piano di massa, che mantiene la distanza dalle altre reti. |
| **stack-up (struttura a strati)** | L'ordine, lo spessore e il materiale dei livelli di rame e isolante di una scheda; da esso dipende l'impedenza di una linea. |
| **DRC (controllo delle regole di progetto)** | Il controllo di una scheda rispetto a limiti come la pista più stretta, la distanza, il foro e l'anello anulare. Comando: `phoenix check`. |
| **ERC (controllo delle regole elettriche)** | Il controllo di uno schema: pin su nessuna rete, ingressi che nessuno pilota, alimentazioni senza sorgente, ecc. Comando: `phoenix erc`. |
| **linee di collegamento (ratsnest)** | Le linee rette che mostrano quali collegamenti sono ancora da instradare. Comando: `phoenix ratsnest`. |
| **Gerber X2** | Il formato dei file di rame, solder mask, pasta e serigrafia che legge il produttore; X2 aggiunge attributi come la funzione del livello. |
| **Excellon** | Il formato dei file di foratura: gli utensili e le posizioni dei fori, metallizzati o no. |
| **DSN / SES** | I file Specctra per un autorouter esterno: DSN è il progetto che esce, SES la sessione con i percorsi che ritorna. Comando: `phoenix autoroute`. |
| **taglio a V (V-score)** | Una scanalatura in linea retta tra due schede di un pannello, per poterle separare spezzandole. |
| **pannello** | Più schede disposte insieme su un foglio, con cornice, ponticelli, fiducial e fori di riferimento, per produrle e assemblarle in una volta. Comando: `phoenix panel`. |
| **ponticelli forati (mouse bites)** | Un ponticello tra due schede attraversato da una fila di piccoli fori, perché si rompa in modo netto. |
| **fiducial** | Un piccolo segno rotondo di rame nudo che la macchina di montaggio vede per trovare la posizione della scheda. |
| **posizionamento componenti (pick and place)** | Il file con riferimento, posizione, rotazione e lato di ogni componente, per la macchina di montaggio. Comando: `phoenix pnp`. |
| **stencil per pasta saldante** | La lamina sottile con aperture attraverso cui si stende la pasta saldante sui pad. Comando: `phoenix stencil`. |
| **BOM (distinta base)** | L'elenco dei componenti di una scheda, raggruppati per valore e impronta, con quantità e prezzi. Comando: `phoenix bom`. |
| **coppia differenziale** | Due piste che portano un segnale come differenza delle loro tensioni; devono mantenere una distanza, lunghezze uguali e simmetria. Comando: `phoenix pairs`. |
| **diafonia (NEXT, FEXT)** | La parte di un segnale che compare su una pista vicina; vicina (NEXT) lato pilota, lontana (FEXT) all'altro capo. Comando: `phoenix si crosstalk`. |
| **parametri S / Touchstone** | La riflessione e la trasmissione di una rete a ogni frequenza; Touchstone (.s1p, .s2p) è il file che le memorizza. Comando: `phoenix rf touchstone`. |
| **ROS (rapporto d'onda stazionaria)** | Un numero da 1 in su che dice quanto male è adattato un carico a una linea: 1 è un adattamento perfetto. |
| **Monte Carlo** | Molte esecuzioni di un circuito in cui i valori variano a caso entro le loro tolleranze; un seme fisso dà sempre la stessa tabella. |
| **netlist** | Il testo che elenca gli elementi di un circuito e i nodi che collegano; è ciò che legge il simulatore. Comando: `phoenix netlist`. |
| **prova a secco** | Un'esecuzione che fa tutto tranne scrivere: mostra cosa verrebbe scritto o cambiato. Quasi ogni comando è una prova a secco salvo che `--out` indichi un file nuovo. |

* [English](../../GLOSSARY.md)
