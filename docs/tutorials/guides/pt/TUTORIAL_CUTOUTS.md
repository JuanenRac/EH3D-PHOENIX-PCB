# Levar as aberturas do contorno e os recortes de um preenchimento pelos formatos de ficheiro

`examples/guides/cutout.brd` é uma placa EAGLE com dois tipos de abertura: um quadrado fechado de traços na camada 20 dentro do contorno (uma abertura da placa, uma ranhura ou janela) e um polígono na camada 41 dentro do preenchimento da camada `Top` (um recorte do preenchimento, onde não pode haver cobre). Escreve-se a placa em DXF, lê-se o DXF de volta, escrevem-se ficheiros KiCad e Gerber, e vê-se para onde vai cada abertura.

Execute os comandos a partir da raiz do repositório; veja [como executar os guias](README.md).

1. Liste o que um desenho DXF da placa conteria. O contorno e a abertura são dois contornos fechados na camada `OUTLINE`; o relatório diz `layer OUTLINE: 2` e que nada foi escrito.

```console
# test
$ phoenix dxf-export board examples/guides/cutout.brd
~ layer OUTLINE: 2
~ dry run: nothing was written
```

2. Escreva o desenho num ficheiro novo.

```console
# test
$ phoenix dxf-export board examples/guides/cutout.brd --out build/cutout.dxf
~ written: build/cutout.dxf
```

3. Leia o desenho de volta na placa. O maior contorno exterior é o contorno da placa e o que está dentro dele é mantido como recorte do contorno (`PHOENIX-DXF-015`, informação). A operação é um simulacro, verificado pelas regras de projeto antes e depois, por isso indica-se o ficheiro de regras.

```console
$ phoenix dxf-import build/cutout.dxf --board examples/guides/cutout.brd --layer OUTLINE --rules examples/tutorials/rules_relaxed.json
```

4. Escreva uma placa KiCad. O contorno e a abertura vão para `Edge.Cuts`, e o recorte do preenchimento passa a ser uma área de regras na sua camada que mantém o preenchimento de fora e permite pistas, vias e pastilhas. O relatório lista o que foi escrito e o que o formato não consegue dizer.

```console
$ phoenix kicad-export --out build/cutout.kicad_pcb board examples/guides/cutout.brd
```

5. Escreva os ficheiros de fabrico. O ficheiro de perfil tem o contorno e depois a abertura, cada um um contorno fechado, e no ficheiro de cobre o recorte do preenchimento é escrito como uma região de polaridade clara depois do preenchimento.

```console
$ phoenix export --out build/cutout_fab examples/guides/cutout.brd
```

## Como ler o resultado

* De onde vem uma abertura: numa placa EAGLE, os traços da camada 20 que se seguem e se fecham, dentro do maior anel desse tipo, são aberturas da placa; um polígono na camada 41 (`tRestrict`, a de cima) ou 42 (a de baixo) dentro de um preenchimento dessa camada é um recorte do preenchimento. Numa placa KiCad, os contornos fechados de `Edge.Cuts` dentro do maior são as aberturas, e num DXF os contornos dentro do maior contorno exterior.
* Cada formato diz o que pode. Os recortes de um preenchimento são áreas de regras no KiCad, áreas de restrição no EAGLE e áreas de exclusão no ficheiro Specctra do encaminhador automático (que não tem buraco num plano, pelo que as pistas e as vias também ficam de fora). O painel leva os dois tipos de abertura para cada cópia, movidos, rodados e espelhados com ela.

## Ver também

* [docs/DXF.md](../../../../docs/DXF.md)
* [docs/KICAD_EXPORT.md](../../../../docs/KICAD_EXPORT.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/PANEL.md](../../../../docs/PANEL.md)
