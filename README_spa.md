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
  <img src="https://img.shields.io/badge/Tests-223-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Comprobación de honestidad - lo que se ejecuta hoy:** un espacio de trabajo de Rust cuyo comando `phoenix inventory` cuenta los archivos de diseño de una carpeta privada sin mostrar ningún nombre ni valor: archivos por extensión, tipo y versión declarada, y una huella del contenido. Tiene 220 pruebas propias y pasan en un ordenador. Solo se ha ejecutado con carpetas creadas por sus propias pruebas, nunca con archivos de diseño reales. También tiene los cimientos de un modelo de diseño (valores enteros en nanómetros y micrograos con rango comprobado, ids de objeto y JSON canónico), el archivo de proyecto `.phoenixproj` (manifiesto, dominios, migraciones) y un modelo de placa con operaciones reversibles (capas, redes, componentes, pistas, vías, zonas), todo probado solo con datos generados. La biblioteca de placa también halla qué cobre toca a cuál y comprueba tamaños y holguras con un archivo de reglas; una quinta biblioteca lee librerías, placas y esquemáticos EAGLE XML en un resumen y reescribe el árbol XML de forma idéntica. Todo se ha probado solo con archivos generados o hechos a mano. También existen un modelo de esquemático y su comprobación de coherencia con la placa. No hay importación de diseños ajenos al modelo, ni editor, y las reglas cubren solo parte de las comprobaciones previstas: la mayor parte de la función de EDA sigue planificada. El modelo de la placa también guarda texto y trazos de serigrafía (con una fuente de trazos incorporada) que usan la comprobación de reglas y la exportación Gerber, y el esquemático puede dibujarse en hojas con piezas colocadas y cables que se comparan con la lista de redes y con la placa; una placa puede escribirse como fichero XML de EAGLE y leerse de nuevo, lo cual solo se ha comprobado con ficheros escritos por este código y nunca se ha abierto en EAGLE; el contrato para pares de ficheros binarios de EAGLE y su XML está escrito, pero no existe ningún lector de los ficheros binarios.

---

> **Estado de seguridad (safety) y de seguridad informática:** este repositorio está en sus primeros pasos y no es un sistema de seguridad (safety). Nada de lo escrito aquí sustituye los límites, los enclavamientos y la parada de emergencia de la máquina y de su controlador.

## 📋 De un vistazo

| | |
|---|---|
| Versión | `0.0.1` |
| Madurez | Andamiaje |
| Pruebas | 223 |
| Pila tecnológica | Rust, Python 3.10+ |
| En hardware real | Aún no se ha ejecutado en hardware |

