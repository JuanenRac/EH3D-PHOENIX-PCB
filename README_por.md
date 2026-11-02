<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# 🛠️ EH3D-PHOENIX-PCB

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | 🇵🇹 <b>Português</b></p>

### EH3D-PHOENIX-PCB: planeado como um ambiente EDA com captura de esquemas, desenho de PCB, ferramentas de verificação, simulação de circuitos e módulos especializados para fontes comutadas e RF. Ainda sem código.

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Markdown%20%2B%20Python-blue.svg" alt="Markdown and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-3-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** nada. EH3D-PHOENIX-PCB está apenas planeado: não tem código, nem testes próprios, e nunca foi executado. O que existe é a base que todo o repositório da suite leva, mais o âmbito, os requisitos, as dependências, o modelo de ameaças e as decisões escritos em `docs/`.

---

> **Estado de segurança (safety) e de segurança informática:** este repositório está nos primeiros passos e não é um sistema de segurança (safety). Nada do que está escrito aqui substitui os limites, os bloqueios e a paragem de emergência da máquina e do seu controlador.

## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.1` |
| Maturidade | Estrutura inicial |
| Testes | 3 |
| Tecnologias | Markdown, Python 3.10+ |
| Em hardware real | Ainda não executado em hardware |

**Ir para:** [Visão geral](#-visão-geral) · [Arquitetura](#-arquitetura) · [Início rápido](#-início-rápido) · [Testes](#-testes) · [Estrutura do repositório](#-estrutura-do-repositório) · [Projetos relacionados](#-projetos-relacionados)

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Verificações do repositório | 🟡 | A verificação do repositório, a das imagens e os 3 testes da base correm na CI e passam. |
| Testes | 🟠 | Só a base tem testes (3); o código do repositório terá os seus à medida que for escrito. |
| Hardware real | 🟠 | Não foi executado em nenhuma máquina nem placa: nada aqui foi experimentado em hardware. |
| Função planeada | 🔴 | Por começar: nada da função planeada está escrito. |
| Requisitos | 🔴 | Todos os requisitos estão apenas planeados: nenhum está implementado nem verificado. |

## 🎯 Visão geral

* **Planeado:** EH3D-PHOENIX-PCB pretende ser um ambiente EDA com captura de esquemas, desenho de PCB, ferramentas de verificação, simulação de circuitos e módulos especializados para fontes comutadas e RF. Está planeado como um programa em Rust com janela para o ambiente de trabalho (um espaço de trabalho de crates de motor, uma interface de linha de comandos e uma aplicação) que funciona sozinho e pode usar o servidor para tarefas de cálculo. Nada disto existe ainda.

## 🧩 Papel no EH3D

EH3D-PHOENIX-PCB é o projeto principal da família PHOENIX da EH3D Fabrication Suite; os outros repositórios da família assentam nele. O que fará, o que ainda não fará, os seus requisitos e as suas relações com outros repositórios estão escritos na pasta `docs/`.

## 🏛️ Arquitetura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitetura de EH3D-PHOENIX-PCB: quem o usa, o que contém e de que depende" width="100%">
</p>

Como as partes deste repositório se encaixam: quem o usa, o que contém e de que depende. A imagem é desenhada a partir de `docs/architecture.json` e verificada com o código.

## 🚀 Início rápido

Precisa de Python 3.10 ou mais recente; não há nada a instalar.

```bash
python -I tools/ci_validate.py
python -I tools/make_architecture.py --check
python -I tools/make_structure.py --check --readmes
python -I -m unittest discover -s tools -p "test_*.py"
```

## 🧪 Testes

A base tem 3 testes (`python -I -m unittest discover -s tools -p "test_*.py"`), e passam. A CI executa a verificação do repositório, a das imagens e esses testes.

## 🏗️ Compilação e publicação

A versão vive no ficheiro `VERSION` e em `eh3d.project.json`, e cada versão tem a sua entrada em [CHANGELOG.md](CHANGELOG.md). Não há nada a compilar; as versões são publicadas no GitHub.

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
│   ├── SCOPE.md REQUIREMENTS.md                 o que está planeado e os seus requisitos
│   ├── DEPENDENCIES.md THREAT_MODEL.md          relações com outros repositórios e modelo de ameaças
│   ├── DECISIONS.md STATUS.md                   decisões que se aplicam aqui e estado honesto
│   ├── NOT_YET.md RULES.md                      ideias para mais tarde e regras para escrever código
│   └── ROADMAP.md                               o mapa de fases, dos alicerces à suite tudo-em-um
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

* Apenas planeado: não há código, nada foi executado e nenhum requisito está implementado.

**A seguir**

* Esperar que os contratos de que este projeto depende sejam aceites (ver `docs/DEPENDENCIES.md`), escrever depois a primeira versão com testes e substituir as linhas de estado por resultados reais.

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
