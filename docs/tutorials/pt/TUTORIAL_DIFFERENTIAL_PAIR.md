# Rever a integridade de sinal de um par diferencial

A placa `examples/tutorials/diffpair.kicad_pcb` tem duas pistas de 0,25 mm, `D_P` e `D_N`, lado a lado ao longo de 20 mm com 0,75 mm entre os bordos. Verifica-se o par com um ficheiro que diz como deve ser, vê-se o resultado quando a separação está errada, lê-se em espanhol e estima-se a diafonia. As ferramentas de integridade de sinal são estimativas quase estáticas com fórmulas fechadas, não um solucionador de campo.

Execute os comandos a partir da raiz do repositório; veja [como executar os tutoriais](README.md).

1. O ficheiro `pairs.json` diz que o par é `D_P` e `D_N`, com pistas de 0,25 mm e uma separação de 0,75 mm. A placa coincide: sem erros nem avisos.

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs.json examples/tutorials/diffpair.kicad_pcb
~ pairs: 1, errors: 0, warnings: 0
~ pair p (D_P / D_N)
```

2. Agora o ficheiro pede uma separação de 0,3 mm e a placa tem 0,75 mm: a separação está fora de tolerância, o código é `PHOENIX-PAIR-003` e o código de saída é 1.

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ PHOENIX-PAIR-003
```

3. O mesmo relatório em espanhol. `--lang` aceita en, es, de, fr, it, ja, zh e pt.

```console
# test
$ phoenix pairs --lang es --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ La separación del par está fuera de tolerancia
```

4. Estime a diafonia entre as duas pistas a partir do comprimento em paralelo, da distância entre centros e do empilhamento. A estimativa é em percentagem do degrau do agressor.

```console
# test
$ phoenix si crosstalk examples/tutorials/diffpair.kicad_pcb
~ crosstalk: 1 coupled pairs of nets
```

5. As outras análises: as reflexões e terminações de uma rede, um diagrama de olho simplificado e todas juntas com o caminho de retorno (código de saída 1 se se exceder um limiar de `config/si.cfg`).

```console
$ phoenix si reflect --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si eye --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si check examples/tutorials/diffpair.kicad_pcb
```

## Ver também

* [docs/DIFFPAIRS.md](../../DIFFPAIRS.md)
* [docs/SI.md](../../SI.md)
* [docs/LENGTHS.md](../../LENGTHS.md)
* [docs/STACKUP.md](../../STACKUP.md)
