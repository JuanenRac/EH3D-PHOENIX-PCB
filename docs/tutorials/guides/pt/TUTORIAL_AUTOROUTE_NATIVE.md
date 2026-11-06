# Encaminhar uma placa com o encaminhador automático integrado, com um par e com as vias que já existem na placa

`phoenix autoroute native` encaminha as ligações abertas de uma placa com uma pesquisa própria e não precisa de nenhum programa externo. `examples/guides/open.brd` tem uma rede cujas duas pastilhas não estão unidas; `examples/guides/open_pair.brd` tem as duas redes `D_P` e `D_N` de um par, também por unir. O plano só acrescenta pistas e vias, e as regras são verificadas antes e depois. Indica-se o ficheiro de regras porque as placas EAGLE dos exemplos não têm regras de projeto próprias.

Execute os comandos a partir da raiz do repositório; veja [como executar os guias](README.md).

1. Planeie os encaminhamentos. É um simulacro: o relatório lista as ligações encaminhadas, as pistas, vias e comprimento acrescentados e as ligações ainda abertas, e diz no fluxo de erro que nada foi escrito.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json examples/guides/open.brd
```

2. Escreva a placa com os encaminhamentos num ficheiro novo. O ficheiro é o JSON do modelo de projeto, seja qual for o seu nome, e um ficheiro que exista nunca é substituído.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --out build/open_routed.json examples/guides/open.brd
```

3. Encaminhe de novo a placa escrita: já não tem nenhuma ligação aberta, por isso não há nada a acrescentar.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json build/open_routed.json
```

4. Peça o par. `pair_dp_dn.json` diz que `D_P` e `D_N` são um par com pistas de 0,2 mm e 0,3 mm entre elas. Havendo espaço, as duas pistas são postas lado a lado numa camada e o relatório diz `coupled pair: p`; caso contrário, as duas redes são encaminhadas como redes comuns e uma nota com o código `PHOENIX-AROUTE-017` dá o motivo.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json examples/guides/open_pair.brd
```

5. Encaminhe só as duas redes do par e recuse um plano que deixe uma ligação aberta: com `--require-complete` o comando termina com código de saída 2 e o código `PHOENIX-AROUTE-016` se restar alguma ligação.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json --nets D_P,D_N --require-complete examples/guides/open_pair.brd
```

## Como ler o resultado

* Uma via da rede que já está na placa, sobre um nó da grelha, é uma mudança de camada gratuita: o percurso muda de camada aí e não põe nenhuma via nova. Com `via_snap_nm` maior que 0 em `config/autoroute_native.cfg` usa-se também uma via que esteja no máximo a essa distância de um nó, com uma nota `PHOENIX-AROUTE-019`. É a reutilização de vias; o plano nunca altera uma pista, uma via ou uma pastilha que já existisse.
* Um par só é acoplado quando cada rede tem exatamente uma ligação aberta e as quatro pastilhas partilham uma camada. As duas pistas têm o mesmo comprimento e mantêm a distância do par. Sem `--pairs` nem `--pair-config`, as redes de um par são encaminhadas como duas redes comuns e a verificação de pares do editor de placas julga o resultado depois.
* Um protótipo experimentado em pequenas placas feitas à mão e pelos testes. A pesquisa, os seus custos e os seus limites (que contam nós e nunca tempo) estão em `config/autoroute_native.cfg`. Se os encaminhamentos de uma rede juntos acrescentarem um erro de regras, essa rede fica de fora e é indicada com `PHOENIX-AROUTE-015`: o plano nunca acrescenta um erro que a placa não tinha.

## Ver também

* [docs/AUTOROUTE_NATIVE.md](../../../../docs/AUTOROUTE_NATIVE.md)
* [docs/DIFFPAIRS.md](../../../../docs/DIFFPAIRS.md)
* [docs/AUTOROUTE.md](../../../../docs/AUTOROUTE.md)
