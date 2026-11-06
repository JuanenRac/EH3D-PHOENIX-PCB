# Adaptar una impedancia y sintetizar un filtro

Las herramientas RF trabajan con fórmulas de libros de texto y componentes ideales. Se diseña una red en L que adapta 100 Ω a 50 Ω a 100 MHz, un transformador de cuarto de onda y filtros en escalera a partir de las tablas de prototipos. Los resultados son un prototipo que no se ha comparado con un analizador de redes ni con un solucionador de campos.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar los tutoriales](README.md).

1. Adapte una carga de 100 Ω a una fuente de 50 Ω a 0,1 GHz. La red en L lleva una bobina en serie de 79,5775 nH y un condensador en paralelo de 15,9155 pF en la forma de paso bajo, y la dual en la forma de paso alto.

```console
# test
$ phoenix rf match l --source 50 --load 100 --freq-ghz 0.1
~ 79.5775 nH
~ 15.9155 pF
~ low-pass
~ high-pass
```

2. Un transformador de cuarto de onda entre 50 Ω y una carga de 200 Ω tiene la impedancia √(50 × 200) = 100 Ω.

```console
# test
$ phoenix rf match qw --load 200 --z0 50 --freq-ghz 1
~ transformer: 100.0000 ohm
```

3. Sintetice un filtro paso bajo: Butterworth, orden 3, corte a 0,1 GHz, con un barrido de tres puntos. Los valores del prototipo son 1, 2 y 1, así que el filtro lleva condensadores de 31,8310 pF y una bobina de 159,1549 nH entre ellos.

```console
# test
$ phoenix rf filter butterworth 3 lowpass --fc-ghz 0.1 --sweep-ghz 0.01,0.2,3
~ prototype g: 1.0000 2.0000 1.0000
~ capacitor 31.8310 pF
~ inductor 159.1549 nH
~ frequency, insertion loss, return loss
```

4. Un filtro paso banda: Chebyshev, orden 5, rizado de 0,1 dB, de 90 MHz a 110 MHz, en un sistema de 75 Ω.

```console
# test
$ phoenix rf filter chebyshev 5 bandpass --f1-ghz 0.09 --f2-ghz 0.11 --ripple-db 0.1 --z0 75
~ Chebyshev (0.1 dB ripple) filter of order 5, band-pass
```

5. Dibuje una red de adaptación en una placa a partir del apilado y la frecuencia, como operaciones reversibles con un informe. La lista de estructuras (líneas, curvas, stubs, filtros, acopladores, antenas) sale con `--list`.

```console
$ phoenix rfgen --list
```

## Véase también

* [docs/RF.md](../../RF.md)
* [docs/RFGEN.md](../../RFGEN.md)
* [docs/STACKUP.md](../../STACKUP.md)
* [docs/ORACLE.md](../../ORACLE.md)
