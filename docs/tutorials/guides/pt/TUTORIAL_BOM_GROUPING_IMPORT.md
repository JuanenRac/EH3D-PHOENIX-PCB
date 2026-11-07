# Agrupar e exportar a lista de materiais, e importar a lista de outro programa

A placa `examples/guides/bom_four.brd` tem três resistências de 10 k e uma de 4k7. Escolhe-se no ficheiro de definições como se agrupam as peças e que colunas tem o ficheiro CSV, e depois lê-se uma lista escrita por outro programa com um mapa das suas colunas. Não se descarrega nada de uma rede e a tarifa das definições é apenas um exemplo da forma de um orçamento.

Execute os comandos a partir da raiz do repositório; veja [como executar os guias](README.md).

1. Liste a lista com o agrupamento por omissão (valor, pegada, fabricante e número de peça) e os dados de fabricante de `bom_parts.csv`. R1 e R2 partilham fabricante e número, R3 tem outro fabricante, por isso há três linhas. Não há ficheiro de preços, pelo que cada linha é assinalada com `PHOENIX-BOM-005`.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

2. A mesma placa com `group_by = value,footprint` em `bom_costs_joined.cfg`: o fabricante e o número de peça saem do agrupamento, e R1, R2 e R3 passam a ser uma linha de quantidade 3 que não tem nenhum dos dois.

```console
$ phoenix bom --config examples/guides/bom_costs_joined.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

3. Escreva o ficheiro CSV. As suas colunas são as `csv_columns` das definições, por essa ordem (aqui `references,quantity,value,footprint,manufacturer,mpn,status`); a página HTML e o ficheiro JSON mantêm todas as colunas. Um ficheiro que exista nunca é substituído.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv --csv build/bom.csv examples/guides/bom_four.brd
```

4. Importe a lista de outro programa. `bom_other_program.csv` usa ponto e vírgula e nomes de coluna próprios; `bom_import_map.cfg` diz como a ler. O relatório compara a lista com a placa: R5 está na lista e não na placa (`PHOENIX-BOM-021`), e R4 está marcada como não montada, pelo que fica fora da lista e é mostrada como informação (`PHOENIX-BOM-009`). `--parts-out` guarda o que a lista diz sobre fabricantes e números como ficheiro de dados de peças.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --import examples/guides/bom_other_program.csv --import-map examples/guides/bom_import_map.cfg --parts-out build/parts_from_bill.csv examples/guides/bom_four.brd
```

5. Use o ficheiro de dados de peças guardado nas execuções seguintes em vez da lista original.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts build/parts_from_bill.csv examples/guides/bom_four.brd
```

## Como ler o resultado

* O ficheiro de definições por omissão é `config/costs.cfg` e o mapa por omissão é `config/bom_import.cfg`, se existirem. Um mapa indica o delimitador (vírgula, ponto e vírgula ou tabulação) e, para cada coluna, os nomes que ela pode ter no ficheiro; só as referências são obrigatórias. Os nomes fornecidos são uma suposição e não são o formato de nenhum programa: copie o ficheiro e escreva os nomes do seu próprio ficheiro.
* Os códigos `PHOENIX-BOM-019`, `PHOENIX-BOM-020` e `PHOENIX-BOM-021` são avisos e não alteram nada: uma quantidade que não é o número de referências da sua linha, uma referência em duas linhas (usa-se a primeira) e uma peça que só um lado tem ou cujo valor difere. Um intervalo como `R1-R5` é expandido nas suas peças individuais; um intervalo mal formado, invertido ou mais longo do que `max_range` interrompe a importação (`PHOENIX-BOM-022` a `PHOENIX-BOM-024`). `--import-format` também lê uma lista de peças do EAGLE ou uma lista de materiais do KiCad (`docs/BOM.md`). O que `--parts` diz prevalece sobre a lista importada.
* Sem `--csv`, `--json-file` e `--html` o comando não escreve nada. O código de saída é 0 se terminar bem, 1 se um componente indicar uma pegada que não está na placa e 2 se os argumentos ou a entrada forem inutilizáveis.

## Ver também

* [docs/BOM.md](../../../../docs/BOM.md)
* [docs/PARTS.md](../../../../docs/PARTS.md)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
