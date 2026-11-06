# Arredondar os cantos de um preenchimento de cobre e escrever alívios térmicos e limites de ilhas nos ficheiros Gerber

A placa `examples/guides/pour.brd` tem um preenchimento de massa na camada `Top` em redor de duas resistências e de uma pista. Preenche-se com cantos vivos e com cantos arredondados e comparam-se, e depois escrevem-se os ficheiros Gerber com um perfil que dá alívio térmico às pastilhas retangulares do preenchimento e deixa de fora as suas ilhas pequenas. Indica-se o ficheiro de regras `rules_relaxed.json` porque uma placa EAGLE sem regras de projeto não tem limites próprios.

Execute os comandos a partir da raiz do repositório; veja [como executar os guias](README.md).

1. Preencha a zona com as definições do repositório, que deixam todos os cantos vivos (`corner_radius_nm = 0`). O relatório dá os polígonos e os nanómetros quadrados de cobre.

```console
$ phoenix zones fill --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

2. Preencha-a de novo com `zones_round.cfg`, que define `corner_radius_nm = 1000000` (um arco de 1 mm). Arredondam-se os cantos do preenchimento virados para fora e os cantos das folgas do alívio térmico; compare a área de cobre com a primeira execução.

```console
$ phoenix zones fill --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

3. Meça o preenchimento arredondado em relação à placa e às regras. O arredondamento retira cobre ao preenchimento ou só o acrescenta dentro de uma folga de um alívio térmico, por isso o preenchimento não se aproxima do cobre de outra rede e a verificação não encontra erros de afastamento.

```console
$ phoenix zones check --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

4. Escreva primeiro os ficheiros Gerber com o perfil genérico. A pastilha `R2.2` da rede de massa está dentro do preenchimento e fica ligada a ele sem alívio.

```console
$ phoenix export --out build/pour_plain examples/guides/pour.brd
```

5. Agora com o perfil `pour` de `export_pour.json`: quatro raios (`gerber.zone_thermal_spokes`), uma folga e um raio de 0,25 mm, o mesmo alívio para as pastilhas retangulares e oblongas (`gerber.zone_thermal_non_round`) e nenhuma ilha abaixo de 1 milímetro quadrado (`gerber.zone_island_min_area_nm2`). Compare o ficheiro de cobre da camada `Top` nas duas pastas: o segundo tem as folgas do alívio escritas como regiões de polaridade clara à volta da pastilha.

```console
$ phoenix export --profiles examples/guides/export_pour.json --profile pour --out build/pour_relief examples/guides/pour.brd
```

## Como ler o resultado

* Nas definições do preenchimento, `corner_radius_nm` é o raio do arco (zero deixa todos os cantos vivos) e `corner_tolerance_nm` quanto as cordas do arco se podem afastar dele. Um arco ocupa no máximo metade de cada lado junto ao canto, por isso um raio que não cabe é reduzido. Os cantos dos buracos que outro cobre deixa no preenchimento não são arredondados.
* Na exportação, um alívio tem quatro raios ou dois (`gerber.zone_thermal_spokes` é 0, 2 ou 4; 0 deixa as pastilhas ligadas em sólido). Uma pastilha rodada por um ângulo que não é reto fica em sólido e di-lo (`PHOENIX-EXPORT-022`). Uma ilha menor do que `gerber.zone_island_min_area_nm2` não é escrita e o relatório di-lo (`PHOENIX-EXPORT-021`); zero escreve todas as zonas.

## Ver também

* [docs/ZONES.md](../../../../docs/ZONES.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
