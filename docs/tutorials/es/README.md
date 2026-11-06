# Tutoriales

## Cómo ejecutar los tutoriales

Ejecute cada comando desde la raíz del repositorio. Los comandos solo escriben ficheros nuevos o una carpeta nueva dentro de `build/`, que se crea antes con `mkdir build`; nada de lo que existe se reemplaza. La salida mostrada es el comienzo de lo que imprime cada comando. Los bloques que empiezan por `# test` los ejecutan las pruebas del repositorio, que comparan el comienzo de la salida (una línea que empieza por `~` debe aparecer en algún punto de la salida). Todas las herramientas de este programa son prototipos que no se han comparado con una herramienta de referencia.

La ayuda del programa está en el programa: `phoenix help` lista los temas y los comandos; `phoenix help sim` y `phoenix help --lang es sim` explican un tema o un comando.

## Tutoriales

1. [Del esquema a la netlist y a la simulación](TUTORIAL_NETLIST_AND_SIMULATION.md)
1. [De la placa a los ficheros de fabricación](TUTORIAL_BOARD_TO_FABRICATION.md)
1. [Importar un proyecto de KiCad o EAGLE y revisarlo](TUTORIAL_IMPORT_KICAD_EAGLE.md)
1. [Panelizar una placa y escribir los ficheros del panel](TUTORIAL_PANEL.md)
1. [Simular un filtro RC y un amplificador inversor](TUTORIAL_FILTER_AND_AMPLIFIER.md)
1. [Editar una biblioteca y generar huellas IPC](TUTORIAL_LIBRARY_AND_FOOTPRINTS.md)
1. [Adaptar una impedancia y sintetizar un filtro](TUTORIAL_RF_MATCHING_FILTER.md)
1. [Revisar la integridad de señal de un par diferencial](TUTORIAL_DIFFERENTIAL_PAIR.md)
1. [Usar scripts: una matriz de LED y un panel](TUTORIAL_SCRIPTS.md)

## Glosario y guía de migración

* [Glosario](../../glossary/es/GLOSSARY.md)
* [Guía de migración: si viene de EAGLE o de KiCad](../../migration/es/MIGRATION.md)

* [English](../README.md)
