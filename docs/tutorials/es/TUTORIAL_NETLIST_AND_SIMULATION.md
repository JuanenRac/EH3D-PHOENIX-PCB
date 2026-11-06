# Del esquema a la netlist y a la simulación

El fichero `examples/tutorials/divider.json` es un esquema con una fuente de 10 V y dos resistencias de 1 kΩ y 3 kΩ en serie. Se crea su netlist de simulación sin escribirla a mano, se simula y se comprueba el resultado con la fórmula del divisor: 10 V × 3 / (1 + 3) = 7,5 V.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar los tutoriales](README.md).

1. Imprima la netlist del esquema. No se escribe nada (simulacro) y antes se ejecuta la comprobación de reglas eléctricas. La primera línea es el título; después van los elementos, con los valores en la notación del simulador.

```console
# test
$ phoenix netlist examples/tutorials/divider.json
divider
~ R1 in out 1e3
~ R2 out 0 3e3
~ V1 in 0 1e1
```

2. Escríbala en un fichero nuevo. Un fichero que ya existe nunca se sobrescribe.

```console
# test
$ phoenix netlist examples/tutorials/divider.json --out build/divider.cir
~ written:
```

3. Calcule el punto de trabajo. La fórmula del divisor da 7,5 V en el nodo `out`.

```console
# test
$ phoenix sim build/divider.cir --op --probe "v(out)"
name,value
v(out),7.5e0
```

4. Sin fichero: `--schematic` crea la netlist del esquema y se la pasa directamente al solucionador.

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --op --probe "v(out)"
name,value
v(out),7.5e0
```

5. Barra la fuente de 0 V a 10 V en pasos de 5 V: la salida es siempre tres cuartos de la entrada.

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --dc V1 0 10 5 --probe "v(out)"
~ 5e0,3.75e0
~ 1e1,7.5e0
```

## Véase también

* [docs/SIM.md](../../SIM.md)
* [docs/NETLIST.md](../../NETLIST.md)
* [docs/ERC.md](../../ERC.md)
