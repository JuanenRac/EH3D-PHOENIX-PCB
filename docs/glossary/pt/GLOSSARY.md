# Glossário

Os termos do domínio tal como este programa os usa. A tabela no fim da página em inglês dá cada termo nos oito idiomas.

| Termo | Significado |
|---|---|
| **nm (nanómetro)** | A unidade de todo o comprimento no modelo, guardada como inteiro: 1 mm são 1 000 000 nm, por isso somas e comparações não têm erro de arredondamento. |
| **rede** | Todos os pads, pistas e vias que têm de estar ligados eletricamente; tem um nome como GND. |
| **classe de rede** | Um grupo de redes que partilham regras: largura de pista, afastamento e tamanho de via. O ficheiro de regras do segundo formato guarda as classes. |
| **via** | Um furo metalizado que une o cobre de duas camadas. |
| **anel anular** | O anel de cobre que fica à volta de um furo; se for demasiado fino, a broca pode rompê-lo. |
| **footprint (pegada)** | Os pads de cobre, furos e desenhos de um componente na placa. |
| **área de exclusão (courtyard)** | A zona à volta de um footprint onde nenhuma outra peça pode entrar. |
| **zona de cobre** | Uma área de cobre preenchida, normalmente um plano de massa, que mantém o afastamento das outras redes. |
| **empilhamento** | A ordem, a espessura e o material das camadas de cobre e isolante de uma placa; a impedância de uma linha depende dele. |
| **DRC (verificação das regras de projeto)** | A verificação de uma placa face a limites como a pista mais estreita, o afastamento, o furo e o anel anular. Comando: `phoenix check`. |
| **ERC (verificação das regras elétricas)** | A verificação de um esquema: pinos em nenhuma rede, entradas que ninguém conduz, alimentações sem fonte, etc. Comando: `phoenix erc`. |
| **linhas por ligar (ratsnest)** | As linhas retas que mostram que ligações ainda faltam encaminhar. Comando: `phoenix ratsnest`. |
| **Gerber X2** | O formato dos ficheiros de cobre, máscara, pasta e serigrafia que o fabricante lê; o X2 acrescenta atributos como a função da camada. |
| **Excellon** | O formato dos ficheiros de furação: as ferramentas e as posições dos furos, metalizados ou não. |
| **DSN / SES** | Os ficheiros Specctra para um autorouter externo: o DSN é o projeto que sai, a SES a sessão com as rotas que volta. Comando: `phoenix autoroute`. |
| **corte em V (V-score)** | Um sulco em linha reta entre duas placas de um painel, para as poder separar partindo-as. |
| **painel** | Várias placas dispostas juntas numa folha, com moldura, pontes, marcas fiduciais e furos de ferramenta, para serem fabricadas e montadas de uma vez. Comando: `phoenix panel`. |
| **pontes com furos (mouse bites)** | Uma ponte entre duas placas atravessada por uma fila de furos pequenos, para que parta de forma limpa. |
| **marca fiducial** | Uma pequena marca redonda de cobre nu que a máquina de montagem vê para encontrar a posição da placa. |
| **posicionamento de componentes (pick and place)** | O ficheiro com a referência, posição, rotação e face de cada peça, para a máquina de montagem. Comando: `phoenix pnp`. |
| **estêncil** | A folha fina com aberturas por onde se espalha a pasta de soldar sobre os pads. Comando: `phoenix stencil`. |
| **BOM (lista de materiais)** | A lista das peças de uma placa, agrupadas por valor e footprint, com quantidades e preços. Comando: `phoenix bom`. |
| **par diferencial** | Duas pistas que levam um sinal como a diferença das suas tensões; têm de manter uma separação, comprimentos iguais e simetria. Comando: `phoenix pairs`. |
| **diafonia (NEXT, FEXT)** | A parte de um sinal que aparece numa pista vizinha; a próxima (NEXT) vê-se no emissor, a distante (FEXT) na outra ponta. Comando: `phoenix si crosstalk`. |
| **parâmetros S / Touchstone** | A reflexão e a transmissão de uma rede em cada frequência; o Touchstone (.s1p, .s2p) é o ficheiro que as guarda. Comando: `phoenix rf touchstone`. |
| **ROE (razão de onda estacionária)** | Um número a partir de 1 que diz quão mal está adaptada uma carga a uma linha: 1 é uma adaptação perfeita. |
| **Monte Carlo** | Muitas execuções de um circuito em que os valores variam ao acaso dentro das suas tolerâncias; uma semente fixa dá sempre a mesma tabela. |
| **netlist (lista de ligações)** | O texto que lista os elementos de um circuito e os nós que ligam; é o que o simulador lê. Comando: `phoenix netlist`. |
| **simulacro** | Uma execução que faz tudo exceto escrever: mostra o que seria escrito ou alterado. Quase todos os comandos são um simulacro a menos que `--out` indique um ficheiro novo. |

* [English](../../GLOSSARY.md)
