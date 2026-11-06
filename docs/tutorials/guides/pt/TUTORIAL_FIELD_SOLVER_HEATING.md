# Resolver a impedância de uma linha com o solucionador de campo e estimar quanto aquece uma pista

`examples/guides/stackup_job.json` descreve um empilhamento de duas camadas com uma linha. Resolve-se a secção com o solucionador de campo 2D e compara-se o resultado com as fórmulas fechadas, depois tira-se a linha de uma placa e por fim estima-se o aumento de temperatura da pista de uma rede com duas correntes. As duas ferramentas são protótipos quase estáticos que não foram comparados com outro solucionador nem com uma medição.

Execute os comandos a partir da raiz do repositório; veja [como executar os guias](README.md).

1. Resolva a linha do trabalho. O relatório dá a malha, as iterações e o resíduo, depois a impedância, a permitividade efetiva, a indutância, a capacidade, o atraso e a perda dielétrica, com a fórmula fechada ao lado e a diferença em percentagem. É um simulacro: não se escreve nada.

```console
$ phoenix field2d examples/guides/stackup_job.json
```

2. Refine a malha por 2 para ver quanto a resposta se move; um resultado que muda pouco convergiu. Uma solução que não atinge a tolerância dá `PHOENIX-FIELD-004` e nenhum número.

```console
$ phoenix field2d examples/guides/stackup_job.json --refine 2
```

3. Tire a linha de uma placa. A camada e a largura por onde corre a maior parte do cobre da rede `MID` (uma pista de 0,2 mm em `Top`) dão uma microfita, resolvida sobre o empilhamento do trabalho.

```console
$ phoenix field2d examples/tutorials/tut.brd --stackup examples/guides/stackup_job.json --net MID
```

4. Junte o solucionador de campo ao relatório de `stackup`, para que a fórmula fechada e a solução de campo apareçam num só relatório.

```console
$ phoenix stackup examples/guides/stackup_job.json --freq-ghz 1 --field2d
```

5. Estime o aquecimento da rede `MID` com 0,5 A. O relatório lista a pista por camada, largura e espessura de cobre, e dá a subida pelo ajuste de IPC-2221 e pelo modelo de convecção. A subida fica abaixo do limite das definições e o código de saída é 0.

```console
$ phoenix thermal --currents examples/guides/currents_low.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

6. A mesma rede com 3 A. A subida excede o limite, a pista é marcada `[above the limit]` com o aviso `PHOENIX-HEAT-004` e o código de saída é 1.

```console
$ phoenix thermal --currents examples/guides/currents_high.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

## Como ler o resultado

* O solucionador toma a linha como quase-TEM numa malha retangular dentro de uma caixa com as paredes a potencial zero, o que deixa a impedância um pouco mais baixa do que numa região aberta. Os condutores são retângulos e a máscara de soldadura e a perda do cobre ficam de fora. Uma diferença para a fórmula fechada acima de `compare_warn_percent` é o aviso `PHOENIX-FIELD-009`.
* A estimativa de aquecimento faz passar toda a corrente da rede por cada pista e por cada via, pelo que uma rede que reparte a corrente é julgada do lado seguro. A IPC-2152 não é aplicada e todos os relatórios o dizem (`PHOENIX-HEAT-012`). Uma camada que o empilhamento não nomeia usa os valores por omissão das definições (`PHOENIX-HEAT-008`).
* Os dois comandos são simulacros e só escrevem um relatório JSON num ficheiro novo indicado com `--out`. As constantes e os limites estão em `config/field2d.cfg` e `config/heating.cfg`, não no código. O código de saída é 0 se tudo estiver bem, 1 se uma linha não pôde ser resolvida ou uma pista excede o limite, e 2 para argumentos ou entrada inutilizáveis.

## Ver também

* [docs/FIELD2D.md](../../../../docs/FIELD2D.md)
* [docs/THERMAL.md](../../../../docs/THERMAL.md)
* [docs/STACKUP.md](../../../../docs/STACKUP.md)
