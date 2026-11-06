# Da placa aos ficheiros de fabrico

O projeto KiCad de `examples/tutorials/kicad/` tem duas resistências numa placa de 30 mm por 20 mm ligadas pela rede `SIG`. Confrontam-se esquema e placa, correm-se as regras de projeto e escrevem-se os ficheiros de fabrico com um manifesto de somas.

Execute os comandos a partir da raiz do repositório; veja [como executar os tutoriais](README.md).

1. Execute a verificação das regras elétricas do esquema. Cada resultado tem um código e os confirmados ficam separados dos inconclusivos.

```console
$ phoenix erc examples/tutorials/kicad/demo.kicad_sch
```

2. Compare o esquema com a placa pela identidade das peças. `sync` é um simulacro: lista o que a transferência mudaria e não escreve nada a menos que acrescente `--apply --out FICHEIRO`.

```console
$ phoenix sync examples/tutorials/kicad/demo.kicad_sch examples/tutorials/kicad/demo.kicad_pcb
```

3. Corra as regras de projeto com os limites de um ficheiro de regras. A pista tem 0,25 mm e o ficheiro pede pelo menos 0,2 mm, por isso não há erros e o código de saída é 0.

```console
# test
$ phoenix check --rules examples/tutorials/rules_relaxed.json examples/tutorials/kicad/demo.kicad_pcb
Confirmed: 0
Inconclusive: 0
```

4. Escreva os ficheiros de fabrico numa pasta nova: um Gerber X2 por camada, os ficheiros de furação, o de posição, a lista de materiais e o `manifest.json` com o SHA-256 de cada ficheiro. Antes de escrever, os ficheiros são lidos de volta e comparados com a placa.

```console
# test
$ phoenix export --out build/fab examples/tutorials/kicad/demo.kicad_pcb
~ files written: build/fab
```

5. Crie o pacote completo, com o estêncil e o posicionamento de cada face, num zip determinista: a mesma placa dá os mesmos bytes. Sem `--out` é um simulacro que lista os ficheiros.

```console
$ phoenix fabpack --out build/demo_pack.zip examples/tutorials/kicad/demo.kicad_pcb
```

## Ver também

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
