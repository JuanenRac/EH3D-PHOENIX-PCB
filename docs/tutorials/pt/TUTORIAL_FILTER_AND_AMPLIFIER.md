# Simular um filtro RC e um amplificador inversor

Dois circuitos escritos como netlists ao estilo SPICE: um passa-baixo RC de 1 kΩ e 1 µF (`rc_step.cir` e `rc_ac.cir`) e um amplificador inversor de ganho −R2/R1 com um amplificador operacional (`inv_amp.cir`). Tiram-se medidas com `--measure`, percorre-se uma lista de valores com `--step`, encontram-se a frequência de corte e o polo, e faz-se um estudo de Monte Carlo com semente fixa.

Execute os comandos a partir da raiz do repositório; veja [como executar os tutoriais](README.md).

1. Meça o tempo de subida de 10 % a 90 % da resposta ao degrau. A tabela tem a coluna `run` e uma coluna para a medida. Num circuito RC é ln 9 · R · C, cerca de 2,197 ms.

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 10u 10m --measure "tran rise10 rise v(out)"
run,rise10
~ nominal,
```

2. Execute o circuito uma vez por cada valor de uma resistência com `--step`. Com 2 kΩ o tempo de subida duplica. A tabela tem uma linha por valor.

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 20u 40m --step "R1 list 1k 2k" --measure "tran rise10 rise v(out)"
r1,run,rise10
```

3. Varra a frequência e peça as figuras de mérito da resposta. A frequência de corte do passa-baixo é 1 / (2π R C), cerca de 159 Hz.

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 100 1 100k --figures "v(out)"
name,value,unit
~ cutoff_3db,
```

4. Ajuste os polos e zeros à resposta em CA. O polo deste circuito é real e está em −1 / (R C) = −1000 rad/s.

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 20 10 100k --poles "v(out)"
~ pole,
```

5. Agora o amplificador inversor. Com R1 = 1 kΩ e R2 = 10 kΩ o ganho é −10; uma entrada de 1 V dá −9,999 V à saída, um pouco menos de 10 V em valor absoluto, porque o ganho do modelo do amplificador operacional é finito (1e5).

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --op --probe "v(out)"
name,value
~ v(out),-9.99
```

6. Percorra três valores de R2 e meça a saída no único ponto de uma varredura em CC. Os ganhos ficam perto de −2, −10 e −20.

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --dc V1 1 1 1 --step "R2 list 2k 10k 20k" --measure "dc vout at v(out) 1"
r2,run,vout
```

7. Monte Carlo: um divisor cujas resistências têm tolerância de 1 % (`tol=1%` na netlist). A mesma semente dá a mesma tabela em qualquer computador. Depois das execuções vêm as linhas `count`, `mean`, `std`, `min`, `max` e os percentis.

```console
# test
$ phoenix sim examples/tutorials/div_tol.cir --mc 200 --seed 5
run,vb
```

## Ver também

* [docs/SIM.md](../../SIM.md)
* [docs/SIM_ADVANCED.md](../../SIM_ADVANCED.md)
* [docs/SIM_NONLINEAR.md](../../SIM_NONLINEAR.md)
* [docs/ORACLE.md](../../ORACLE.md)
