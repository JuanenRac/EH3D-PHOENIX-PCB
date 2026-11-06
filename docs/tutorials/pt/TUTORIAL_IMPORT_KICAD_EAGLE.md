# Importar um projeto KiCad ou EAGLE e revê-lo

Lê-se um projeto KiCad completo para o modelo de projeto do programa, vê-se o que ficou e o que ficou de fora, escreve-se o modelo como ficheiros novos e verifica-se a placa com um ficheiro de regras. Depois faz-se a mesma revisão com uma placa EAGLE.

Execute os comandos a partir da raiz do repositório; veja [como executar os tutoriais](README.md).

1. Leia o projeto (o `.kicad_pro` com o `.kicad_sch` e o `.kicad_pcb` do mesmo nome). É um simulacro: o relatório diz o que foi lido, quanto ficou e o que o modelo não guarda.

```console
# test
$ phoenix kicad-import examples/tutorials/kicad/demo.kicad_pro
~ usable: yes
~ components: 2
~ tracks: 1
~ vias: 1
~ dry run: nothing was written
```

2. Escreva o modelo como ficheiros novos com o nome indicado. Um ficheiro que existe, ou uma entrada, nunca é sobrescrito.

```console
# test
$ phoenix kicad-import --out build/demo.json examples/tutorials/kicad/demo.kicad_pro
~ written:
```

3. Verifique a placa importada. Uma placa KiCad não dá regras de projeto ao programa, por isso os limites vêm de um ficheiro de regras. O ficheiro pede pistas de pelo menos 0,3 mm e a pista da placa tem 0,25 mm: um resultado confirmado e código de saída 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
Confirmed: 1
~ PHOENIX-CHECK-001
```

4. Um ficheiro EAGLE: pergunte o que uma reescrita mantém. O relatório prova que escrever a árvore XML e lê-la de novo nada altera, e lista o que o modelo de projeto simplifica ou deixa de fora.

```console
# test
$ phoenix eagle-check examples/tutorials/tut.brd
board (version 9.6.2)
~ rewrite of the tree: identical
```

5. Aplique o mesmo ficheiro de regras à placa EAGLE. A pista da rede `MID` tem 0,2 mm.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/tut.brd
# exit 1
Confirmed: 1
```

6. Confronte o esquema EAGLE com a placa EAGLE: o relatório lista as redes em falta, os pinos por ligar e o cobre ainda por encaminhar.

```console
$ phoenix connectivity examples/tutorials/tut.sch examples/tutorials/tut.brd
```

## Ver também

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD.md](../../KICAD.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/RULES.md](../../RULES.md)
