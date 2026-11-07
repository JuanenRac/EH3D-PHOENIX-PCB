# Escrever os resultados de uma verificação como tabela e como página, e verificar uma placa com o perfil de um fabricante

Executam-se as regras de projeto, guardam-se os resultados como tabela CSV e como página HTML agrupada por categoria, e depois verifica-se uma pequena placa EAGLE com um perfil de capacidades de um fabricante. O perfil do exemplo é inventado: mostra a forma do ficheiro e não é a capacidade de nenhum fabricante.

Execute os comandos a partir da raiz do repositório; veja [como executar os guias](README.md).

1. Execute as regras de um ficheiro de regras e escreva os dois relatórios. Cada opção escreve um ficheiro novo: nunca se substitui um que exista, e não se escreve nada se uma das saídas for recusada (código de saída 2). A pista da placa é mais fina do que o ficheiro pede: um resultado confirmado e o código de saída é 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json --csv build/check.csv --html build/check.html examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ Confirmed: 1
~ PHOENIX-CHECK-001
```

2. Escreva os mesmos relatórios noutro idioma. A coluna do título do CSV e os cabeçalhos da página seguem `--lang`; as chaves das colunas, das categorias e das gravidades ficam em inglês para que um script as leia em qualquer idioma.

```console
$ phoenix check --lang es --rules examples/tutorials/rules_strict.json --csv build/check_es.csv --html build/check_es.html examples/tutorials/kicad/demo.kicad_pcb
```

3. A comparação de um esquema e de uma placa aceita as mesmas duas opções.

```console
$ phoenix connectivity --csv build/connectivity.csv --html build/connectivity.html examples/tutorials/tut.sch examples/tutorials/tut.brd
```

4. Verifique a placa EAGLE com o perfil `examples/guides/dfm_small_shop.cfg`. Com `--dfm` os limites do perfil substituem as regras da placa, por isso não se indica `--rules`. A placa mede 30 mm por 20 mm e o perfil permite 10 mm no lado curto: um erro com o código `PHOENIX-DFM-003` e código de saída 1. Uma placa com mais de uma camada de cobre recebe também `PHOENIX-DFM-001`, e a informação `PHOENIX-DFM-006` avisa que o perfil não está verificado.

```console
$ phoenix check --dfm examples/guides/dfm_small_shop.cfg --csv build/dfm.csv --html build/dfm.html examples/tutorials/tut.brd
```

5. Agora o perfil de exemplo do repositório, `config/dfm/example.cfg`. A placa está dentro de todos os seus limites, por isso o único resultado da categoria de capacidade é a informação de que o perfil não está verificado.

```console
$ phoenix check --dfm config/dfm/example.cfg --html build/dfm_example.html examples/tutorials/tut.brd
```

## Como ler o resultado

* O CSV tem o cabeçalho `list,category,severity,code,title,location,x_nm,y_nm,measured_nm,limit_nm,detail` e uma linha por resultado, os comprovados primeiro (`list` é `confirmed` ou `inconclusive`). Uma célula de texto que uma folha de cálculo executaria como fórmula leva uma plica à frente.
* A página HTML não tem script nem ficheiro externo e escapa todo o texto que vem da placa, por isso pode abrir-se sem risco num navegador. Começa com a contagem dos resultados comprovados e inconclusivos de cada categoria e continua com uma tabela por cada categoria que tenha resultados; os do perfil estão na categoria `capability`.
* O perfil é um ficheiro de texto com linhas `chave = valor` em nanómetros; uma linha em falta ou com 0 está desligada. Copie `config/dfm/example.cfg`, ponha as cifras que lê na documentação do seu fabricante, anote em `source` onde as leu e só então defina `verified = true`. Nada verifica o ficheiro junto de um fabricante.

## Ver também

* [docs/DRC_ADVANCED.md](../../../../docs/DRC_ADVANCED.md)
* [docs/DFM.md](../../../../docs/DFM.md)
* [docs/RULES.md](../../../../docs/RULES.md)
