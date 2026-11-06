# Hacer capturas de las ventanas con el modo de captura de phoenix-gui

`phoenix-gui --shot FICHERO.png` abre la ventana, espera unos fotogramas, guarda una imagen de ella como fichero PNG y se cierra sola. Así se hacen las imágenes del README. En este modo la ventana no lee ni escribe preferencias ni lista de ficheros recientes, de modo que una ejecución deja sus ajustes como estaban. `phoenix-gui` es el programa de la ventana, junto a `phoenix` en el archivo de la versión; se construye con la característica `gui`.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar las guías](README.md).

1. Haga la captura de la pantalla de inicio, con un tamaño, un tema y un número de fotogramas de espera. El tamaño es `ANCHOxALTO` en píxeles, cada lado de 320 a 8192; el tema es `dark`, `light` o `contrast`.

```console
phoenix-gui --shot build/start.png --size 1280x720 --theme dark --wait 6
```

2. Haga la captura de la ventana de una placa: `--open` da el fichero y la ventana mostrada es `document` por defecto. Una placa necesita `--open`; la pantalla de inicio no puede llevarlo.

```console
phoenix-gui --shot build/board.png --open examples/tutorials/led_board.brd --theme light --wait 12
```

3. Haga la captura de otra ventana con `--window`: `3d`, `panel`, `gerber`, `search`, `measures`, `layers`, `history`, `appearance`, `shortcuts` o `help` (`3d`, `panel` y `gerber` necesitan `--open`). `--lang` fija el idioma de la ventana.

```console
phoenix-gui --shot build/help_pt.png --window help --lang pt --size 1280x720
```

4. Haga todo el conjunto de imágenes de una versión con la herramienta del repositorio. `--dry-run` imprime las líneas de comando y no ejecuta nada; sin él la herramienta inicia el programa una vez por ventana, comprueba cada PNG (no puede estar vacío ni ser de un solo color, y debe pesar menos de 400 KB) y escribe los ficheros `phoenix-pcb-NN-VENTANA.png`.

```console
py -3.14 -I tools/make_shots.py --exe PATH/phoenix-gui --out build/screens --dry-run
```

## Cómo leer el resultado

* La imagen es lo que dibujó la ventana, en píxeles físicos, así que en una pantalla con escalado puede ser mayor que `--size`. El código de salida es 0 si se escribió la imagen, 1 si no y 2 si hay un error en los argumentos. Un fichero que exista no se sobrescribe salvo que se dé `--replace`; sin él la ejecución se detiene antes de abrir la ventana.
* Cada ejecución de la herramienta tiene su propia carpeta de ajustes dentro de una carpeta temporal, así que no se leen ni escriben sus ajustes ni sus ficheros recientes. La toma de la imagen en sí aún no se ha ejecutado en una pantalla, y no se afirma que ninguna imagen se vea bien hasta que usted la haya visto.

## Véase también

* [docs/VIEWER.md](../../../../docs/VIEWER.md)
* [tools/make_shots.py](../../../../tools/make_shots.py)
