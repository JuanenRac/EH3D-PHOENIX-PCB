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
  <img src="https://img.shields.io/badge/Tests-288-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** um espaço de trabalho Rust cujo comando `phoenix inventory` conta os ficheiros de projeto de uma pasta privada sem mostrar nenhum nome nem valor: ficheiros por extensão, tipo e versão declarada, e uma impressão digital do conteúdo. Tem 285 testes próprios, que passam num computador. Só foi executado com pastas criadas pelos seus próprios testes, nunca com ficheiros de projeto reais. Tem também os alicerces de um modelo de projeto (valores inteiros em nanómetros e micrograus com intervalo verificado, ids de objeto e JSON canónico), o ficheiro de projeto `.phoenixproj` (manifesto, domínios, migrações) e um modelo de placa com operações reversíveis (camadas, redes, componentes, pistas, vias, zonas), tudo experimentado apenas com dados gerados. A biblioteca de placa também descobre que cobre toca em que cobre e verifica tamanhos e distâncias com um ficheiro de regras; uma quinta biblioteca lê bibliotecas, placas e esquemas EAGLE XML para um resumo e volta a escrever a árvore XML de forma idêntica. Tudo foi experimentado apenas com ficheiros gerados ou feitos à mão. Existem também um modelo de esquema e a sua verificação de coerência com a placa. Não há importação de projetos alheios para o modelo, nem editor, e as regras cobrem só parte das verificações previstas: a maior parte da função de EDA continua planeada. O modelo da placa também guarda texto e traços de serigrafia (com um tipo de letra de traços incorporado) que a verificação de regras e a exportação Gerber usam, e o esquema pode ser desenhado em folhas com peças colocadas e fios que são comparados com a lista de redes e com a placa; uma placa pode ser escrita como ficheiro XML do EAGLE e lida de novo, o que só foi verificado com ficheiros escritos por este código e nunca foi aberto no EAGLE; o contrato para pares de ficheiros binários do EAGLE e do seu XML está escrito, mas não existe nenhum leitor dos ficheiros binários. Uma biblioteca de conectividade e regras (`eh3d-phoenix-rules`) lê os limites de um ficheiro de regras ou das regras de projeto de uma placa EAGLE, verifica uma placa por uma ordem fixa (largura, via, furo, anel, distância, limite da placa, distância entre furos, máscara de soldadura, zonas, serigrafia), encontra curto-circuitos e redes por encaminhar e compara as redes de uma placa com as de um esquema, e os comandos `phoenix check` e `phoenix connectivity` imprimem o relatório com os resultados confirmados e os inconclusivos em separado; só foi experimentada com placas feitas à mão e nunca foi comparada com a ferramenta de referência.

---

> **Estado de segurança (safety) e de segurança informática:** este repositório está nos primeiros passos e não é um sistema de segurança (safety). Nada do que está escrito aqui substitui os limites, os bloqueios e a paragem de emergência da máquina e do seu controlador.

## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.2` |
| Maturidade | Estrutura inicial |
| Testes | 288 |
| Tecnologias | Rust, Python 3.10+ |
| Em hardware real | Ainda não executado em hardware |

**Ir para:** [Visão geral](#-visão-geral) · [Arquitetura](#-arquitetura) · [Início rápido](#-início-rápido) · [Testes](#-testes) · [Estrutura do repositório](#-estrutura-do-repositório) · [Projetos relacionados](#-projetos-relacionados)

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Verificações do repositório | 🟡 | A verificação do repositório, a das imagens e os 3 testes da base correm na CI e passam. |
| Testes | 🟡 | Os 285 testes do código Rust e os 3 da base passam num computador. Usam apenas pastas criadas pelos próprios testes; nenhum foi executado com ficheiros de projeto reais. |
| Hardware real | 🟠 | Não foi executado em nenhuma máquina nem placa: nada aqui foi experimentado em hardware. |
| Função planeada | 🟠 | Escrito: o inventário ocultado de um corpus, os alicerces do modelo (unidades, ids, JSON canónico), o ficheiro de projeto `.phoenixproj`, o modelo de placa com anular e refazer, a sua conectividade e uma primeira verificação de regras, e um leitor de resumo de EAGLE XML. Por começar: o editor de esquemas, a importação de outros formatos para o modelo, os escritores de outros formatos, a maior parte das regras, o visualizador e os editores, a simulação e tudo o resto do planeado. O modelo da placa também guarda texto e traços de serigrafia (com um tipo de letra de traços incorporado) que a verificação de regras e a exportação Gerber usam, e o esquema pode ser desenhado em folhas com peças colocadas e fios que são comparados com a lista de redes e com a placa; uma placa pode ser escrita como ficheiro XML do EAGLE e lida de novo, o que só foi verificado com ficheiros escritos por este código e nunca foi aberto no EAGLE; o contrato para pares de ficheiros binários do EAGLE e do seu XML está escrito, mas não existe nenhum leitor dos ficheiros binários. |
| Requisitos | 🟠 | 7 de 30 requisitos (o modo ocultado, os alicerces do modelo, as operações reversíveis, o ficheiro de projeto, os códigos de diagnóstico e as verificações de conectividade e de regras) estão em protótipo, 23 estão apenas planeados e nenhum está validado. |

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

## 🧪 Testes

O código tem 285 testes (`cargo test`) e a base tem 3 (`python -I -m unittest discover -s tools -p "test_*.py"`); todos passam num computador. A CI executa a verificação do repositório, a das imagens, os testes da base e, para o código Rust, a verificação do formato, o clippy com os avisos tratados como erros e os testes em Linux e Windows.

## 🏗️ Compilação e publicação

A versão vive no ficheiro `VERSION` e em `eh3d.project.json`, e cada versão tem a sua entrada em [CHANGELOG.md](CHANGELOG.md). `cargo build --release` compila a biblioteca e o programa `phoenix`; as versões são publicadas no GitHub.

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
│   ├── GATE_F1.md                               o que a porta da primeira fase verifica e o que não verifica
│   └── adr/                                     os registos das decisões aceites
├── AGENTS.md                                    regras para quem altera o repositório
├── deny.toml                                    política de licenças e avisos das dependências
├── crates/                                      inventário, modelo, projeto, placa, esquema, EAGLE, regras, hub, exportação, CLI
├── config/                                      o ficheiro de definições (todo o valor ajustado)
├── Cargo.toml Cargo.lock                        o espaço de trabalho Rust e as suas dependências fixadas
├── tools/                                       a verificação do repositório, os seus testes e as ferramentas das imagens
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
* A interface de utilizador e a biblioteca de geometria continuam a ser decisões em aberto (ver `docs/STATUS.md`).

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
