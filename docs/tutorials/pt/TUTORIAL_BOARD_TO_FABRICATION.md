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

6. Veja a placa em números antes de a enviar: o tamanho, as contagens, o cobre de cada camada, as furações e a densidade. Não se escreve nada.

```console
# test
$ phoenix stats examples/tutorials/kicad/demo.kicad_pcb
Board statistics:
Board: 30.000000 x 20.000000 mm; 600.000000 mm2; Cutouts: 0.000000 mm2
Components: 2 (top: 2; bottom: 0)
```

7. Marque uma pastilha como ponto de teste com uma operação do editor de placas (`examples/tutorials/testpoint_ops.json` marca a segunda pastilha de R1, na rede `MID`) e obtenha o relatório dos pontos de teste e das redes que nenhuma sonda alcança. O ficheiro de entrada nunca é substituído.

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/testpoint_ops.json --out build/with_test_point.json examples/tutorials/tut.brd
~ written: build/with_test_point.json
$ phoenix testpoints build/with_test_point.json
Test points of the board:
Test points: 1 (probe side: both)
Nets asked: 1; covered: 1; coverage: 100.00 %
```

8. Adicione duas cotas a uma placa que tem uma camada de documentação (`examples/tutorials/dimension_ops.json`: a largura e a altura da placa, com a linha de cota 3 mm para fora) e escreva-as num desenho DXF; na janela faz-se o mesmo com a ferramenta de cotas, e *Editar cota* move os seus pontos. O desenho é um simulacro até que `--out` indique um ficheiro novo.

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/dimension_ops.json --out build/with_dimensions.json examples/tutorials/notes_board.json
~ written: build/with_dimensions.json
$ phoenix dxf-export board --dimensions build/with_dimensions.json
entities: 21
layer DIMENSIONS: 16
```

9. Pergunte ao assistente de fornecedores onde comprar a lista de materiais. Lê um ficheiro de preços (`examples/tutorials/prices.csv`, uma linha por fornecedor e escalão de quantidade) e os fornecedores, os portes e a encomenda mínima de um ficheiro de definições (`examples/tutorials/sourcing.cfg`), e orça a lista em cada fornecedor e como compra repartida. Os preços são os do ficheiro: não se consulta nenhuma rede.

```console
# test
$ phoenix bom --sourcing --config examples/tutorials/sourcing.cfg --prices examples/tutorials/prices.csv examples/tutorials/kicad/demo.kicad_pcb
~ suppliers: 2 considered, 1 lines, EUR
~ cheapest single supplier: Shop A for 2.210000
~ split purchase: 1 orders for 2.210000
```

## Ver também

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
* [docs/STATS.md](../../STATS.md)
* [docs/TESTPOINTS.md](../../TESTPOINTS.md)
* [docs/DIMENSIONS.md](../../DIMENSIONS.md)
* [docs/BOM.md](../../BOM.md)
