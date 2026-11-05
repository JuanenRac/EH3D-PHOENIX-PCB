<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-PCB
<p align="center"><a href="README.md">🇺🇸 English</a> | 🇪🇸 <b>Español</b> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D-PHOENIX-PCB: planificado como un entorno EDA con captura de esquemas, diseño de PCB, herramientas de verificación, simulación de circuitos y módulos especializados para fuentes conmutadas y RF. Hasta ahora solo está escrito el inventario del corpus.

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20Python-blue.svg" alt="Rust and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-431-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Comprobación de honestidad - lo que se ejecuta hoy:** un espacio de trabajo de Rust cuyo comando `phoenix inventory` cuenta los archivos de diseño de una carpeta privada sin mostrar ningún nombre ni valor: archivos por extensión, tipo y versión declarada, y una huella del contenido. Tiene 952 pruebas propias y pasan en un ordenador. Solo se ha ejecutado con carpetas creadas por sus propias pruebas, nunca con archivos de diseño reales. También tiene los cimientos de un modelo de diseño (valores enteros en nanómetros y micrograos con rango comprobado, ids de objeto y JSON canónico), el archivo de proyecto `.phoenixproj` (manifiesto, dominios, migraciones) y un modelo de placa con operaciones reversibles (capas, redes, componentes, pistas, vías, zonas), todo probado solo con datos generados. La biblioteca de placa también halla qué cobre toca a cuál y comprueba tamaños y holguras con un archivo de reglas; una quinta biblioteca lee librerías, placas y esquemáticos EAGLE XML en un resumen y reescribe el árbol XML de forma idéntica. Todo se ha probado solo con archivos generados o hechos a mano. También existen un modelo de esquemático y su comprobación de coherencia con la placa. No hay importación de diseños ajenos al modelo, ni editor, y las reglas cubren solo parte de las comprobaciones previstas: la mayor parte de la función de EDA sigue planificada. El modelo de la placa también guarda texto y trazos de serigrafía (con una fuente de trazos incorporada) que usan la comprobación de reglas y la exportación Gerber, y el esquemático puede dibujarse en hojas con piezas colocadas y cables que se comparan con la lista de redes y con la placa; una placa puede escribirse como fichero XML de EAGLE y leerse de nuevo, lo cual solo se ha comprobado con ficheros escritos por este código y nunca se ha abierto en EAGLE; el contrato para pares de ficheros binarios de EAGLE y su XML está escrito, pero no existe ningún lector de los ficheros binarios. Una calculadora de apilado y líneas de transmisión (`phoenix stackup`: impedancia, permitividad efectiva, pérdidas, retardo y ancho de pista de microstrip, stripline, guía coplanar y pares acoplados con fórmulas publicadas) es un prototipo, nunca comparado con un simulador de campo ni con una medición. Una biblioteca de conectividad y reglas (`eh3d-phoenix-rules`) lee los límites de un fichero de reglas o de las reglas de diseño de una placa EAGLE, comprueba una placa en un orden fijo (ancho, vía, taladro, anillo, separación, borde, separación entre taladros, máscara de soldadura, zonas, serigrafía), encuentra cortocircuitos y redes sin rutar y compara las redes de una placa con las de un esquema, y los comandos `phoenix check` y `phoenix connectivity` imprimen el informe con los hallazgos confirmados y los no concluyentes por separado; solo se ha probado con placas hechas a mano y nunca se ha comparado con la herramienta de referencia. La exportación de fabricación (`phoenix export`) genera archivos Gerber X2 de cada capa, archivos de taladro Excellon metalizados y sin metalizar, un archivo de coordenadas de montaje y una lista de materiales con perfiles leídos de un archivo JSON, un manifiesto con sumas SHA-256 y una comprobación que relee los archivos y los compara con la placa; es un prototipo cuyos archivos solo han sido leídos por los analizadores de este repositorio, nunca abiertos en un visor independiente ni por un fabricante. Un conjunto de herramientas de RF (`eh3d-phoenix-rf`, comando `phoenix rf`) lee y escribe archivos Touchstone, convierte parámetros S, Z, Y y ABCD, calcula ROE, pérdidas, retardo de grupo y estabilidad, diseña redes en L, adaptadores de stub simple y secciones de cuarto de onda para impedancias complejas, da las coordenadas de la carta de Smith y diseña filtros en escalera de Butterworth, Chebyshev y Bessel con componentes ideales; es un prototipo comprobado solo con valores de libro, nunca comparado con un analizador de redes, un simulador de campo ni otro programa de RF. Un lector de KiCad (`eh3d-phoenix-kicad`, comando `phoenix kicad-read`) lee las bibliotecas de símbolos, las huellas, los esquemas con sus hojas, las placas y los archivos de proyecto de KiCad 6 a 9 hacia el mismo modelo que los archivos de EAGLE, con un informe de importación y un nivel de fidelidad; es un prototipo probado solo con archivos escritos a mano para sus pruebas, nunca comparado con KiCad ni ejecutado sobre una colección pública de diseños. Un saneador de bibliotecas (`eh3d-phoenix-sanitizer`, comando `phoenix sanitize`) lee una carpeta de bibliotecas EAGLE antiguas e informa de las huellas, los símbolos y los dispositivos duplicados o casi duplicados, con un hash geométrico canónico, una puntuación frente a un umbral, un informe en HTML y en JSON y un plan de biblioteca maestra; solo analiza, nunca reescribe una biblioteca ni aplica el plan, y es un prototipo probado solo con bibliotecas hechas por sus propias pruebas, nunca con bibliotecas reales. Una comprobación eléctrica (`eh3d-phoenix-erc`, comando `phoenix erc`) juzga un esquema: pines que no están en ninguna red, redes con un solo pin, tipos de pin que una matriz de un fichero de ajustes no permite juntos, entradas sin emisor, alimentaciones sin fuente, nombres de red parecidos, referencias repetidas, valores vacíos, piezas sin huella y cables sueltos, con los hallazgos confirmados y los no concluyentes por separado; una sincronización (`eh3d-phoenix-sync`, comando `phoenix sync`) compara un esquema y una placa por la identidad de sus piezas y hace los cambios de la transferencia a la placa, o de la anotación de vuelta al esquema, como un paquete que se prueba antes en seco y que se puede deshacer y rehacer; el fichero de reglas del segundo formato guarda además clases de red con límites propios para pistas y vías. Todo es un prototipo probado solo con esquemas y placas hechos a mano y por las pruebas, nunca comparado con la herramienta de referencia, y sigue sin haber editor de esquemas. Un simulador de circuitos (`eh3d-phoenix-sim`, comando `phoenix sim`) lee y escribe netlists al estilo SPICE, traduce un esquema a una netlist y resuelve circuitos lineales y con diodos por un análisis nodal modificado propio (punto de operación en continua, barridos de alterna con retardo de grupo y análisis transitorio), con una comprobación frente a los filtros en escalera de las herramientas RF; es un prototipo probado con valores de libro, nunca comparado con SPICE ni con una medición.

