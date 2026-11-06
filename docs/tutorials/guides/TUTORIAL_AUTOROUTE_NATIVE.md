# Route a board with the built-in autorouter, with a pair and with the vias already on the board

`phoenix autoroute native` routes the open connections of a board with a search of its own and needs no outside program. `examples/guides/open.brd` has one net whose two pads are not joined; `examples/guides/open_pair.brd` has the two nets `D_P` and `D_N` of a pair, also not joined. The plan only adds tracks and vias, and the rules are checked before and after it. The rule file is given because the EAGLE boards of the examples have no design rules of their own.

Run the commands from the root of the repository; see [how to run the guides](README.md).

1. Plan the routes. It is a dry run: the report lists the connections routed, the tracks, vias and length added and the connections still open, and says on the error stream that nothing was written.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json examples/guides/open.brd
```

2. Write the board with the routes to a new file. The file is the JSON of the design model, whatever its name, and an existing file is never replaced.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --out build/open_routed.json examples/guides/open.brd
```

3. Route the written board again: it has no open connection left, so there is nothing to add.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json build/open_routed.json
```

4. Ask for the pair. `pair_dp_dn.json` says that `D_P` and `D_N` are a pair with tracks of 0.2 mm and 0.3 mm between them. When there is room the two tracks are laid side by side on one layer and the report says `coupled pair: p`; otherwise both nets are routed as ordinary nets and a note with the code `PHOENIX-AROUTE-017` gives the reason.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json examples/guides/open_pair.brd
```

5. Route only the two nets of the pair and refuse a plan that leaves a connection open: with `--require-complete` the command ends with exit code 2 and the code `PHOENIX-AROUTE-016` if any connection is left.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json --nets D_P,D_N --require-complete examples/guides/open_pair.brd
```

## How to read the result

* A via of the net that is already on the board, on a node of the grid, is a free change of layer: the route changes layer there and puts no new via. With `via_snap_nm` above 0 in `config/autoroute_native.cfg` a via that is at most that far from a node is used too, with a note `PHOENIX-AROUTE-019`. This is the reuse of vias; the plan never changes a track, a via or a pad that was already there.
* A pair is coupled only when each net has exactly one open connection and the four pads share a layer. The two tracks have the same length and keep the gap of the pair. Without `--pairs` or `--pair-config` the nets of a pair are routed as two ordinary nets and the pair check of the board editor judges the result afterwards.
* A prototype tried on small boards made by hand and by the tests. The search, its costs and its limits (which count nodes and never time) are in `config/autoroute_native.cfg`. If the routes of a net together add a rule error, that net is left out and reported with `PHOENIX-AROUTE-015`: the plan never adds an error that the board did not have.

## See also

* [docs/AUTOROUTE_NATIVE.md](../../../docs/AUTOROUTE_NATIVE.md)
* [docs/DIFFPAIRS.md](../../../docs/DIFFPAIRS.md)
* [docs/AUTOROUTE.md](../../../docs/AUTOROUTE.md)
