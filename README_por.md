<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-PCB
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | 🇵🇹 <b>Português</b></p>

### EH3D-PHOENIX-PCB: planeado como um ambiente EDA com captura de esquemas, desenho de PCB, ferramentas de verificação, simulação de circuitos e módulos especializados para fontes comutadas e RF. Até agora só está escrito o inventário do corpus.

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20Python-blue.svg" alt="Rust and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-431-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** um espaço de trabalho Rust cujo comando `phoenix inventory` conta os ficheiros de projeto de uma pasta privada sem mostrar nenhum nome nem valor: ficheiros por extensão, tipo e versão declarada, e uma impressão digital do conteúdo. Tem 2123 testes próprios, que passam num computador. Só foi executado com pastas criadas pelos seus próprios testes, nunca com ficheiros de projeto reais. Tem também os alicerces de um modelo de projeto (valores inteiros em nanómetros e micrograus com intervalo verificado, ids de objeto e JSON canónico), o ficheiro de projeto `.phoenixproj` (manifesto, domínios, migrações) e um modelo de placa com operações reversíveis (camadas, redes, componentes, pistas, vias, zonas), tudo experimentado apenas com dados gerados. A biblioteca de placa também descobre que cobre toca em que cobre e verifica tamanhos e distâncias com um ficheiro de regras; uma quinta biblioteca lê bibliotecas, placas e esquemas EAGLE XML para um resumo e volta a escrever a árvore XML de forma idêntica. Tudo foi experimentado apenas com ficheiros gerados ou feitos à mão. Existem também um modelo de esquema e a sua verificação de coerência com a placa. Não há importação de projetos alheios para o modelo, nem editor, e as regras cobrem só parte das verificações previstas: a maior parte da função de EDA continua planeada. O modelo da placa também guarda texto e traços de serigrafia (com um tipo de letra de traços incorporado) que a verificação de regras e a exportação Gerber usam, e o esquema pode ser desenhado em folhas com peças colocadas e fios que são comparados com a lista de redes e com a placa; uma placa pode ser escrita como ficheiro XML do EAGLE e lida de novo, o que só foi verificado com ficheiros escritos por este código e nunca foi aberto no EAGLE; o contrato para pares de ficheiros binários do EAGLE e do seu XML está escrito, mas não existe nenhum leitor dos ficheiros binários. Uma calculadora de empilhamento e linhas de transmissão (`phoenix stackup`: impedância, permitividade efetiva, perdas, atraso e largura da pista de microstrip, stripline, guia coplanar e pares acoplados com fórmulas publicadas) é um protótipo, nunca comparado com um simulador de campo nem com uma medição. Com a base de dados de materiais do EH3D-COMMON (`phoenix materials`, `phoenix stackup --material ID`), o Dk e o Df de uma camada vêm dos pontos que uma folha de dados publica, nunca extrapolados; nenhum destes valores foi comparado com a medição de uma placa. Uma biblioteca de conectividade e regras (`eh3d-phoenix-rules`) lê os limites de um ficheiro de regras ou das regras de projeto de uma placa EAGLE, verifica uma placa por uma ordem fixa (largura, via, furo, anel, distância, limite da placa, distância entre furos, máscara de soldadura, zonas, serigrafia), encontra curto-circuitos e redes por encaminhar e compara as redes de uma placa com as de um esquema, e os comandos `phoenix check` e `phoenix connectivity` imprimem o relatório com os resultados confirmados e os inconclusivos em separado; só foi experimentada com placas feitas à mão e nunca foi comparada com a ferramenta de referência. A exportação de fabrico (`phoenix export`) gera ficheiros Gerber X2 de cada camada, ficheiros de furação Excellon metalizados e não metalizados, um ficheiro de coordenadas de montagem e uma lista de materiais com perfis lidos de um ficheiro JSON, um manifesto com somas SHA-256 e uma verificação que relê os ficheiros e os compara com a placa; é um protótipo cujos ficheiros só foram lidos pelos analisadores deste repositório, nunca abertos num visualizador independente nem por um fabricante. Um conjunto de ferramentas de RF (`eh3d-phoenix-rf`, comando `phoenix rf`) lê e escreve ficheiros Touchstone, converte parâmetros S, Z, Y e ABCD, calcula ROE, perdas, atraso de grupo e estabilidade, projeta redes em L, adaptadores de stub simples e secções de quarto de onda para impedâncias complexas, dá as coordenadas da carta de Smith e projeta filtros em escada de Butterworth, Chebyshev e Bessel com componentes ideais; é um protótipo verificado apenas com valores de manual, nunca comparado com um analisador de redes, um simulador de campo nem outro programa de RF. Um leitor de KiCad (`eh3d-phoenix-kicad`, comando `phoenix kicad-read`) lê as bibliotecas de símbolos, as pegadas, os esquemas com as suas folhas, as placas e os ficheiros de projeto do KiCad 6 a 9 para o mesmo modelo dos ficheiros EAGLE, com um relatório de importação e um nível de fidelidade; é um protótipo experimentado apenas com ficheiros escritos à mão para os seus testes, nunca comparado com o KiCad nem executado numa coleção pública de projetos. Um saneador de bibliotecas (`eh3d-phoenix-sanitizer`, comando `phoenix sanitize`) lê uma pasta de bibliotecas EAGLE antigas e comunica as pegadas, os símbolos e os dispositivos duplicados ou quase duplicados, com um hash geométrico canónico, uma pontuação face a um limiar, um relatório HTML e JSON e um plano de biblioteca mestre; apenas analisa, nunca reescreve uma biblioteca nem aplica o plano, e é um protótipo experimentado apenas com bibliotecas feitas pelos seus próprios testes, nunca com bibliotecas reais. Uma verificação elétrica (`eh3d-phoenix-erc`, comando `phoenix erc`) avalia um esquema: pinos em nenhuma rede, redes com um só pino, tipos de pino que uma matriz de um ficheiro de definições não permite em conjunto, entradas sem emissor, alimentações sem fonte, nomes de rede parecidos, referências repetidas, valores vazios, peças sem pegada e fios soltos, com os resultados confirmados e os inconclusivos em separado; uma sincronização (`eh3d-phoenix-sync`, comando `phoenix sync`) compara um esquema e uma placa pela identidade das suas peças e faz as alterações da transferência para a placa, ou da anotação de retorno para o esquema, como um pacote experimentado primeiro em seco e que se pode desfazer e refazer; o ficheiro de regras do segundo formato guarda ainda classes de rede com limites próprios para pistas e vias. Tudo é um protótipo experimentado apenas com esquemas e placas feitos à mão e pelos testes, nunca comparado com a ferramenta de referência, e o editor de esquemas (o comando `phoenix schedit` e um modo de edição da janela) é também um protótipo, experimentado apenas sem ecrã: operações que se podem anular, uma lista de redes que acompanha o desenho, a verificação das regras após cada alteração e a transferência para a placa. Um simulador de circuitos (`eh3d-phoenix-sim`, comando `phoenix sim`) lê e escreve netlists no estilo SPICE, traduz um esquema numa netlist e resolve circuitos lineares, com díodos (com resistência série, capacidade de junção e disrupção), com transístores bipolares e MOS, com interruptores controlados por tensão e com macromodelos de amplificador operacional por uma análise nodal modificada própria (ponto de funcionamento em contínua e varrimento em contínua de uma fonte, varrimentos em alternada com atraso de grupo, ruído e análise transitória com um passo que para nos cantos das fontes), com uma verificação face aos filtros em escada das ferramentas RF; lê também as linhas `.op`, `.dc`, `.tran` e `.ac` de uma netlist e o `.include` de ficheiros da mesma pasta; é um protótipo testado com valores de manual e soluções fechadas, nunca comparado com o SPICE nem com uma medição, e os seus modelos de dispositivos são simples. Uma biblioteca para a lógica de edição e encaminhamento (`eh3d-phoenix-route`, comandos `phoenix route` e `phoenix ratsnest`) lista as ligações que faltam encaminhar e encaminha entre dois elementos de uma rede com troços a 0, 45 e 90 graus, parando diante de um obstáculo ou contornando-o, com a largura da classe de rede e vias dimensionadas pelas regras; as suas edições são operações reversíveis que indicam o que acrescentam à verificação de regras. É um protótipo testado com placas feitas à mão e pelos testes, sem arcos nem zonas de cobre; a janela que edita com ele (`phoenix view`) nunca foi vista num ecrã. O encaminhamento com um encaminhador automático externo faz-se por ficheiros (`phoenix autoroute`): a placa é escrita como ficheiro Specctra DSN, uma sessão Specctra é lida como um plano cujas pistas são verificadas pelas regras e aplicadas todas ou nenhuma, e um adaptador que lança um programa está desligado por defeito e não lança nada sem um interruptor num ficheiro de definições e uma opção na linha de comandos; é um protótipo experimentado apenas com sessões feitas à mão, nunca com um encaminhador automático real. Um intérprete de scripts (`eh3d-phoenix-script`, comandos `phoenix script run`, `phoenix script report` e `phoenix cam-import`) lê ficheiros de script da linguagem de comandos do fluxo de trabalho anterior, executa `WIRE`, `VIA`, `MOVE`, `CHANGE`, `DELETE`, `DRC` e os restantes comandos que têm equivalente como operações reversíveis da placa, tudo ou nada e com um anular que devolve o original byte a byte, lista cada comando de cada script como equivalente, adaptado ou pendente com a sua causa e lê um trabalho de saída como perfil de exportação; foi construído a partir da descrição pública da linguagem, é um protótipo testado só com scripts escritos para os seus testes e nunca foi comparado com a ferramenta original nem executado com os scripts próprios do autor. Um gerador de estruturas de RF (`eh3d-phoenix-rfgen`, comando `phoenix rfgen`) desenha, a partir do empilhamento e da frequência, linhas de microfita, curvas com chanfro e em arco, transições de largura, stubs, redes de adaptação, filtros passa-baixo de impedâncias escalonadas e passa-banda de linhas acopladas, um divisor de Wilkinson, um acoplador de ramos, antenas de patch e dipolo e uma transição SMA com cerca de vias, com fórmulas publicadas, e coloca-os na placa como operações reversíveis, tudo ou nada, com um relatório das dimensões, das fórmulas usadas e da verificação de regras do resultado; é um protótipo, testado só com valores de manuais e com placas feitas pelos testes, e nunca foi comparado com um simulador eletromagnético nem com uma medição. Um módulo de escrita de KiCad (`phoenix kicad-export`) escreve uma placa, um esquema, pegadas e símbolos como ficheiros KiCad 8, com um relatório do que o modelo não consegue exprimir; é um protótipo cujos ficheiros nunca foram abertos no KiCad.

