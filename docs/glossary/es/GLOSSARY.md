# Glosario

Los términos del dominio tal como los usa este programa. La tabla al final de la página en inglés da cada término en los ocho idiomas.

| Término | Significado |
|---|---|
| **nm (nanómetro)** | La unidad de toda longitud en el modelo, guardada como entero: 1 mm son 1 000 000 nm, así que sumas y comparaciones no tienen error de redondeo. |
| **red (eléctrica)** | Todos los pads, pistas y vías que deben estar conectados eléctricamente; tiene un nombre como GND. |
| **clase de red** | Un grupo de redes que comparten reglas: ancho de pista, separación y tamaño de vía. El fichero de reglas del segundo formato guarda las clases. |
| **vía** | Un taladro metalizado que une el cobre de dos capas. |
| **anillo anular** | El anillo de cobre que queda alrededor de un taladro; si es demasiado fino, la broca puede romperlo. |
| **huella** | Los pads de cobre, taladros y dibujos de un componente en la placa. |
| **área de cortesía (courtyard)** | La zona alrededor de una huella en la que no puede entrar otra pieza. |
| **zona de cobre** | Un área de cobre rellena, normalmente un plano de masa, que guarda su separación con las demás redes. |
| **apilado (stack-up)** | El orden, el espesor y el material de las capas de cobre y aislante de una placa; de él depende la impedancia de una línea. |
| **DRC (comprobación de reglas de diseño)** | La comprobación de una placa frente a límites como la pista más estrecha, la separación, el taladro y el anillo anular. Comando: `phoenix check`. |
| **ERC (comprobación de reglas eléctricas)** | La comprobación de un esquema: pines sin red, entradas que nadie maneja, alimentaciones sin fuente, etc. Comando: `phoenix erc`. |
| **líneas de conexión pendientes (ratsnest)** | Las líneas rectas que muestran qué conexiones faltan por rutar. Comando: `phoenix ratsnest`. |
| **Gerber X2** | El formato de los ficheros de cobre, máscara, pasta y serigrafía que lee el fabricante; X2 añade atributos como la función de la capa. |
| **Excellon** | El formato de los ficheros de taladros: las herramientas y las posiciones de los agujeros, metalizados o no. |
| **DSN / SES** | Los ficheros Specctra para un autorouter externo: DSN es el diseño que sale, SES la sesión con las rutas que vuelve. Comando: `phoenix autoroute`. |
| **corte en V (V-score)** | Una ranura en línea recta entre dos placas de un panel, para poder separarlas rompiéndolas. |
| **panel** | Varias placas dispuestas juntas en una hoja, con marco, puentes, fiduciales y taladros de herramienta, para fabricarlas y montarlas de una vez. Comando: `phoenix panel`. |
| **puentes con taladros (mouse bites)** | Un puente entre dos placas atravesado por una fila de agujeros pequeños, para que se rompa limpiamente. |
| **marca fiducial** | Una pequeña marca redonda de cobre desnudo que la máquina de montaje ve para hallar la posición de la placa. |
| **posicionamiento de componentes (pick and place)** | El fichero con la referencia, posición, rotación y cara de cada pieza, para la máquina de montaje. Comando: `phoenix pnp`. |
| **plantilla de pasta (stencil)** | La lámina fina con aberturas por las que se extiende la pasta de soldar sobre los pads. Comando: `phoenix stencil`. |
| **BOM (lista de materiales)** | La lista de las piezas de una placa, agrupadas por valor y huella, con cantidades y precios. Comando: `phoenix bom`. |
| **par diferencial** | Dos pistas que llevan una señal como diferencia de sus tensiones; deben mantener una separación, longitudes iguales y simetría. Comando: `phoenix pairs`. |
| **diafonía (NEXT, FEXT)** | La parte de una señal que aparece en una pista vecina; la cercana (NEXT) se ve en el emisor y la lejana (FEXT) en el otro extremo. Comando: `phoenix si crosstalk`. |
| **parámetros S / Touchstone** | La reflexión y la transmisión de una red en cada frecuencia; Touchstone (.s1p, .s2p) es el fichero que las guarda. Comando: `phoenix rf touchstone`. |
| **ROE (relación de onda estacionaria)** | Un número desde 1 que dice cuán mal está adaptada una carga a una línea: 1 es una adaptación perfecta. |
| **Monte Carlo** | Muchas ejecuciones de un circuito en las que los valores varían al azar dentro de sus tolerancias; una semilla fija da siempre la misma tabla. |
| **netlist (lista de conexiones)** | El texto que lista los elementos de un circuito y los nodos que unen; es lo que lee el simulador. Comando: `phoenix netlist`. |
| **simulacro** | Una ejecución que hace todo salvo escribir: muestra lo que se escribiría o cambiaría. Casi todos los comandos son un simulacro salvo que `--out` nombre un fichero nuevo. |

* [English](../../GLOSSARY.md)
