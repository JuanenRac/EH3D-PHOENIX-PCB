# Leer los buses de un esquema EAGLE

`examples/guides/bus.sch` es el esquema del primer tutorial con un bus `D[0..1]` dibujado en la hoja y dos redes, `D0` y `D1`, cuyos hilos terminan en el hilo del bus. El lector convierte el nombre del bus en un objeto bus con sus miembros y pone una entrada donde cada red encuentra el bus. Un bus es una forma de nombrar: sus miembros son redes normales.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar las guías](README.md).

1. Lea la jerarquía como JSON. La clave `buses` cuenta los buses de la hoja: 1. La lista de problemas no tiene ningún código de la familia `PHOENIX-BUS`, porque ambos miembros los usa una entrada.

```console
$ phoenix hier --json examples/guides/bus.sch
```

2. Lo mismo como texto: la lista de redes solo contiene las redes que unen al menos dos pines, aquí `MID`, así que los miembros `D0` y `D1` del bus, que no tienen pin, no aparecen en ella, y la última línea dice si hay problemas.

```console
$ phoenix hier examples/guides/bus.sch
```

3. Escriba el esquema como ficheros de KiCad. El bus se escribe como su etiqueta y una línea desde su ancla por los extremos de sus entradas, y cada entrada como un `bus_entry` con la etiqueta de su miembro.

```console
$ phoenix kicad-export --out build/bus.kicad_sch schematic examples/guides/bus.sch
```

## Cómo leer el resultado

* Un vector `prefijo[primero..último]` da los miembros `prefijo<primero>` a `prefijo<último>` (`D[0..1]` da `D0` y `D1`; un vector puede ir hacia abajo, como `A[2..0]`). El lector acepta también las listas de EAGLE (`A[0..3],B[0..3]`) y un nombre con dos puntos (`BUS:A[0..3]`), donde lo anterior a los dos puntos nombra el propio bus.
* El bus se ancla donde empieza su primer hilo. Se crea una entrada en el extremo de cada red de la hoja que tiene el nombre de un miembro y termina en un hilo del bus. Los hilos y etiquetas del bus, los nombres que no son vectores y un bus sin hilo no se leen y se cuentan como partes omitidas. Los códigos `PHOENIX-BUS-001` a `PHOENIX-BUS-007` dicen qué falla en un bus, y `phoenix hier` los lista.

## Véase también

* [docs/BUSES.md](../../../../docs/BUSES.md)
* [docs/HIERARCHY.md](../../../../docs/HIERARCHY.md)
* [docs/KICAD_EXPORT.md](../../../../docs/KICAD_EXPORT.md)