---

> **Estado de segurança (safety) e de segurança informática:** este repositório está nos primeiros passos e não é um sistema de segurança (safety). Nada do que está escrito aqui substitui os limites, os bloqueios e a paragem de emergência da máquina e do seu controlador.

## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.4` |
| Maturidade | Estrutura inicial |
| Testes | 431 |
| Tecnologias | Rust, Python 3.10+ |
| Em hardware real | Ainda não executado em hardware |

**Ir para:** [Visão geral](#-visão-geral) · [Arquitetura](#-arquitetura) · [Início rápido](#-início-rápido) · [Testes](#-testes) · [Estrutura do repositório](#-estrutura-do-repositório) · [Projetos relacionados](#-projetos-relacionados)

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Verificações do repositório | 🟡 | A verificação do repositório, a das imagens e os 3 testes da base correm na CI e passam. |
| Testes | 🟡 | Os 2123 testes do código Rust e os 3 da base passam num computador. Usam apenas pastas criadas pelos próprios testes; nenhum foi executado com ficheiros de projeto reais. |
| Hardware real | 🟠 | Não foi executado em nenhuma máquina nem placa: nada aqui foi experimentado em hardware. |
| Função planeada | 🟠 | Escrito: o inventário ocultado de um corpus, os alicerces do modelo (unidades, ids, JSON canónico), o ficheiro de projeto `.phoenixproj`, o modelo de placa com anular e refazer, a sua conectividade e uma primeira verificação de regras, e um leitor de resumo de EAGLE XML. Por começar: a importação de formatos que não sejam KiCad para o modelo, os escritores de outros formatos, a maior parte das regras, os editores e o visualizador definitivo, a comparação do simulador com o SPICE e tudo o resto do planeado. O modelo da placa também guarda texto e traços de serigrafia (com um tipo de letra de traços incorporado) que a verificação de regras e a exportação Gerber usam, e o esquema pode ser desenhado em folhas com peças colocadas e fios que são comparados com a lista de redes e com a placa; uma placa pode ser escrita como ficheiro XML do EAGLE e lida de novo, o que só foi verificado com ficheiros escritos por este código e nunca foi aberto no EAGLE; o contrato para pares de ficheiros binários do EAGLE e do seu XML está escrito, mas não existe nenhum leitor dos ficheiros binários. Uma calculadora de empilhamento e linhas de transmissão (`phoenix stackup`: impedância, permitividade efetiva, perdas, atraso e largura da pista de microstrip, stripline, guia coplanar e pares acoplados com fórmulas publicadas) é um protótipo, nunca comparado com um simulador de campo nem com uma medição. Com a base de dados de materiais do EH3D-COMMON (`phoenix materials`, `phoenix stackup --material ID`), o Dk e o Df de uma camada vêm dos pontos que uma folha de dados publica, nunca extrapolados; nenhum destes valores foi comparado com a medição de uma placa. Um conjunto de ferramentas de RF (`phoenix rf`: ficheiros Touchstone, parâmetros de dois portos, adaptação e filtros em escada ideais) é um protótipo, verificado apenas com valores de manual. Um leitor de KiCad (`phoenix kicad-read`: símbolos, pegadas, esquemas com folhas, placas e o ficheiro de projeto) é um protótipo, experimentado apenas com ficheiros escritos à mão. Um saneador de bibliotecas (`phoenix sanitize`: inventário de uma pasta de bibliotecas EAGLE, hash geométrico, correspondência aproximada com limiar, grupos de duplicados, relatório HTML e JSON e um plano de biblioteca mestre que nunca é aplicado) é um protótipo, experimentado apenas com bibliotecas feitas pelos seus testes. Uma verificação elétrica (`phoenix erc`) e a sincronização de um esquema e de uma placa (`phoenix sync`: transferência para a placa e anotação de retorno, como um pacote que se pode desfazer) são protótipos, experimentados apenas com esquemas e placas feitos à mão. Um simulador de circuitos próprio (`phoenix sim`: netlists no estilo SPICE, a netlist de um esquema e análises em contínua, em alternada e transitória de circuitos lineares e com díodos) é um protótipo, verificado só com valores de manual e nunca comparado com o SPICE. A lógica de edição e encaminhamento (`phoenix route`, `phoenix ratsnest`: as ligações por fazer, um encaminhador de troços a 0, 45 e 90 graus e edições reversíveis com a verificação de regras em direto) é um protótipo, testado só com placas feitas à mão e pelos testes. Um intérprete de scripts (`phoenix script run`, `phoenix script report`, `phoenix cam-import`: scripts da linguagem de comandos do fluxo de trabalho anterior executados como operações reversíveis da placa, tudo ou nada, um relatório de compatibilidade por comando e por script, e trabalhos de saída lidos como perfil de exportação) é um protótipo, construído a partir da descrição pública da linguagem e testado só com scripts escritos para os seus testes. Um gerador de estruturas de RF (`phoenix rfgen`: linhas de microfita, curvas, stubs, redes de adaptação, filtros, um divisor de Wilkinson, um acoplador de ramos, antenas de patch e dipolo e uma transição SMA com cerca de vias, desenhados a partir do empilhamento e da frequência como operações reversíveis da placa com um relatório das fórmulas e da verificação de regras) é um protótipo, verificado só com valores de manuais e nunca comparado com um simulador eletromagnético nem com uma medição. Um conjunto de casos de referência (`phoenix oracle`) verifica os cálculos com valores publicados, a sua fonte e a sua tolerância; não é uma comparação com o EAGLE, o KiCad, um resolvedor de campo nem uma medição, e nenhum estado acima muda por isso. |
| Janela | 🟠 | O visualizador e editor de placas e esquemas (`phoenix view`, egui, atrás da funcionalidade `gui`) está escrito e a sua lógica está testada: uma placa edita-se com anular e refazer e com os achados das regras de cada edição, um esquema é verificado, sincronizado com uma placa e editado (colocar, mover, rodar, espelhar, apagar, copiar e colar, fios, etiquetas, portos de alimentação e nomes de barramento, com a verificação das regras após cada alteração), e nunca se guarda nada por cima do ficheiro lido; mas a janela nunca foi vista num ecrã: o desenho, o rato e a fluidez no ecrã não estão verificados. A mesma janela tem um modo de bibliotecas que edita os símbolos, pegadas e dispositivos das bibliotecas EAGLE (`phoenix libedit`, `phoenix footprint`, `docs/LIBRARY_EDIT.md`) com anulação, uma verificação da biblioteca e geradores de pegadas; também nunca foi vista num ecrã. Um modo de edição de placas (`phoenix boardedit`, `docs/BOARD_EDIT.md`) coloca componentes, traça pistas a 45 graus e vias, cola redes e ajusta comprimentos com meandros, e cada operação é verificada pelas regras de projeto; nunca foi visto num ecrã nem usado com um rato. Uma janela para os ficheiros Gerber e Excellon de uma pasta (`docs/VIEWER.md`: uma camada por ficheiro, opacidade, medição e comparação com a placa) e uma janela 3D da placa montada (câmara orbital, um clique escolhe um componente, colisões e alturas assinaladas) são protótipos; nenhuma foi vista num ecrã nem usada com um rato. O programa `phoenix-gui` abre-a com um ecrã inicial (abrir um ficheiro, uma biblioteca nova, ficheiros recentes, largar um ficheiro) e, no Windows, sem janela de consola. |
| Requisitos | 🟠 | 19 de 30 requisitos (o modo ocultado, os alicerces do modelo, as operações reversíveis, o ficheiro de projeto, os códigos de diagnóstico, as calculadoras RF analíticas, as verificações de conectividade e de regras e a exportação de fabrico com o seu pacote, o visualizador e editor de placas e esquemas os leitores e escritores de KiCad, o saneador de bibliotecas, as verificações e a sincronização do esquema o simulador de circuitos, a lógica de encaminhamento e o intérprete de scripts) estão em protótipo, 11 estão apenas planeados e nenhum está validado. |

## 🎯 Visão geral

* **O que existe:** um espaço de trabalho com uma biblioteca que conta um corpus privado de ficheiros de projeto em modo ocultado (apenas contagens, tipos, versões declaradas e uma impressão digital, nunca um nome nem um valor) e uma linha de comandos, `phoenix inventory`. Olha para os primeiros bytes de um ficheiro e não lê projetos. Uma segunda biblioteca guarda os alicerces do modelo de projeto: comprimentos e ângulos como inteiros com intervalo verificado, ids de objeto (UUID v7) e JSON canónico. Uma terceira biblioteca lê e escreve o projeto `.phoenixproj` sobre o contentor partilhado do EH3D-COMMON: o manifesto com os seus domínios, ligações e migrações, os domínios que não entende mantidos intactos e a regra `must_understand`. Uma quarta biblioteca guarda o modelo de placa: camadas, redes, pegadas, componentes, pistas com linhas e arcos, vias e zonas ligadas por ids estáveis, com operações reversíveis e um histórico de anular e refazer. Cada falha tem um código estável `PHOENIX-<ÁREA>-<NNN>`. A biblioteca de placa também calcula as ilhas de cada rede e verifica larguras, furos, anéis e distâncias com um ficheiro de regras em milímetros, separando o confirmado do inconclusivo. Uma quinta biblioteca lê bibliotecas, placas e esquemas EAGLE XML para um resumo, volta a escrever a árvore XML de forma idêntica e mede a taxa de leitura sobre cópias de um corpus verificadas com SHA-256, sem alterar os originais; o comando `phoenix eagle-rate` executa-o. Uma sexta biblioteca escreve os ficheiros Gerber X2, Excellon e `.eh3dproj` 1.0.0 de uma placa com EH3D-CORE-LIBS e EH3D-COMMON, com os mesmos bytes para a mesma placa; os ficheiros só são relidos pelos analisadores da mesma biblioteca. Uma sétima biblioteca guarda o esquema (símbolos, peças e redes ligados por ids estáveis) e confronta-o com uma placa: peças sem componente e diferenças de designadores, valores, pegadas e redes, com os códigos `PHOENIX-LINK-<NNN>`. Mais uma biblioteca, `eh3d-phoenix-rules`, deduz as redes do cobre (pastilhas, pistas, vias e zonas unidas pela geometria com uma tolerância), compara-as com as do esquema e executa as regras de projeto por uma ordem fixa contra limites lidos de um ficheiro de regras ou das regras de projeto de uma placa EAGLE, com os resultados confirmados e os inconclusivos em duas listas; os comandos `phoenix check` e `phoenix connectivity` imprimem o seu relatório como texto em oito idiomas ou como JSON.

* **Planeado:** EH3D-PHOENIX-PCB pretende ser um ambiente EDA com captura de esquemas, desenho de PCB, ferramentas de verificação, simulação de circuitos e módulos especializados para fontes comutadas e RF. Está planeado como um programa em Rust com janela para o ambiente de trabalho (um espaço de trabalho de crates de motor, uma interface de linha de comandos e uma aplicação) que funciona sozinho e pode usar o servidor para tarefas de cálculo. Nada disto existe ainda.

## 🧩 Papel no EH3D

EH3D-PHOENIX-PCB é o projeto principal da família PHOENIX da EH3D Fabrication Suite; os outros repositórios da família assentam nele. O que fará, o que ainda não fará, os seus requisitos e as suas relações com outros repositórios estão escritos na pasta `docs/`.

## 🏛️ Arquitetura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitetura de EH3D-PHOENIX-PCB: quem o usa, o que contém e de que depende" width="100%">
</p>

Como as partes deste repositório se encaixam: quem o usa, o que contém e de que depende. A imagem é desenhada a partir de `docs/architecture.json` e verificada com o código.

## 🚀 Início rápido

Precisa de Python 3.10 ou superior para as verificações e de uma cadeia de ferramentas Rust para o código.

```bash
cargo build --release
cargo test
cargo run -p eh3d-phoenix-cli -- inventory <folder>
python -I tools/ci_validate.py
python -I tools/make_architecture.py --check
python -I tools/make_structure.py --check --readmes
python -I -m unittest discover -s tools -p "test_*.py"
```

**Descarregar e abrir a janela.** O arquivo de uma versão (Windows de 64 e 32 bits, Linux e macOS) contém dois programas: `phoenix`, a linha de comandos, e `phoenix-gui`, a janela do visualizador e editor. Faça duplo clique em `phoenix-gui` (`phoenix-gui.exe` no Windows): abre um ecrã inicial onde pode abrir uma placa, um esquema ou uma biblioteca, escolher um dos ficheiros recentes ou largar um ficheiro sobre a janela; `phoenix-gui <ficheiro>` abre logo um ficheiro. No Windows não tem janela de consola, por isso, quando não abre, o motivo fica escrito em `last_error.txt`, na pasta de definições do utilizador (`%APPDATA%\EH3D-PHOENIX-PCB` no Windows, `~/.config/eh3d-phoenix-pcb` nos outros sistemas), onde também está a lista de ficheiros recentes (`recent.cfg`); `config/viewer.cfg` define quantos guarda. O `phoenix` sozinho só escreve texto e nunca abre uma janela. A janela é um protótipo que não foi experimentado com o rato (veja o estado acima). Para compilar os dois programas: `cargo build --release -p eh3d-phoenix-cli --features gui`.

## 🧪 Testes

O código tem 2123 testes (`cargo test`) e a base tem 3 (`python -I -m unittest discover -s tools -p "test_*.py"`); todos passam num computador. A CI executa a verificação do repositório, a das imagens, os testes da base e, para o código Rust, a verificação do formato, o clippy com os avisos tratados como erros e os testes em Linux e Windows.

## 🏗️ Compilação e publicação

A versão vive no ficheiro `VERSION` e em `eh3d.project.json`, e cada versão tem a sua entrada em [CHANGELOG.md](CHANGELOG.md). `cargo build --release` compila a biblioteca e o programa `phoenix`; as versões são publicadas no GitHub. Os arquivos de uma versão contêm o `phoenix` e, ao lado, o `phoenix-gui`, a janela; `cargo build --release -p eh3d-phoenix-cli --features gui` compila os dois.

## 📂 Estrutura do repositório

<p align="center">
  <img src="images/repository-structure.svg" alt="Estrutura do repositório EH3D-PHOENIX-PCB" width="100%">
</p>

<details>
<summary>Versão em texto da árvore</summary>

```text
EH3D-PHOENIX-PCB/
├── docs/
│   ├── architecture.json structure.json         os dados das duas imagens
│   ├── SCOPE.md REQUIREMENTS.md                 o âmbito e os requisitos
│   ├── DEPENDENCIES.md THREAT_MODEL.md          relações com outros repositórios e modelo de ameaças
│   ├── DECISIONS.md STATUS.md                   decisões que se aplicam aqui e estado honesto
│   ├── NOT_YET.md RULES.md                      ideias para mais tarde e regras para escrever código
│   ├── ROADMAP.md                               o mapa de fases, dos alicerces à suite tudo-em-um
│   ├── ERROR_CODES.md                           os códigos de diagnóstico PHOENIX-<ÁREA>-<NNN>
│   ├── STACKUP.md                               cálculos de empilhamento e linhas, fórmulas
│   ├── RF.md                                    ferramentas de RF: Touchstone, dois portos, adaptação, filtros
│   ├── RFGEN.md                                 estruturas de RF na placa: fórmulas, fontes e limites
│   ├── KICAD.md                                 os leitores de KiCad: o que se lê de cada formato
│   ├── KICAD_IMPORT.md                          ler um projeto KiCad completo: o comando, o relatório
│   ├── KICAD_EXPORT.md                          escrever ficheiros KiCad: o que se escreve, o relatório, o comando
│   ├── VIEWER.md                                o protótipo de visualizador e editor e o seu teste de desempenho
│   ├── SANITIZER.md                             o saneador de bibliotecas: duplicados, hash, relatório e plano
│   ├── SCRIPT.md                                o intérprete de scripts: comandos, relatório e trabalhos de saída
│   ├── ERC.md                                   verificação elétrica e sincronização
│   ├── SCHEMATIC_EDIT.md                        o editor de esquemas: operações, redes, janela
│   ├── LIBRARY_EDIT.md                          o editor de bibliotecas: edições, verificações, geradores
│   ├── BOARD_EDIT.md                            o editor de placas: operações, regras, janela
│   ├── SIM.md                                   simulação de circuitos: netlist, análises, exportação do esquema
│   ├── SIM_NONLINEAR.md                         simulador: dispositivos não lineares, varrimento em contínua, convergência
│   ├── ROUTE.md                                 edição e encaminhamento: ligações por fazer, encaminhador, edições reversíveis
│   ├── AUTOROUTE.md                             encaminhamento com um encaminhador automático externo: DSN, sessão, adaptador
│   ├── ZONES.md                                 preenchimento de zonas de cobre: recorte, alívios, ilhas, verificação
│   ├── LENGTHS.md                               comprimentos, grupos de igualação e pares diferenciais
│   ├── GERBER_IMPORT.md                         importação de ficheiros Gerber e Excellon para revisão, e a ida e volta
│   ├── PANEL.md                                 painelização: matriz, moldura, patilhas, V-score, fiduciais, furos de ferramenta
│   ├── BOM.md                                   lista de materiais com custos, CSV, JSON, HTML
│   ├── ASSEMBLY3D.md                            montagem 3D: modelos STEP, colisões, OBJ, STEP
│   ├── ORACLE.md                                casos de referência e o plano de comparação com o EAGLE e o KiCad
│   ├── GATE_F1.md                               o que a porta da primeira fase verifica e o que não verifica
│   └── adr/                                     os registos das decisões
├── AGENTS.md                                    regras para quem altera o repositório
├── deny.toml                                    política de licenças e avisos das dependências
├── crates/                                      inventário, modelo, projeto, placa, esquema, EAGLE, KiCad, regras, ERC, sincronização, hub, exportação, importação Gerber, painel, empilhamento, RF, simulador, encaminhador, zonas, comprimentos, lista de materiais, montagem 3D, saneador, scripts, visualizador, CLI
├── config/                                      o ficheiro de definições (todo o valor ajustado)
├── Cargo.toml Cargo.lock                        o espaço de trabalho Rust e as suas dependências fixadas
├── tools/                                       a verificação do repositório, os seus testes e as ferramentas das imagens
├── tests/                                       os casos de referência: valores publicados com fonte e tolerância
├── images/                                      banner e as imagens de Arquitetura e Estrutura do repositório
├── .github/                                     fluxos de CI e do painel, modelos de issues e pull requests
├── eh3d.project.json                            manifesto: versão, papel, maturidade e comando de compilação
├── VERSION                                      a versão do repositório
├── CHANGELOG.md                                 o que mudou em cada versão
├── SECURITY.md SUPPORT.md                       como comunicar uma vulnerabilidade e onde perguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   como contribuir, o código de conduta e a licença
```

</details>

## 🗺️ Roteiro e limites conhecidos

**Limites conhecidos hoje**

* Código inicial: só existem o inventário do corpus e os alicerces do modelo; o inventário só foi executado com pastas criadas pelos seus próprios testes, nunca com ficheiros de projeto reais.
* Não há esquema, nem leitor ou escritor de ficheiros de projeto de outros programas, nem editor, nem regras; o ficheiro de projeto e o modelo de placa só foram experimentados com dados gerados, nunca com um projeto real.
* A pilha final da interface de utilizador continua em aberto (há uma escolha provisória em `docs/adr/`), tal como a biblioteca de geometria (ver `docs/STATUS.md`).

**A seguir**

* Esperar que os contratos de que este projeto depende sejam aceites (ver `docs/DEPENDENCIES.md`), escrever depois o modelo e os primeiros leitores com testes e substituir as linhas de estado por resultados reais.

## 🌍 Idiomas

O README e os textos existem em inglês, espanhol, francês, italiano, alemão, chinês, japonês e português europeu.

## 🔗 Projetos relacionados

* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS): o mapa da suite, a sua arquitetura, as convenções e as verificações partilhadas
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): construído junto a este repositório; o contentor de projeto partilhado (`eh3d-container`) e o diagnóstico comum (`eh3d-diagnostic`) que este projeto usa
* Repositórios irmãos do PHOENIX: [EH3D-VALIDATION-LAB](https://github.com/JuanenRac/EH3D-VALIDATION-LAB) (corpus e oráculo de ficheiros de referência) e [EH3D-PHOENIX-CAD](https://github.com/JuanenRac/EH3D-PHOENIX-CAD) (lado mecânico), ambos em código inicial
* Relações planeadas, ainda por construir (EH3D-CORE-LIBS, EH3D-SERVER, EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE, EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER, EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-SUITE-LAUNCHER, EH3D-HYDRA-BRIDGE): ver [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md)

## 📚 Documentação

* [As convenções da suite](https://github.com/JuanenRac/EH3D-DOCS/blob/main/docs/CONVENTIONS.md)
* [Registo de alterações](CHANGELOG.md)
* [Âmbito, requisitos, dependências, modelo de ameaças e decisões](docs/SCOPE.md)

## 🤝 Contribuir e comunidade

* Antes de um pull request, execute `python -I tools/ci_validate.py` e os testes da base: tudo tem de passar. Consulte [CONTRIBUTING.md](CONTRIBUTING.md) e o [código de conduta](CODE_OF_CONDUCT.md).
* O README existe em oito idiomas: uma alteração ao texto faz-se nos oito ficheiros, e as duas imagens são desenhadas a partir de `docs/architecture.json` e `docs/structure.json` com `tools/make_architecture.py` e `tools/make_structure.py`.
* Comunique uma vulnerabilidade em privado: [SECURITY.md](SECURITY.md). Perguntas, ideias e relatórios: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Ver [LICENSE](LICENSE). Os projetos de hardware da suite usam a CERN OHL-S.
