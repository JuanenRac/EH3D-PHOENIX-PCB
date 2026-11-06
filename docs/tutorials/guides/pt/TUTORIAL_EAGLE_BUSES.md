# Ler os barramentos de um esquema EAGLE

`examples/guides/bus.sch` é o esquema do primeiro tutorial com um barramento `D[0..1]` desenhado na folha e duas redes, `D0` e `D1`, cujos fios terminam no fio do barramento. O leitor transforma o nome do barramento num objeto barramento com os seus membros e põe uma entrada onde cada rede encontra o barramento. Um barramento é uma forma de dar nomes: os seus membros são redes comuns.

Execute os comandos a partir da raiz do repositório; veja [como executar os guias](README.md).

1. Leia a hierarquia como JSON. A chave `buses` conta os barramentos da folha: 1. A lista de problemas não tem nenhum código da família `PHOENIX-BUS`, porque os dois membros são usados por uma entrada.

```console
$ phoenix hier --json examples/guides/bus.sch
```

2. O mesmo em texto: a lista de redes só tem as redes que ligam pelo menos dois pinos, aqui `MID`, pelo que os membros `D0` e `D1` do barramento, que não têm pino, não aparecem nela, e a última linha diz se há problemas.

```console
$ phoenix hier examples/guides/bus.sch
```

3. Escreva o esquema como ficheiros KiCad. O barramento é escrito como a sua etiqueta e uma linha desde a sua âncora pelas extremidades das suas entradas, e cada entrada como um `bus_entry` com a etiqueta do seu membro.

```console
$ phoenix kicad-export --out build/bus.kicad_sch schematic examples/guides/bus.sch
```

## Como ler o resultado

* Um vetor `prefixo[primeiro..último]` dá os membros `prefixo<primeiro>` a `prefixo<último>` (`D[0..1]` dá `D0` e `D1`; um vetor pode descer, como `A[2..0]`). O leitor aceita também as listas do EAGLE (`A[0..3],B[0..3]`) e um nome com dois pontos (`BUS:A[0..3]`), em que o que precede os dois pontos dá nome ao próprio barramento.
* O barramento é ancorado onde começa o seu primeiro fio. Cria-se uma entrada na extremidade de cada rede da folha que tem o nome de um membro e termina num fio do barramento. Os fios e as etiquetas do barramento, os nomes que não são vetores e um barramento sem fio não são lidos e contam-se como partes omitidas. Os códigos `PHOENIX-BUS-001` a `PHOENIX-BUS-007` dizem o que está mal num barramento, e `phoenix hier` lista-os.

## Ver também

* [docs/BUSES.md](../../../../docs/BUSES.md)
* [docs/HIERARCHY.md](../../../../docs/HIERARCHY.md)
* [docs/KICAD_EXPORT.md](../../../../docs/KICAD_EXPORT.md)
