# Llevar las aberturas del contorno y los recortes de un relleno por los formatos de fichero

`examples/guides/cutout.brd` es una placa EAGLE con dos clases de abertura: un cuadrado cerrado de hilos en la capa 20 dentro del contorno (una abertura de la placa, una ranura o ventana) y un polígono en la capa 41 dentro del relleno de la capa `Top` (un recorte del relleno, donde no debe haber cobre). Se escribe la placa en DXF, se lee de vuelta el DXF y se escriben ficheros de KiCad y Gerber, y se ve adónde va cada abertura.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar las guías](README.md).

1. Liste lo que tendría un dibujo DXF de la placa. El contorno y la abertura son dos contornos cerrados en la capa `OUTLINE`; el informe dice `layer OUTLINE: 2` y que no se escribió nada.

```console
$ phoenix dxf-export board examples/guides/cutout.brd
```

2. Escriba el dibujo en un fichero nuevo.

```console
$ phoenix dxf-export board examples/guides/cutout.brd --out build/cutout.dxf
```

3. Lea el dibujo de vuelta en la placa. El contorno exterior mayor es el contorno de la placa y el que está dentro se conserva como recorte del contorno (`PHOENIX-DXF-015`, información). La operación es un simulacro, comprobado con las reglas de diseño antes y después, por eso se da el fichero de reglas.

```console
$ phoenix dxf-import build/cutout.dxf --board examples/guides/cutout.brd --layer OUTLINE --rules examples/tutorials/rules_relaxed.json
```

4. Escriba una placa de KiCad. El contorno y la abertura van a `Edge.Cuts`, y el recorte del relleno pasa a ser un área de reglas en su capa que mantiene fuera el relleno y permite pistas, vías y almohadillas. El informe lista lo que se escribió y lo que el formato no puede decir.

```console
$ phoenix kicad-export --out build/cutout.kicad_pcb board examples/guides/cutout.brd
```

5. Escriba los ficheros de fabricación. El fichero de perfil lleva el contorno y después la abertura, cada uno un contorno cerrado, y en el fichero de cobre el recorte del relleno se escribe como una región de polaridad clara después del relleno.

```console
$ phoenix export --out build/cutout_fab examples/guides/cutout.brd
```

## Cómo leer el resultado

* De dónde viene una abertura: en una placa EAGLE, los hilos de la capa 20 que se siguen y se cierran, dentro del mayor anillo de ese tipo, son aberturas de la placa; un polígono en la capa 41 (`tRestrict`, la superior) o 42 (la inferior) dentro de un relleno de esa capa es un recorte del relleno. En una placa KiCad, los contornos cerrados de `Edge.Cuts` dentro del mayor son las aberturas, y en un DXF los contornos dentro del mayor contorno exterior.
* Cada formato dice lo que puede. Los recortes de un relleno son áreas de reglas en KiCad, áreas de restricción en EAGLE y áreas de exclusión en el fichero Specctra del autorruteador (que no tiene agujero en un plano, así que también se excluyen pistas y vías). El panel lleva los dos tipos de abertura a cada copia, movidos, girados y reflejados con ella.

## Véase también

* [docs/DXF.md](../../../../docs/DXF.md)
* [docs/KICAD_EXPORT.md](../../../../docs/KICAD_EXPORT.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/PANEL.md](../../../../docs/PANEL.md)
