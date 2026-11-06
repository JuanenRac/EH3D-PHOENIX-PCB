# Redondear las esquinas de un relleno de cobre y escribir alivios térmicos y límites de islas en los ficheros Gerber

La placa `examples/guides/pour.brd` tiene un relleno de masa en la capa `Top` alrededor de dos resistencias y una pista. Se rellena con esquinas vivas y con esquinas redondeadas y se comparan, y después se escriben los ficheros Gerber con un perfil que da alivio térmico a las almohadillas rectangulares del relleno y deja fuera sus islas pequeñas. Se da el fichero de reglas `rules_relaxed.json` porque una placa EAGLE sin reglas de diseño no tiene límites propios.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar las guías](README.md).

1. Rellene la zona con los ajustes del repositorio, que dejan todas las esquinas vivas (`corner_radius_nm = 0`). El informe da los polígonos y los nanómetros cuadrados de cobre.

```console
$ phoenix zones fill --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

2. Rellénela de nuevo con `zones_round.cfg`, que fija `corner_radius_nm = 1000000` (un arco de 1 mm). Se redondean las esquinas del relleno que miran hacia fuera y las esquinas de los huecos del alivio térmico; compare el área de cobre con la primera ejecución.

```console
$ phoenix zones fill --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

3. Mida el relleno redondeado frente a la placa y las reglas. El redondeo quita cobre del relleno o lo añade solo dentro de un hueco de un alivio térmico, así que el relleno no se acerca al cobre de otra red y la comprobación no halla errores de separación.

```console
$ phoenix zones check --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

4. Escriba primero los ficheros Gerber con el perfil genérico. La almohadilla `R2.2` de la red de masa está dentro del relleno y queda unida a él sin alivio.

```console
$ phoenix export --out build/pour_plain examples/guides/pour.brd
```

5. Ahora con el perfil `pour` de `export_pour.json`: cuatro radios (`gerber.zone_thermal_spokes`), un hueco y un radio de 0,25 mm, el mismo alivio para las almohadillas rectangulares y oblongas (`gerber.zone_thermal_non_round`) y ninguna isla por debajo de 1 milímetro cuadrado (`gerber.zone_island_min_area_nm2`). Compare el fichero de cobre de la capa `Top` en las dos carpetas: el segundo lleva los huecos del alivio escritos como regiones de polaridad clara alrededor de la almohadilla.

```console
$ phoenix export --profiles examples/guides/export_pour.json --profile pour --out build/pour_relief examples/guides/pour.brd
```

## Cómo leer el resultado

* En los ajustes del relleno, `corner_radius_nm` es el radio del arco (cero deja todas las esquinas vivas) y `corner_tolerance_nm` cuánto pueden alejarse del arco sus cuerdas. Un arco toma como mucho la mitad de cada lado junto a la esquina, así que un radio que no cabe se reduce. Las esquinas de los huecos que otro cobre deja en el relleno no se redondean.
* En la exportación, un alivio tiene cuatro radios o dos (`gerber.zone_thermal_spokes` es 0, 2 o 4; 0 deja las almohadillas unidas en sólido). Una almohadilla girada un ángulo que no es recto queda en sólido y lo dice (`PHOENIX-EXPORT-022`). Una isla menor que `gerber.zone_island_min_area_nm2` no se escribe y el informe lo dice (`PHOENIX-EXPORT-021`); cero escribe todas las zonas.

## Véase también

* [docs/ZONES.md](../../../../docs/ZONES.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
