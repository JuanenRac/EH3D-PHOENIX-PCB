# Editar uma biblioteca e gerar footprints IPC

Verifica-se uma biblioteca EAGLE, acrescenta-se-lhe um footprint com um ficheiro de operações (tudo ou nada, e fica uma cópia do original) e geram-se footprints pelo método IPC-7351B, com os seus nomes canónicos. As dimensões dos footprints gerados são um protótipo e não foram verificadas com nenhuma folha de dados.

Execute os comandos a partir da raiz do repositório; veja [como executar os tutoriais](README.md).

1. Verifique a biblioteca. Sem `--out` não se escreve nada; o relatório conta footprints, símbolos e conjuntos de dispositivos e lista os resultados.

```console
# test
$ phoenix libedit examples/tutorials/parts.lbr
~ 1 footprints, 1 symbols, 1 device sets
~ 0 errors, 0 warnings
~ dry run: nothing was written
```

2. Aplique um ficheiro de operações que cria o encapsulamento `HDR1X2` e lhe acrescenta dois pads. A biblioteca nova é escrita num ficheiro novo, lida de volta e comparada; ao lado fica uma cópia do original.

```console
# test
$ phoenix libedit --apply examples/tutorials/lib_ops.json --out build/parts_hdr.lbr examples/tutorials/parts.lbr
~ operation 3: add pad to HDR1X2
~ the file was read back and is the same library
```

3. Gere o footprint de uma resistência chip 0603 no nível de densidade B (nominal). O seu nome canónico é `RESC1608X55N`: o corpo tem 1,6 mm por 0,8 mm e 0,55 mm de altura. Sem `--out` é um simulacro que lista os pads.

```console
# test
$ phoenix ipc generate chip 0603 --density B
RESC1608X55N (density B,
~ dry run: nothing was written
```

4. Escreva-o num ficheiro `.lbr` novo (um nome `.kicad_mod` escreve em vez disso um footprint do KiCad).

```console
# test
$ phoenix ipc generate chip 0603 --density B --out build/RESC1608X55N.lbr
~ written: build/RESC1608X55N.lbr
```

5. O gerador mais simples `footprint` cria um footprint a partir de parâmetros (famílias chip, SOIC, TSSOP, QFN, QFP, DIP e barra de pinos).

```console
# test
$ phoenix footprint chip --param size=0603 --out build/chip.lbr
~ footprint CHIP_0603
~ written:
```

6. Verifique as bibliotecas criadas: cada uma tem um footprint.

```console
# test
$ phoenix libedit build/chip.lbr
~ 1 footprints
$ phoenix libedit build/RESC1608X55N.lbr
~ 1 footprints
```

7. Compare um footprint da sua biblioteca com o gerado, pad a pad (código de saída 1 se diferirem), e liste as famílias da biblioteca.

```console
$ phoenix ipc compare build/chip.lbr chip 0603
$ phoenix ipc list
```

## Ver também

* [docs/LIBRARY_EDIT.md](../../LIBRARY_EDIT.md)
* [docs/IPC_LIBRARY.md](../../IPC_LIBRARY.md)
* [docs/LIBINDEX.md](../../LIBINDEX.md)
* [docs/SANITIZER.md](../../SANITIZER.md)