**Ir a:** [Resumen](#-resumen) · [Arquitectura](#-arquitectura) · [Inicio rápido](#-inicio-rápido) · [Pruebas](#-pruebas) · [Estructura del repositorio](#-estructura-del-repositorio) · [Proyectos relacionados](#-proyectos-relacionados)

## 🚦 Estado del proyecto

Cada línea es una parte de este repositorio y la luz dice hasta dónde llega hoy. Se escribe con lo que el repositorio puede demostrar (pruebas, compilaciones, ejecuciones), no con lo previsto.

🟢 Hecho y verificado · 🟡 Funciona, con límites conocidos · 🟠 Hecho a medias o sin verificar · 🔴 Sin hacer o sin funcionar

| Área | Estado | Qué significa aquí |
|---|---|---|
| Comprobaciones del repositorio | 🟡 | La comprobación del repositorio, la de las imágenes y las 3 pruebas de la base se ejecutan en el CI y pasan. |
| Pruebas | 🟡 | Las 220 pruebas del código Rust y las 3 de la base pasan en un ordenador. Usan solo carpetas creadas por las propias pruebas; ninguna se ha ejecutado con archivos de diseño reales. |
| Hardware real | 🟠 | No se ha ejecutado en ninguna máquina ni placa: nada de aquí se ha probado en hardware. |
| Función planificada | 🟠 | Escrito: el inventario redactado de un corpus, los cimientos del modelo (unidades, ids, JSON canónico), el archivo de proyecto `.phoenixproj`, el modelo de placa con deshacer y rehacer, su conectividad y una primera comprobación de reglas, y un lector de resumen de EAGLE XML. Sin empezar: el editor de esquemáticos, la importación de otros formatos al modelo, los escritores de otros formatos, la mayor parte de las reglas, el visor y los editores, la simulación y todo lo demás planificado. El modelo de la placa también guarda texto y trazos de serigrafía (con una fuente de trazos incorporada) que usan la comprobación de reglas y la exportación Gerber, y el esquemático puede dibujarse en hojas con piezas colocadas y cables que se comparan con la lista de redes y con la placa; una placa puede escribirse como fichero XML de EAGLE y leerse de nuevo, lo cual solo se ha comprobado con ficheros escritos por este código y nunca se ha abierto en EAGLE; el contrato para pares de ficheros binarios de EAGLE y su XML está escrito, pero no existe ningún lector de los ficheros binarios. |
| Requisitos | 🟠 | 5 de 30 requisitos (el modo redactado, los cimientos del modelo, las operaciones reversibles, el archivo de proyecto y los códigos de diagnóstico) están en prototipo, 25 solo están planificados y ninguno está validado. |

## 🎯 Resumen

* **Lo que existe:** un espacio de trabajo con una biblioteca que cuenta un corpus privado de archivos de diseño en modo redactado (solo recuentos, tipos, versiones declaradas y una huella, nunca un nombre ni un valor) y una línea de comandos, `phoenix inventory`. Mira los primeros bytes de un archivo y no lee diseños. Una segunda biblioteca guarda los cimientos del modelo de diseño: longitudes y ángulos como enteros con rango comprobado, ids de objeto (UUID v7) y JSON canónico. Una tercera biblioteca lee y escribe el proyecto `.phoenixproj` sobre el contenedor compartido de EH3D-COMMON: el manifiesto con sus dominios, enlaces y migraciones, los dominios que no entiende conservados intactos y la regla `must_understand`. Una cuarta biblioteca guarda el modelo de placa: capas, redes, huellas, componentes, pistas con líneas y arcos, vías y zonas unidas por ids estables, con operaciones reversibles y un historial de deshacer y rehacer. Cada fallo tiene un código estable `PHOENIX-<ÁREA>-<NNN>`. La biblioteca de placa también calcula las islas de cada red y comprueba anchos, taladros, anillos y holguras con un archivo de reglas en milímetros, separando lo confirmado de lo no concluyente. Una quinta biblioteca lee librerías, placas y esquemáticos EAGLE XML en un resumen, reescribe el árbol XML de forma idéntica y mide la tasa de lectura sobre copias de un corpus comprobadas con SHA-256, sin cambiar los originales; el comando `phoenix eagle-rate` lo ejecuta. Una sexta biblioteca escribe los archivos Gerber X2, Excellon y `.eh3dproj` 1.0.0 de una placa con EH3D-CORE-LIBS y EH3D-COMMON, los mismos bytes para la misma placa; los archivos solo se releen con los analizadores de la misma biblioteca. Una séptima biblioteca guarda el esquemático (símbolos, piezas y redes unidos por ids estables) y lo comprueba contra una placa: piezas sin componente y diferencias de designadores, valores, huellas y redes, con los códigos `PHOENIX-LINK-<NNN>`.

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

El código tiene 220 pruebas (`cargo test`) y la base tiene 3 (`python -I -m unittest discover -s tools -p "test_*.py"`); todas pasan en un ordenador. El CI ejecuta la comprobación del repositorio, la de las imágenes, las pruebas de la base y, para el código Rust, la comprobación de formato, clippy con las advertencias como errores y las pruebas en Linux y Windows.

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
│   ├── GATE_F1.md                               qué comprueba la puerta de la primera fase y qué no
│   └── adr/                                     los registros de decisiones aceptadas
├── AGENTS.md                                    reglas para quien cambia el repositorio
├── deny.toml                                    política de licencias y avisos de las dependencias
├── crates/                                      inventario, modelo, proyecto, placa, esquemático, EAGLE, hub, exportación, comandos
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
* La interfaz de usuario y la biblioteca de geometría siguen siendo decisiones abiertas (véase `docs/STATUS.md`).

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
