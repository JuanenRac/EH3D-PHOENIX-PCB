# Tirar imagens das janelas com o modo de imagem de phoenix-gui

`phoenix-gui --shot FICHEIRO.png` abre a janela, espera alguns fotogramas, guarda uma imagem dela como ficheiro PNG e fecha-se sozinha. É assim que se fazem as imagens do README. Neste modo a janela não lê nem escreve preferências nem lista de ficheiros recentes, pelo que uma execução deixa as suas definições como estavam. `phoenix-gui` é o programa da janela, ao lado de `phoenix` no arquivo da versão; constrói-se com a funcionalidade `gui`.

Execute os comandos a partir da raiz do repositório; veja [como executar os guias](README.md).

1. Tire a imagem do ecrã inicial, com um tamanho, um tema e um número de fotogramas de espera. O tamanho é `LARGURAxALTURA` em píxeis, cada lado de 320 a 8192; o tema é `dark`, `light` ou `contrast`.

```console
phoenix-gui --shot build/start.png --size 1280x720 --theme dark --wait 6
```

2. Tire a imagem da janela de uma placa: `--open` indica o ficheiro e a janela mostrada é `document` por omissão. Uma placa precisa de `--open`; o ecrã inicial não pode ter um.

```console
phoenix-gui --shot build/board.png --open examples/tutorials/led_board.brd --theme light --wait 12
```

3. Tire a imagem de outra janela com `--window`: `3d`, `panel`, `gerber`, `search`, `measures`, `layers`, `history`, `appearance`, `shortcuts` ou `help` (`3d`, `panel` e `gerber` precisam de `--open`). `--lang` define o idioma da janela.

```console
phoenix-gui --shot build/help_pt.png --window help --lang pt --size 1280x720
```

4. Tire o conjunto completo de imagens de uma versão com a ferramenta do repositório. `--dry-run` imprime as linhas de comando e não executa nada; sem ele a ferramenta inicia o programa uma vez por janela, verifica cada PNG (não pode estar vazio nem ter uma só cor, e tem de pesar menos de 400 KB) e escreve os ficheiros `phoenix-pcb-NN-JANELA.png`.

```console
py -3.14 -I tools/make_shots.py --exe PATH/phoenix-gui --out build/screens --dry-run
```

## Como ler o resultado

* A imagem é o que a janela desenhou, em píxeis físicos, pelo que num ecrã com escala pode ser maior do que `--size`. O código de saída é 0 se a imagem foi escrita, 1 se não foi e 2 por um erro nos argumentos. Um ficheiro que exista não é sobrescrito a menos que se dê `--replace`; sem ele a execução pára antes de a janela se abrir.
* Cada execução da ferramenta tem a sua própria pasta de definições dentro de uma pasta temporária, pelo que as suas definições e os ficheiros recentes não são lidos nem escritos. A captura em si ainda não foi executada num ecrã, e não se afirma que nenhuma imagem esteja certa até a ter visto.

## Ver também

* [docs/VIEWER.md](../../../../docs/VIEWER.md)
* [tools/make_shots.py](../../../../tools/make_shots.py)
