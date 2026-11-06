# Painelizar uma placa e escrever os ficheiros do painel

Dispõe-se duas vezes a placa de 30 mm por 20 mm, lado a lado e unidas por um corte em V, e escrevem-se os ficheiros Gerber e de furação de todo o painel. As definições do painel estão em `examples/tutorials/panel.cfg` (colunas, linhas, tipo de junção, folga e moldura); a lista completa é `config/panel.cfg`.

Execute os comandos a partir da raiz do repositório; veja [como executar os tutoriais](README.md).

1. Planeie o painel. Um simulacro imprime o tamanho, a junção, os furos e os ficheiros que seriam escritos, e não escreve nada.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ joint: V-score, 1 lines
~ dry run: nothing written
```

2. Escreva as camadas de cada cópia, os ficheiros de furação, o contorno com a linha de corte em V, o posicionamento e a lista de materiais numa pasta nova.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ written
```

3. Nunca se escreve numa pasta que existe: a segunda execução é recusada com o código `PHOENIX-PANEL-008` e código de saída 2.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
# exit 2
~~ PHOENIX-PANEL-008
```

4. Verifique a estrutura de um ficheiro Gerber do painel e desenhe o painel como desenho DXF para a parte mecânica.

```console
$ phoenix gerber-check build/panel/panel_01_top.gbr
$ phoenix dxf-export panel --out build/panel.dxf --panel-config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
```

## Ver também

* [docs/PANEL.md](../../PANEL.md)
* [docs/GERBER_IMPORT.md](../../GERBER_IMPORT.md)
* [docs/DXF.md](../../DXF.md)
* [docs/FABPACK.md](../../FABPACK.md)