---

> **Estado de seguridad (safety) y de seguridad informática:** este repositorio está en sus primeros pasos y no es un sistema de seguridad (safety). Nada de lo escrito aquí sustituye los límites, los enclavamientos y la parada de emergencia de la máquina y de su controlador.

## 📋 De un vistazo

| | |
|---|---|
| Versión | `0.0.2` |
| Madurez | Andamiaje |
| Pruebas | 431 |
| Pila tecnológica | Rust, Python 3.10+ |
| En hardware real | Aún no se ha ejecutado en hardware |

**Ir a:** [Resumen](#-resumen) · [Arquitectura](#-arquitectura) · [Inicio rápido](#-inicio-rápido) · [Pruebas](#-pruebas) · [Estructura del repositorio](#-estructura-del-repositorio) · [Proyectos relacionados](#-proyectos-relacionados)

## 🚦 Estado del proyecto

Cada línea es una parte de este repositorio y la luz dice hasta dónde llega hoy. Se escribe con lo que el repositorio puede demostrar (pruebas, compilaciones, ejecuciones), no con lo previsto.

🟢 Hecho y verificado · 🟡 Funciona, con límites conocidos · 🟠 Hecho a medias o sin verificar · 🔴 Sin hacer o sin funcionar

| Área | Estado | Qué significa aquí |
|---|---|---|
| Comprobaciones del repositorio | 🟡 | La comprobación del repositorio, la de las imágenes y las 3 pruebas de la base se ejecutan en el CI y pasan. |
| Pruebas | 🟡 | Las 952 pruebas del código Rust y las 3 de la base pasan en un ordenador. Usan solo carpetas creadas por las propias pruebas; ninguna se ha ejecutado con archivos de diseño reales. |
| Hardware real | 🟠 | No se ha ejecutado en ninguna máquina ni placa: nada de aquí se ha probado en hardware. |
| Función planificada | 🟠 | Escrito: el inventario redactado de un corpus, los cimientos del modelo (unidades, ids, JSON canónico), el archivo de proyecto `.phoenixproj`, el modelo de placa con deshacer y rehacer, su conectividad y una primera comprobación de reglas, y un lector de resumen de EAGLE XML. Sin empezar: el editor de esquemáticos, la importación de formatos distintos de KiCad al modelo, los escritores de otros formatos, la mayor parte de las reglas, los editores y el visor definitivo, la comparación del simulador con SPICE y todo lo demás planificado. El modelo de la placa también guarda texto y trazos de serigrafía (con una fuente de trazos incorporada) que usan la comprobación de reglas y la exportación Gerber, y el esquemático puede dibujarse en hojas con piezas colocadas y cables que se comparan con la lista de redes y con la placa; una placa puede escribirse como fichero XML de EAGLE y leerse de nuevo, lo cual solo se ha comprobado con ficheros escritos por este código y nunca se ha abierto en EAGLE; el contrato para pares de ficheros binarios de EAGLE y su XML está escrito, pero no existe ningún lector de los ficheros binarios. Una calculadora de apilado y líneas de transmisión (`phoenix stackup`: impedancia, permitividad efectiva, pérdidas, retardo y ancho de pista de microstrip, stripline, guía coplanar y pares acoplados con fórmulas publicadas) es un prototipo, nunca comparado con un simulador de campo ni con una medición. Un conjunto de herramientas de RF (`phoenix rf`: archivos Touchstone, parámetros de dos puertos, adaptación y filtros en escalera ideales) es un prototipo, comprobado solo con valores de libro. Un lector de KiCad (`phoenix kicad-read`: símbolos, huellas, esquemas con hojas, placas y el archivo de proyecto) es un prototipo, probado solo con archivos escritos a mano. Un saneador de bibliotecas (`phoenix sanitize`: inventario de una carpeta de bibliotecas EAGLE, hash geométrico, coincidencia aproximada con umbral, grupos de duplicados, informe HTML y JSON y un plan de biblioteca maestra que nunca se aplica) es un prototipo, probado solo con bibliotecas hechas por sus pruebas. Una comprobación eléctrica (`phoenix erc`) y la sincronización de un esquema y una placa (`phoenix sync`: transferencia a la placa y anotación de vuelta, como un paquete que se puede deshacer) son prototipos, probados solo con esquemas y placas hechos a mano. Un simulador de circuitos propio (`phoenix sim`: netlists al estilo SPICE, la netlist de un esquema y análisis de continua, de alterna y transitorio de circuitos lineales y con diodos) es un prototipo, comprobado solo con valores de libro y nunca comparado con SPICE. |
| Ventana | 🟠 | El visor de placas y esquemas de solo lectura (`phoenix view`, egui, tras la característica `gui`) está escrito y su lógica está probada, pero la ventana no se ha visto nunca en una pantalla: el dibujo, el ratón y la fluidez en pantalla no están comprobados. |
| Requisitos | 🟠 | 15 de 30 requisitos (el modo redactado, los cimientos del modelo, las operaciones reversibles, el archivo de proyecto, los códigos de diagnóstico, las calculadoras RF analíticas, las comprobaciones de conectividad y de reglas y la exportación de fabricación con su paquete, el visor de placas y esquemas los lectores de KiCad, el saneador de bibliotecas, las comprobaciones y la sincronización del esquema y el simulador de circuitos) están en prototipo, 15 solo están planificados y ninguno está validado. |

## 🎯 Resumen

* **Lo que existe:** un espacio de trabajo con una biblioteca que cuenta un corpus privado de archivos de diseño en modo redactado (solo recuentos, tipos, versiones declaradas y una huella, nunca un nombre ni un valor) y una línea de comandos, `phoenix inventory`. Mira los primeros bytes de un archivo y no lee diseños. Una segunda biblioteca guarda los cimientos del modelo de diseño: longitudes y ángulos como enteros con rango comprobado, ids de objeto (UUID v7) y JSON canónico. Una tercera biblioteca lee y escribe el proyecto `.phoenixproj` sobre el contenedor compartido de EH3D-COMMON: el manifiesto con sus dominios, enlaces y migraciones, los dominios que no entiende conservados intactos y la regla `must_understand`. Una cuarta biblioteca guarda el modelo de placa: capas, redes, huellas, componentes, pistas con líneas y arcos, vías y zonas unidas por ids estables, con operaciones reversibles y un historial de deshacer y rehacer. Cada fallo tiene un código estable `PHOENIX-<ÁREA>-<NNN>`. La biblioteca de placa también calcula las islas de cada red y comprueba anchos, taladros, anillos y holguras con un archivo de reglas en milímetros, separando lo confirmado de lo no concluyente. Una quinta biblioteca lee librerías, placas y esquemáticos EAGLE XML en un resumen, reescribe el árbol XML de forma idéntica y mide la tasa de lectura sobre copias de un corpus comprobadas con SHA-256, sin cambiar los originales; el comando `phoenix eagle-rate` lo ejecuta. Una sexta biblioteca escribe los archivos Gerber X2, Excellon y `.eh3dproj` 1.0.0 de una placa con EH3D-CORE-LIBS y EH3D-COMMON, los mismos bytes para la misma placa; los archivos solo se releen con los analizadores de la misma biblioteca. Una séptima biblioteca guarda el esquemático (símbolos, piezas y redes unidos por ids estables) y lo comprueba contra una placa: piezas sin componente y diferencias de designadores, valores, huellas y redes, con los códigos `PHOENIX-LINK-<NNN>`. Otra biblioteca, `eh3d-phoenix-rules`, deduce las redes del cobre (pads, pistas, vías y zonas unidas por la geometría con una tolerancia), las compara con las del esquemático y ejecuta las reglas de diseño en un orden fijo contra límites leídos de un fichero de reglas o de las reglas de diseño de una placa EAGLE, con los hallazgos confirmados y los no concluyentes en dos listas; los comandos `phoenix check` y `phoenix connectivity` imprimen su informe como texto en ocho idiomas o como JSON.

* **Planificado:** EH3D-PHOENIX-PCB pretende ser un entorno EDA con captura de esquemas, diseño de PCB, herramientas de verificación, simulación de circuitos y módulos especializados para fuentes conmutadas y RF. Se planea como un programa en Rust con ventana para el escritorio (un espacio de trabajo de crates de motor, una interfaz de línea de comandos y una aplicación) que funciona solo y puede usar el servidor para tareas de cálculo. Nada de esto existe todavía.

## 🧩 Papel en EH3D

EH3D-PHOENIX-PCB es el proyecto principal de la familia PHOENIX de la EH3D Fabrication Suite; los demás repositorios de la familia se apoyan en él. Lo que hará, lo que todavía no hará, sus requisitos y sus relaciones con otros repositorios están escritos en la carpeta `docs/`.

## 🏛️ Arquitectura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitectura de EH3D-PHOENIX-PCB: quién lo usa, qué contiene y de qué depende" width="100%">
</p>

Cómo encajan las partes de este repositorio: quién lo usa, qué contiene y de qué depende. La imagen se dibuja desde `docs/architecture.json` y se comprueba contra el código.

## 🚀 Inicio rápido

Necesitas Python 3.10 o superior para las comprobaciones y una cadena de herramientas de Rust para el código.

```bash
cargo build --release
cargo test
cargo run -p eh3d-phoenix-cli -- inventory <folder>
python -I tools/ci_validate.py
python -I tools/make_architecture.py --check
python -I tools/make_structure.py --check --readmes
python -I -m unittest discover -s tools -p "test_*.py"
```

## 🧪 Pruebas

El código tiene 952 pruebas (`cargo test`) y la base tiene 3 (`python -I -m unittest discover -s tools -p "test_*.py"`); todas pasan en un ordenador. El CI ejecuta la comprobación del repositorio, la de las imágenes, las pruebas de la base y, para el código Rust, la comprobación de formato, clippy con las advertencias como errores y las pruebas en Linux y Windows.

## 🏗️ Compilación y publicación

La versión vive en el fichero `VERSION` y en `eh3d.project.json`, y cada versión tiene su entrada en [CHANGELOG.md](CHANGELOG.md). `cargo build --release` compila la biblioteca y el programa `phoenix`; las versiones se publican en GitHub.

## 📂 Estructura del repositorio

<p align="center">
  <img src="images/repository-structure.svg" alt="Estructura del repositorio EH3D-PHOENIX-PCB" width="100%">
</p>

<details>
<summary>Versión en texto del árbol</summary>

```text
EH3D-PHOENIX-PCB/
├── docs/
│   ├── architecture.json structure.json         los datos de las dos imágenes
│   ├── SCOPE.md REQUIREMENTS.md                 su alcance y sus requisitos
│   ├── DEPENDENCIES.md THREAT_MODEL.md          relaciones con otros repositorios y modelo de amenazas
│   ├── DECISIONS.md STATUS.md                   decisiones que se aplican aquí y estado honesto
│   ├── NOT_YET.md RULES.md                      ideas para más adelante y reglas para escribir código
│   ├── ROADMAP.md                               el mapa de fases, de los cimientos a la suite todo en uno
│   ├── ERROR_CODES.md                           los códigos de diagnóstico PHOENIX-<ÁREA>-<NNN>
│   ├── STACKUP.md                               cálculos de apilado y líneas, fórmulas
│   ├── RF.md                                    herramientas de RF: Touchstone, bipuertos, adaptación, filtros
│   ├── KICAD.md                                 los lectores de KiCad: qué se lee de cada formato
│   ├── VIEWER.md                                el prototipo de visor de solo lectura y su prueba de rendimiento
│   ├── SANITIZER.md                             el saneador de bibliotecas: duplicados, hash, informe y plan
│   ├── ERC.md                                   comprobación eléctrica y sincronización
│   ├── SIM.md                                   simulación de circuitos: netlist, análisis, exportación del esquema
│   ├── GATE_F1.md                               qué comprueba la puerta de la primera fase y qué no
│   └── adr/                                     los registros de decisiones
├── AGENTS.md                                    reglas para quien cambia el repositorio
├── deny.toml                                    política de licencias y avisos de las dependencias
├── crates/                                      inventario, modelo, proyecto, placa, esquemático, EAGLE, KiCad, reglas, ERC, sincronización, hub, exportación, apilado, RF, simulador, saneador, visor, comandos
├── config/                                      el fichero de ajustes (todo valor afinado)
├── Cargo.toml Cargo.lock                        el espacio de trabajo de Rust y sus dependencias fijadas
├── tools/                                       la comprobación del repositorio, sus pruebas y las herramientas de las imágenes
├── images/                                      banner y las imágenes de Arquitectura y Estructura del repositorio
├── .github/                                     flujos de CI y del panel, plantillas de issues y pull requests
├── eh3d.project.json                            manifiesto: versión, rol, madurez y comando de compilación
├── VERSION                                      la versión del repositorio
├── CHANGELOG.md                                 qué cambió en cada versión
├── SECURITY.md SUPPORT.md                       cómo informar de una vulnerabilidad y dónde preguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   cómo contribuir, el código de conducta y la licencia
```

</details>

## 🗺️ Hoja de ruta y límites conocidos

**Límites conocidos hoy**

* Código inicial: solo existen el inventario del corpus y los cimientos del modelo; el inventario solo se ha ejecutado con carpetas creadas por sus propias pruebas, nunca con archivos de diseño reales.
* No hay esquemático, ni lector ni escritor de archivos de diseño de otros programas, ni editor, ni reglas; el archivo de proyecto y el modelo de placa solo se han probado con datos generados, nunca con un diseño real.
* La pila final de la interfaz de usuario sigue abierta (hay una elección provisional en `docs/adr/`), y también la biblioteca de geometría (véase `docs/STATUS.md`).

**Siguiente**

* Esperar a que se acepten los contratos de los que depende este proyecto (véase `docs/DEPENDENCIES.md`), escribir después el modelo y los primeros lectores con pruebas y sustituir las filas de estado por resultados reales.

## 🌍 Idiomas

El README y los textos existen en inglés, español, francés, italiano, alemán, chino, japonés y portugués de Portugal.

## 🔗 Proyectos relacionados

* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS): el mapa de la suite, su arquitectura, las convenciones y las comprobaciones compartidas
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): se construye junto a este repositorio; el contenedor de proyecto compartido (`eh3d-container`) y el diagnóstico común (`eh3d-diagnostic`) que usa este proyecto
* Repositorios hermanos de PHOENIX: [EH3D-VALIDATION-LAB](https://github.com/JuanenRac/EH3D-VALIDATION-LAB) (corpus y oráculo de archivos dorados) y [EH3D-PHOENIX-CAD](https://github.com/JuanenRac/EH3D-PHOENIX-CAD) (lado mecánico), ambos en código inicial
* Relaciones planificadas, aún sin construir (EH3D-CORE-LIBS, EH3D-SERVER, EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE, EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER, EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-SUITE-LAUNCHER, EH3D-HYDRA-BRIDGE): véase [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md)

## 📚 Documentación

* [Las convenciones de la suite](https://github.com/JuanenRac/EH3D-DOCS/blob/main/docs/CONVENTIONS.md)
* [Registro de cambios](CHANGELOG.md)
* [Alcance, requisitos, dependencias, modelo de amenazas y decisiones](docs/SCOPE.md)

## 🤝 Contribuir y comunidad

* Antes de una pull request ejecuta `python -I tools/ci_validate.py` y las pruebas de la base: todo debe pasar. Consulta [CONTRIBUTING.md](CONTRIBUTING.md) y el [código de conducta](CODE_OF_CONDUCT.md).
* El README existe en ocho idiomas: un cambio en el texto se hace en los ocho archivos, y las dos imágenes se dibujan a partir de `docs/architecture.json` y `docs/structure.json` con `tools/make_architecture.py` y `tools/make_structure.py`.
* Informa de una vulnerabilidad en privado: [SECURITY.md](SECURITY.md). Preguntas, ideas e informes: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCIA

Software: GPL-3.0-or-later. Véase [LICENSE](LICENSE). Los diseños de hardware de la suite usan CERN OHL-S.
