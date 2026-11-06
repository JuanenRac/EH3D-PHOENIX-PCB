# Adaptar uma impedância e sintetizar um filtro

As ferramentas RF trabalham com fórmulas de manuais e componentes ideais. Projeta-se uma rede em L que adapta 100 Ω a 50 Ω a 100 MHz, um transformador de quarto de onda e filtros em escada a partir das tabelas de protótipos. Os resultados são um protótipo que não foi comparado com um analisador de redes nem com um solucionador de campo.

Execute os comandos a partir da raiz do repositório; veja [como executar os tutoriais](README.md).

1. Adapte uma carga de 100 Ω a uma fonte de 50 Ω a 0,1 GHz. A rede em L tem uma bobina em série de 79,5775 nH e um condensador em paralelo de 15,9155 pF na forma passa-baixo, e a dual na forma passa-alto.

```console
# test
$ phoenix rf match l --source 50 --load 100 --freq-ghz 0.1
~ 79.5775 nH
~ 15.9155 pF
~ low-pass
~ high-pass
```

2. Um transformador de quarto de onda entre 50 Ω e uma carga de 200 Ω tem a impedância √(50 × 200) = 100 Ω.

```console
# test
$ phoenix rf match qw --load 200 --z0 50 --freq-ghz 1
~ transformer: 100.0000 ohm
```

3. Sintetize um filtro passa-baixo: Butterworth, ordem 3, corte a 0,1 GHz, com uma varredura de três pontos. Os valores do protótipo são 1, 2 e 1, por isso o filtro tem condensadores de 31,8310 pF e, entre eles, uma bobina de 159,1549 nH.

```console
# test
$ phoenix rf filter butterworth 3 lowpass --fc-ghz 0.1 --sweep-ghz 0.01,0.2,3
~ prototype g: 1.0000 2.0000 1.0000
~ capacitor 31.8310 pF
~ inductor 159.1549 nH
~ frequency, insertion loss, return loss
```

4. Um filtro passa-banda: Chebyshev, ordem 5, ondulação de 0,1 dB, de 90 MHz a 110 MHz, num sistema de 75 Ω.

```console
# test
$ phoenix rf filter chebyshev 5 bandpass --f1-ghz 0.09 --f2-ghz 0.11 --ripple-db 0.1 --z0 75
~ Chebyshev (0.1 dB ripple) filter of order 5, band-pass
```

5. Desenhe uma rede de adaptação numa placa a partir do empilhamento e da frequência, como operações reversíveis com um relatório. A lista de estruturas (linhas, curvas, stubs, filtros, acopladores, antenas) obtém-se com `--list`.

```console
$ phoenix rfgen --list
```

## Ver também

* [docs/RF.md](../../RF.md)
* [docs/RFGEN.md](../../RFGEN.md)
* [docs/STACKUP.md](../../STACKUP.md)
* [docs/ORACLE.md](../../ORACLE.md)
