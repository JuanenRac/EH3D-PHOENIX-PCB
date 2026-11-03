<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# 🛠️ EH3D-PHOENIX-PCB

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | 🇵🇹 <b>Português</b></p>

### EH3D-PHOENIX-PCB: planeado como um ambiente EDA com captura de esquemas, desenho de PCB, ferramentas de verificação, simulação de circuitos e módulos especializados para fontes comutadas e RF. Até agora só está escrito o inventário do corpus.

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20Python-blue.svg" alt="Rust and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-67-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** um espaço de trabalho Rust com um único comando, `phoenix inventory`, que conta os ficheiros de projeto de uma pasta privada sem mostrar nenhum nome nem valor: ficheiros por extensão, tipo e versão declarada, e uma impressão digital do conteúdo. Tem 64 testes próprios, que passam num computador. Só foi executado com pastas criadas pelos seus próprios testes, nunca com ficheiros de projeto reais. Tem também os alicerces de um modelo de projeto (valores inteiros em nanómetros e micrograus com intervalo verificado, ids de objeto e JSON canónico). Não há objetos de projeto, nem leitor ou escritor de ficheiros de projeto, nem editor, nem regras: toda a função de EDA continua planeada.

---

> **Estado de segurança (safety) e de segurança informática:** este repositório está nos primeiros passos e não é um sistema de segurança (safety). Nada do que está escrito aqui substitui os limites, os bloqueios e a paragem de emergência da máquina e do seu controlador.

## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.1` |
| Maturidade | Estrutura inicial |
| Testes | 67 |
| Tecnologias | Rust, Python 3.10+ |
| Em hardware real | Ainda não executado em hardware |

**Ir para:** [Visão geral](#-visão-geral) · [Arquitetura](#-arquitetura) · [Início rápido](#-início-rápido) · [Testes](#-testes) · [Estrutura do repositório](#-estrutura-do-repositório) · [Projetos relacionados](#-projetos-relacionados)

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Verificações do repositório | 🟡 | A verificação do repositório, a das imagens e os 3 testes da base correm na CI e passam. |
| Testes | 🟡 | Os 64 testes do código Rust e os 3 da base passam num computador. Usam apenas pastas criadas pelos próprios testes; nenhum foi executado com ficheiros de projeto reais. |
| Hardware real | 🟠 | Não foi executado em nenhuma máquina nem placa: nada aqui foi experimentado em hardware. |
| Função planeada | 🟠 | Escrito: o inventário ocultado de um corpus e os alicerces do modelo (unidades, ids, JSON canónico). Por começar: os objetos de projeto, os leitores e escritores, as regras, o visualizador e os editores, a simulação e tudo o resto do planeado. |
| Requisitos | 🟠 | 2 de 30 requisitos (o modo ocultado e os alicerces do modelo) estão em protótipo, 28 estão apenas planeados e nenhum está validado. |

## 🎯 Visão geral

* **O que existe:** um espaço de trabalho com uma biblioteca que conta um corpus privado de ficheiros de projeto em modo ocultado (apenas contagens, tipos, versões declaradas e uma impressão digital, nunca um nome nem um valor) e uma linha de comandos, `phoenix inventory`. Olha para os primeiros bytes de um ficheiro e não lê projetos. Uma segunda biblioteca guarda os alicerces do modelo de projeto: comprimentos e ângulos como inteiros com intervalo verificado, ids de objeto (UUID v7) e JSON canónico.

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

O código tem 64 testes (`cargo test`) e a base tem 3 (`python -I -m unittest discover -s tools -p "test_*.py"`); todos passam num computador. A CI executa a verificação do repositório, a das imagens, os testes da base e, para o código Rust, a verificação do formato, o clippy com os avisos tratados como erros e os testes em Linux e Windows.

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
│   └── adr/                                     os registos das decisões aceites
├── AGENTS.md                                    regras para quem altera o repositório
├── deny.toml                                    política de licenças e avisos das dependências
├── crates/                                      o inventário do corpus, os alicerces do modelo e a linha de comandos
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
* Não há objetos de projeto, nem ficheiro de projeto, nem leitor ou escritor de ficheiros de projeto, nem editor, nem regras; além do modo ocultado e dos alicerces do modelo, nenhum requisito foi começado.
* A interface de utilizador e a biblioteca de geometria continuam a ser decisões em aberto (ver `docs/STATUS.md`).

**A seguir**

* Esperar que os contratos de que este projeto depende sejam aceites (ver `docs/DEPENDENCIES.md`), escrever depois o modelo e os primeiros leitores com testes e substituir as linhas de estado por resultados reais.

## 🌍 Idiomas

O README e os textos existem em inglês, espanhol, francês, italiano, alemão, chinês, japonês e português europeu.

## 🔗 Projetos relacionados

* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS): o mapa da suite, a sua arquitetura, as convenções e as verificações partilhadas
* Relações planeadas, ainda por construir (EH3D-COMMON, EH3D-CORE-LIBS, EH3D-SERVER, EH3D-PHOENIX-CAD, EH3D-VALIDATION-LAB, EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE, EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER, EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-SUITE-LAUNCHER, EH3D-HYDRA-BRIDGE): ver [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md)

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
