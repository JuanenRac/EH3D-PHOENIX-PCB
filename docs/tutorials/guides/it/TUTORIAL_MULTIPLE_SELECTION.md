# Selezionare più oggetti nell'editor di schede e spostarli, allinearli o eliminarli come un solo passo

La selezione dell'editor di schede è un insieme ordinato di componenti, piste, via, zone ed elementi di serigrafia. Senza schermo la si guida con `phoenix boardedit` e un file di operazioni: `select` cambia la selezione, e `move_selection`, `delete_selection` e `align_selection` agiscono su di essa come un solo passo della cronologia con inversa esatta. `examples/guides/select_ops.json` seleziona con un rettangolo ciò che sta interamente dentro, qui la resistenza R1 e non la pista, e la sposta di 2 mm verso l'alto.

Eseguite i comandi dalla radice del repository; vedere [come eseguire le guide](README.md).

1. Applicate le operazioni come prova a vuoto. Il rapporto elenca le tre operazioni e gli errori di regole prima e dopo; non si scrive nulla. Si dà il file di regole perché la scheda EAGLE non ha regole di progetto proprie.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json examples/tutorials/tut.brd
```

2. Scrivete la scheda modificata in un file nuovo (`.json` per il modello di progetto, `.brd` per EAGLE). L'ingresso non viene mai sovrascritto. Il codice di uscita è 0 se le regole non provano alcun errore sul risultato e 1 se ne provano uno.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --out build/moved.json examples/tutorials/tut.brd
```

3. Stampate il rapporto come JSON, che uno script può leggere: la chiave `rule_errors` è il numero di errori che le regole provano sul risultato.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --json examples/tutorials/tut.brd
```

## Come leggere il risultato

* I passi di `select` sono `add`, `remove` e `toggle` (gli oggetti di `items`, scritti `{"kind": "component", "id": "..."}`), `net` (tutti gli oggetti di una rete, tramite il suo id), `rect` (un rettangolo tra `from` e `to`, in nanometri come `x_nm` e `y_nm`; `mode` è `contained` o `crossed`, che prende anche ciò che il rettangolo tocca), `all`, `invert` e `clear`. Si applicano in ordine e tutto o niente, e la selezione non fa parte della cronologia.
* `move_selection` sposta di `dx_nm` e `dy_nm` (l'alto è positivo), arrotondato alla griglia a meno che `snap` sia false; le piste non selezionate non seguono un componente, quindi selezionate prima la rete per portare il rame. `delete_selection` elimina la selezione. `align_selection` accetta `left`, `right`, `top`, `bottom`, `center_x`, `center_y` (almeno due oggetti) e `distribute_x`, `distribute_y` (almeno tre). Un componente bloccato rifiuta l'intera operazione (`PHOENIX-BEDIT-006`), una selezione vuota è `PHOENIX-BEDIT-009`.
* Nella finestra, con lo strumento Seleziona: Maiusc o Ctrl+clic alterna un oggetto; Maiusc o Ctrl+trascinamento in un punto vuoto aggiunge ciò che sta interamente nel rettangolo (anche zone ed elementi di serigrafia); Alt+trascinamento aggiunge anche ciò che il rettangolo tocca; `N` seleziona la rete sotto il puntatore; Ctrl+A seleziona tutto; Esc scarta la selezione; le frecce la spostano di un passo di griglia (Maiusc: dieci passi); Canc la elimina; Alt+Sinistra, Destra, Su, Giù, Alt+C e Alt+M la allineano; Alt+H e Alt+V la distribuiscono a intervalli uguali (almeno tre oggetti); Ctrl+I inverte la selezione; un clic su un elemento di serigrafia o sul bordo di una zona lo prende; trascinare un oggetto selezionato sposta tutta la selezione sulla griglia; la barra di stato dà il numero di oggetti con il loro riquadro e le loro dimensioni. Ognuna è un passo della cronologia e Ctrl+Z restituisce gli stessi byte. La finestra non è mai stata vista su uno schermo né usata con un mouse.

## Vedere anche

* [docs/BOARD_EDIT.md](../../../../docs/BOARD_EDIT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
* [docs/VIEWER.md](../../../../docs/VIEWER.md)
