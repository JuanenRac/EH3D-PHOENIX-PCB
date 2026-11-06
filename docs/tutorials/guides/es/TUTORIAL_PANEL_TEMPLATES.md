# Guardar varias disposiciones de panel en un solo fichero de ajustes con plantillas

`examples/guides/panel_templates.cfg` contiene una disposición base (dos placas una junto a otra, unidas por un corte en V, sin marco) y dos plantillas con nombre. Cada plantilla cambia solo las líneas que nombra, y se elige una con `--template`. La placa es la del tutorial del panel, de 30 mm por 20 mm; todos los comandos son simulacros.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar las guías](README.md).

1. Sin plantilla solo se lee la base, y se ignoran las líneas bajo las secciones de plantilla: 60 mm por 20 mm, 2 x 1 placas.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg examples/tutorials/kicad/demo.kicad_pcb
```

2. La plantilla `four` cambia `columns` a 4 y nada más: 120 mm por 20 mm, 4 x 1 placas.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four examples/tutorials/kicad/demo.kicad_pcb
```

3. La plantilla `grid` cambia `rows` a 2: 60 mm por 40 mm, 2 x 2 placas.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template grid examples/tutorials/kicad/demo.kicad_pcb
```

4. `--columns` y `--rows` siguen prevaleciendo sobre una plantilla: con `four` y `--columns 3` el panel es de 90 mm por 20 mm, 3 x 1 placas.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --columns 3 examples/tutorials/kicad/demo.kicad_pcb
```

5. Un nombre que el fichero no tiene se rechaza con el código `PHOENIX-PANEL-001` y código de salida 2.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template ring examples/tutorials/kicad/demo.kicad_pcb
```

6. Escriba los ficheros de la disposición elegida en una carpeta nueva, como en el tutorial del panel.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --out build/panel_four examples/tutorials/kicad/demo.kicad_pcb
```

## Cómo leer el resultado

* Las líneas anteriores a la primera línea `[template NOMBRE]` son la base. Un nombre tiene de 1 a 32 letras, cifras, `_` o `-`; una sección escrita de otro modo se rechaza con `PHOENIX-PANEL-001` y el número de la línea.
* La línea de tamaño es `panel W mm by H mm, C x R boards`. Con la unión en V y hueco 0 las placas se tocan, de modo que el ancho es el número de columnas por 30 mm y el alto el número de filas por 20 mm. Una plantilla es una forma de guardar una disposición con un nombre; no es un ajuste de ningún fabricante.

## Véase también

* [docs/PANEL.md](../../../../docs/PANEL.md)
* [config/panel.cfg](../../../../config/panel.cfg)
