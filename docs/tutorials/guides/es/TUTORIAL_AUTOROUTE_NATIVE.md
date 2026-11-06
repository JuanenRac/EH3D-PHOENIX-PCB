# Rutar una placa con el autorruteador integrado, con un par y con las vías que ya hay en la placa

`phoenix autoroute native` rutea las conexiones abiertas de una placa con una búsqueda propia y no necesita ningún programa externo. `examples/guides/open.brd` tiene una red cuyas dos almohadillas no están unidas; `examples/guides/open_pair.brd` tiene las dos redes `D_P` y `D_N` de un par, también sin unir. El plan solo añade pistas y vías, y las reglas se comprueban antes y después. Se da el fichero de reglas porque las placas EAGLE de los ejemplos no tienen reglas de diseño propias.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar las guías](README.md).

1. Planifique las rutas. Es un simulacro: el informe lista las conexiones ruteadas, las pistas, vías y longitud añadidas y las conexiones aún abiertas, y dice en el flujo de error que no se escribió nada.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json examples/guides/open.brd
```

2. Escriba la placa con las rutas en un fichero nuevo. El fichero es el JSON del modelo de diseño, sea cual sea su nombre, y nunca se reemplaza uno que exista.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --out build/open_routed.json examples/guides/open.brd
```

3. Rutee de nuevo la placa escrita: no le queda ninguna conexión abierta, así que no hay nada que añadir.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json build/open_routed.json
```

4. Pida el par. `pair_dp_dn.json` dice que `D_P` y `D_N` son un par con pistas de 0,2 mm y 0,3 mm entre ellas. Si hay sitio, las dos pistas se colocan una junto a otra en una capa y el informe dice `coupled pair: p`; si no, las dos redes se rutean como redes normales y una nota con el código `PHOENIX-AROUTE-017` da el motivo.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json examples/guides/open_pair.brd
```

5. Rutee solo las dos redes del par y rechace un plan que deje una conexión abierta: con `--require-complete` el comando termina con código de salida 2 y el código `PHOENIX-AROUTE-016` si queda alguna conexión.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json --nets D_P,D_N --require-complete examples/guides/open_pair.brd
```

## Cómo leer el resultado

* Una vía de la red que ya está en la placa, sobre un nodo de la rejilla, es un cambio de capa gratuito: la ruta cambia de capa allí y no pone ninguna vía nueva. Con `via_snap_nm` mayor que 0 en `config/autoroute_native.cfg` también se usa una vía que esté como mucho a esa distancia de un nodo, con una nota `PHOENIX-AROUTE-019`. Es la reutilización de vías; el plan nunca cambia una pista, una vía ni una almohadilla que ya estuviera.
* Un par se acopla solo cuando cada red tiene exactamente una conexión abierta y las cuatro almohadillas comparten una capa. Las dos pistas tienen la misma longitud y mantienen la separación del par. Sin `--pairs` ni `--pair-config` las redes de un par se rutean como dos redes normales y la comprobación de pares del editor de placas juzga el resultado después.
* Un prototipo probado en placas pequeñas hechas a mano y por las pruebas. La búsqueda, sus costes y sus límites (que cuentan nodos y nunca tiempo) están en `config/autoroute_native.cfg`. Si las rutas de una red juntas añaden un error de reglas, esa red se deja fuera y se informa con `PHOENIX-AROUTE-015`: el plan nunca añade un error que la placa no tuviera.

## Véase también

* [docs/AUTOROUTE_NATIVE.md](../../../../docs/AUTOROUTE_NATIVE.md)
* [docs/DIFFPAIRS.md](../../../../docs/DIFFPAIRS.md)
* [docs/AUTOROUTE.md](../../../../docs/AUTOROUTE.md)
