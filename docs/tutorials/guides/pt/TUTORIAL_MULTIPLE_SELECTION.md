# Selecionar vários objetos no editor de placas e movê-los, alinhá-los ou apagá-los como um só passo

A seleção do editor de placas é um conjunto ordenado de componentes, pistas, vias, zonas e elementos de serigrafia. Sem ecrã, comanda-se com `phoenix boardedit` e um ficheiro de operações: `select` altera a seleção, e `move_selection`, `delete_selection` e `align_selection` atuam sobre ela como um só passo do histórico com inversa exata. `examples/guides/select_ops.json` seleciona com um retângulo o que fica inteiro lá dentro, aqui a resistência R1 e não a pista, e move-a 2 mm para cima.

Execute os comandos a partir da raiz do repositório; veja [como executar os guias](README.md).

1. Aplique as operações como simulacro. O relatório lista as três operações e os erros de regras antes e depois; nada é escrito. Indica-se o ficheiro de regras porque a placa EAGLE não tem regras de projeto próprias.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json examples/tutorials/tut.brd
```

2. Escreva a placa editada num ficheiro novo (`.json` para o modelo de projeto, `.brd` para EAGLE). A entrada nunca é sobrescrita. O código de saída é 0 se as regras não provarem nenhum erro no resultado e 1 se provarem algum.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --out build/moved.json examples/tutorials/tut.brd
```

3. Imprima o relatório como JSON, que um script pode ler: a chave `rule_errors` é o número de erros que as regras provam no resultado.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --json examples/tutorials/tut.brd
```

## Como ler o resultado

* Os passos de `select` são `add`, `remove` e `toggle` (os objetos de `items`, escritos `{"kind": "component", "id": "..."}`), `net` (todos os objetos de uma rede, pelo seu id), `rect` (um retângulo entre `from` e `to`, em nanómetros como `x_nm` e `y_nm`; `mode` é `contained` ou `crossed`, que também apanha o que o retângulo toca), `all`, `invert` e `clear`. Aplicam-se por ordem e tudo ou nada, e a seleção não faz parte do histórico.
* `move_selection` move por `dx_nm` e `dy_nm` (cima é positivo), arredondado à grelha a menos que `snap` seja false; as pistas não selecionadas não seguem um componente, por isso selecione primeiro a rede para levar o cobre. `delete_selection` apaga a seleção. `align_selection` aceita `left`, `right`, `top`, `bottom`, `center_x`, `center_y` (pelo menos dois objetos) e `distribute_x`, `distribute_y` (pelo menos três). Um componente bloqueado recusa toda a operação (`PHOENIX-BEDIT-006`), uma seleção vazia é `PHOENIX-BEDIT-009`.
* Na janela, com a ferramenta Selecionar: Shift ou Ctrl+clique alterna um objeto; Shift ou Ctrl+arrastar num sítio vazio acrescenta o que fica inteiro dentro do retângulo (também zonas e elementos de serigrafia); Alt+arrastar acrescenta ainda o que o retângulo toca; `N` seleciona a rede sob o ponteiro; Ctrl+A seleciona tudo; Esc larga a seleção; as setas movem-na um passo da grelha (Shift: dez passos); Delete apaga-a; Alt+Esquerda, Direita, Cima, Baixo, Alt+C e Alt+M alinham-na. Cada uma é um passo do histórico e Ctrl+Z devolve os mesmos bytes. A janela nunca foi vista num ecrã nem usada com um rato.

## Ver também

* [docs/BOARD_EDIT.md](../../../../docs/BOARD_EDIT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
* [docs/VIEWER.md](../../../../docs/VIEWER.md)
