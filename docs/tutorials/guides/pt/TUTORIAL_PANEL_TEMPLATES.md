# Guardar várias disposições de painel num só ficheiro de definições com modelos

`examples/guides/panel_templates.cfg` contém uma disposição base (duas placas lado a lado, unidas por um corte em V, sem moldura) e dois modelos com nome. Cada modelo altera apenas as linhas que nomeia, e escolhe-se um com `--template`. A placa é a do tutorial do painel, de 30 mm por 20 mm; todos os comandos aqui são simulacros.

Execute os comandos a partir da raiz do repositório; veja [como executar os guias](README.md).

1. Sem modelo só se lê a base, e as linhas sob as secções de modelo são ignoradas: 60 mm por 20 mm, 2 x 1 placas.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg examples/tutorials/kicad/demo.kicad_pcb
```

2. O modelo `four` muda `columns` para 4 e nada mais: 120 mm por 20 mm, 4 x 1 placas.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four examples/tutorials/kicad/demo.kicad_pcb
```

3. O modelo `grid` muda `rows` para 2: 60 mm por 40 mm, 2 x 2 placas.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template grid examples/tutorials/kicad/demo.kicad_pcb
```

4. `--columns` e `--rows` continuam a prevalecer sobre um modelo: com `four` e `--columns 3` o painel tem 90 mm por 20 mm, 3 x 1 placas.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --columns 3 examples/tutorials/kicad/demo.kicad_pcb
```

5. Um nome que o ficheiro não tem é recusado com o código `PHOENIX-PANEL-001` e código de saída 2.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template ring examples/tutorials/kicad/demo.kicad_pcb
```

6. Escreva os ficheiros da disposição escolhida numa pasta nova, como no tutorial do painel.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --out build/panel_four examples/tutorials/kicad/demo.kicad_pcb
```

## Como ler o resultado

* As linhas anteriores à primeira linha `[template NOME]` são a base. Um nome tem de 1 a 32 letras, algarismos, `_` ou `-`; uma secção escrita de outro modo é recusada com `PHOENIX-PANEL-001` e o número da linha.
* A linha de tamanho é `panel W mm by H mm, C x R boards`. Com a junção em V e folga 0 as placas tocam-se, pelo que a largura é o número de colunas vezes 30 mm e a altura o número de linhas vezes 20 mm. Um modelo é uma forma de guardar uma disposição com um nome; não é uma definição de nenhum fabricante.

## Ver também

* [docs/PANEL.md](../../../../docs/PANEL.md)
* [config/panel.cfg](../../../../config/panel.cfg)
