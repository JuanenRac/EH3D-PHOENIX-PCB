# Limitar o trabalho das verificações com um ficheiro de limites

Algumas verificações levam um tempo proporcional ao quadrado de uma contagem, por exemplo o teste de que as arestas de um polígono não se cruzam. Um limite restringe esse tempo: um polígono acima do limite não é testado e o relatório di-lo como informação. Executa-se a verificação de uma placa com os valores por omissão e depois com um ficheiro de limites muito baixo, e vê-se a nota.

Execute os comandos a partir da raiz do repositório; veja [como executar os guias](README.md).

1. Execute o ficheiro de regras na placa KiCad com os limites por omissão. A pista da placa é mais fina do que o ficheiro pede, por isso há uma constatação confirmada e o código de saída é 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ Confirmed: 1
~ PHOENIX-CHECK-001
```

2. Execute-o de novo com `--limits examples/guides/limits_small.cfg`, que contém `max_polygon_points = 3`. O contorno da placa tem quatro pontos, por isso é listado como informação com o código `PHOENIX-BOARD-017`, e o código de saída continua a ser 1 porque a informação nunca o altera.

```console
# test
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ PHOENIX-BOARD-017
~ max_polygon_points = 3
~ PHOENIX-CHECK-001
```

3. A mesma nota noutro idioma e como JSON para um script. O código fica em inglês; o título segue `--lang`.

```console
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --lang pt examples/tutorials/kicad/demo.kicad_pcb
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --json examples/tutorials/kicad/demo.kicad_pcb
```

## Como ler o resultado

* `--limits FICHEIRO` é aceite por todos os comandos, antes ou depois do seu nome. O ficheiro tem linhas `chave = valor` de `config/board.cfg` e de `config/eagle_board.cfg` por qualquer ordem; uma linha desconhecida, ou um valor fora do seu intervalo, é recusada com o número da linha e o código de saída 2.
* Um limite só altera o que limita. Com os valores por omissão nenhuma saída muda, e num polígono acima de `max_polygon_points` continuam a verificar-se os pontos repetidos.

## Ver também

* [config/board.cfg](../../../../config/board.cfg)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
* [docs/RULES.md](../../../../docs/RULES.md)
