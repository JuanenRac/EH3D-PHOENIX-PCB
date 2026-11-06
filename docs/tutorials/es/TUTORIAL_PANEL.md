# Panelizar una placa y escribir los ficheros del panel

Se dispone dos veces la placa de 30 mm por 20 mm, una junto a otra y unidas por un corte en V, y se escriben los ficheros Gerber y de taladros de todo el panel. Los ajustes del panel están en `examples/tutorials/panel.cfg` (columnas, filas, tipo de unión, hueco y marco); la lista completa de ajustes es `config/panel.cfg`.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar los tutoriales](README.md).

1. Planifique el panel. Un simulacro imprime su tamaño, la unión, los taladros y los ficheros que se escribirían, y no escribe nada.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ joint: V-score, 1 lines
~ dry run: nothing written
```

2. Escriba las capas de cada copia, los ficheros de taladros, el contorno con la línea de corte en V, la posición y la lista de materiales en una carpeta nueva.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ written
```

3. En una carpeta que existe nunca se escribe: la segunda ejecución se rechaza con el código `PHOENIX-PANEL-008` y código de salida 2.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
# exit 2
~~ PHOENIX-PANEL-008
```

4. Compruebe la estructura de un fichero Gerber del panel y dibuje el panel como dibujo DXF para la parte mecánica.

```console
$ phoenix gerber-check build/panel/panel_01_top.gbr
$ phoenix dxf-export panel --out build/panel.dxf --panel-config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
```

## Véase también

* [docs/PANEL.md](../../PANEL.md)
* [docs/GERBER_IMPORT.md](../../GERBER_IMPORT.md)
* [docs/DXF.md](../../DXF.md)
* [docs/FABPACK.md](../../FABPACK.md)
