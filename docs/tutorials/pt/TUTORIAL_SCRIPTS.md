# Usar scripts: uma matriz de LED e um painel

Os scripts da linguagem `phoenix-script/2` têm variáveis, ciclos com limite, consultas sobre uma placa e edições que formam um único passo reversível, numa sandbox: sem rede, sem programas, sem ficheiros fora da pasta de dados das definições. Uma execução é um simulacro a menos que `--out` indique um ficheiro novo. Os exemplos estão em `examples/scripts/`.

Execute os comandos a partir da raiz do repositório; veja [como executar os tutoriais](README.md).

1. Liste os scripts de exemplo que vêm com o programa e imprima um.

```console
# test
$ phoenix script list
~ led_matrix
~ panelize_matrix
$ phoenix script list --show led_matrix
phoenix-script/2 board
```

2. Verifique um script sem o executar: sintaxe, nomes, número de argumentos, o documento a que cada função se aplica e uma estimativa dos passos.

```console
# test
$ phoenix script check examples/scripts/led_matrix.phs
```

3. Execute a matriz de LED numa placa cujo footprint é `LED0603`: seis LED com passo de 2,5 mm, com o segundo pad de cada um numa mesma rede. A placa não muda: é um simulacro.

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --board examples/tutorials/led_board.brd
~ placed 6 LEDs on net LEDCOM
~ pads on the net: 6
~ dry run: nothing was written
```

4. O script de painelização copia as duas peças da placa numa matriz de 2 por 2: de 2 componentes para 8.

```console
# test
$ phoenix script run examples/scripts/panelize_matrix.phs --board examples/tutorials/tut.brd
~ copies: 4
~ components: 2 -> 8
```

5. Com `--out` o resultado é escrito uma vez num ficheiro novo. Um ficheiro que existe nunca é substituído.

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
~ written
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
# exit 2
~~ never replaced
```

## Ver também

* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/SCRIPT_API.md](../../SCRIPT_API.md)
* [docs/SCRIPT_EXAMPLES.md](../../SCRIPT_EXAMPLES.md)
