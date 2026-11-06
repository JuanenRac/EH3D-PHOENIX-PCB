# Do esquema à netlist e à simulação

O ficheiro `examples/tutorials/divider.json` é um esquema com uma fonte de 10 V e duas resistências de 1 kΩ e 3 kΩ em série. Cria-se a sua netlist de simulação sem a escrever à mão, simula-se e confere-se o resultado com a fórmula do divisor: 10 V × 3 / (1 + 3) = 7,5 V.

Execute os comandos a partir da raiz do repositório; veja [como executar os tutoriais](README.md).

1. Imprima a netlist do esquema. Não se escreve nada (simulacro) e a verificação das regras elétricas corre primeiro. A primeira linha é o título; seguem-se os elementos, com os valores na notação do simulador.

```console
# test
$ phoenix netlist examples/tutorials/divider.json
divider
~ R1 in out 1e3
~ R2 out 0 3e3
~ V1 in 0 1e1
```

2. Escreva-a num ficheiro novo. Um ficheiro que já existe nunca é sobrescrito.

```console
# test
$ phoenix netlist examples/tutorials/divider.json --out build/divider.cir
~ written:
```

3. Calcule o ponto de funcionamento. A fórmula do divisor dá 7,5 V no nó `out`.

```console
# test
$ phoenix sim build/divider.cir --op --probe "v(out)"
name,value
v(out),7.5e0
```

4. Sem ficheiro: `--schematic` cria a netlist a partir do esquema e entrega-a diretamente ao solucionador.

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --op --probe "v(out)"
name,value
v(out),7.5e0
```

5. Varra a fonte de 0 V a 10 V em passos de 5 V: a saída é sempre três quartos da entrada.

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --dc V1 0 10 5 --probe "v(out)"
~ 5e0,3.75e0
~ 1e1,7.5e0
```

## Ver também

* [docs/SIM.md](../../SIM.md)
* [docs/NETLIST.md](../../NETLIST.md)
* [docs/ERC.md](../../ERC.md)
