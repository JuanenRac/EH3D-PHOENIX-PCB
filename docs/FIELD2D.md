# EH3D-PHOENIX-PCB: the field solver of the cross section of a line

The crate `eh3d-phoenix-field2d` and the command `phoenix field2d` solve the electric field in the cross section of a transmission line and work out its impedance, effective permittivity, inductance, delay and dielectric loss, and for a pair of lines the coupling and the differential and common impedance. They are the second opinion beside the closed formulas of `phoenix stackup` (`docs/STACKUP.md`).

**State: prototype.** The solver has been checked against solutions that are known exactly and against the closed formulas of the stack-up crate. It has not been compared with another field solver or with a measurement of a board, the copper loss is not computed, the solder mask is left out, the conductors are rectangles and the open space around the line is a box with walls at zero potential.

## What it solves

The line is taken as quasi-TEM: the field in the cross section follows the Laplace equation `div(eps grad phi) = 0` with the relative permittivity `eps` of each layer, taken at the frequency from the materials of the stack-up (`dk_at`), and the conductors at a fixed potential. With the capacitance per unit length `C` of the section and `C_air` of the same section with vacuum in place of every dielectric:

* `Z0 = 1 / (c sqrt(C C_air))` and `eps_eff = C / C_air`;
* `L = 1 / (c^2 C_air)`: the inductance per unit length is that of the vacuum line, by reciprocity (the copper is not magnetic);
* delay `sqrt(eps_eff) / c`;
* dielectric loss `alpha_d = pi f sqrt(eps_eff) tan_eff / c` in Np/m, with `tan_eff` the energy in each cell weighted by its loss tangent over the whole energy of the field (the odd mode for a pair).

For a symmetric pair the capacitance matrix gives the mutual capacitance `Cm = -C12`, the even mode `Ce = C11 - Cm` and the odd mode `Co = C11 + Cm`, the impedance of each mode as above, `Zdiff = 2 Zodd` and `Zcommon = Zeven / 2`; the inductance matrix is `C_air^-1 / c^2`, with the mutual inductance `Lm` its off-diagonal term. The report also gives how far the two conductors came out from being equal.

## The mesh and the equations

The mesh is rectangular and not uniform. It has a line at every edge of a conductor, of a layer and of the box. Between two lines the cells start at `edge_step_fraction` of the smallest feature of the section (a thickness, a width, a gap, the height of a layer) at the edge of a conductor and grow by `grading_ratio` for each cell up to `max_step_fraction` of the smaller side of the box. A boundary between two dielectrics always lies on a line, so it is handled exactly: the coefficient of the edge between two nodes is the sum of the contribution of the two cells next to it, `eps * size / (2 * length)` (finite volumes, which for a regular mesh is the five-point difference). A node that touches a conductor has its potential, a node on a wall at zero potential is at zero, and a wall called symmetry has no flux through it. Two conductors, or a conductor and a wall, closer than a cell are refused (`PHOENIX-FIELD-013`).

The charge on a conductor is the sum, over the edges that join one of its nodes to a free node, of the coefficient times the difference of potential. The system is solved for each conductor at one volt in turn, by conjugate gradients with a diagonal preconditioner (`method = cg`) or by successive over-relaxation in a fixed order of the nodes (`method = sor`). A solution that has not reached `tolerance` within `max_iterations` gives `PHOENIX-FIELD-004` and no number, and a mesh of more than `max_cells` cells is refused (`PHOENIX-FIELD-003`). Every loop runs in a fixed order and every sum is made term by term in `f64`, so the same input gives the same bytes.

## The cross section of a line

`phoenix field2d` takes the line from the stack-up as `phoenix stackup` does, but keeps the layers apart instead of combining them into one:

* microstrip and pair of microstrips: the layers between the track and the plane, each with its own permittivity and loss tangent, the track on top and air above it;
* stripline and pair of striplines: the layers on each side up to the planes;
* coplanar line, with or without a plane under the substrate: the grounded copper on both sides of the gaps, as far as the box. Without a plane the substrate is followed by air down to a wall at zero potential, and the report says so (`PHOENIX-FIELD-012`).

The box is `box_margin_factor` times the sum of the height of the section and the width the conductors take away from them on every open side, with its walls at zero potential. That closes a region that is open in reality, and it makes the impedance a little lower than it would be without walls; a larger factor costs cells.

## The command

```
phoenix field2d <config|board> [--stackup FILE] [--net NAME] [--settings FILE] [--freq-ghz GHZ]
                [--refine N] [--out FILE] [--json] [--lang CODE]
phoenix stackup <job.json> --freq-ghz <GHz> [--field2d] ...
```

With a job file of `phoenix stackup` (the stack-up, its materials and the lines) every line is solved: a line given by its width with that width, a line given by a target impedance with the width the closed formula finds for it, so the comparison shows how far the field is from the target. With a board and `--stackup FILE` the layer and the width most of the copper of each net runs on (or of `--net NAME`) give a microstrip on an outer layer and a stripline on an inner one. `--refine N` divides every step of the mesh by N for a convergence study. `phoenix stackup --field2d` adds the same solution to the report of `stackup` (a key `field2d` in the JSON).

For each line the report gives the mesh, the iterations and the residual, the impedance, permittivity, inductance, capacitance in vacuum and with the dielectric, delay and dielectric loss, for a pair the modes and the coupling, and the closed formula beside it with the difference in percent. A difference above `compare_warn_percent` is the warning `PHOENIX-FIELD-009`. The command is a dry run: it writes nothing unless `--out` names a file, which must not exist. Exit code 0 when every line was solved, 1 when a line could not be, 2 when the arguments or the input are unusable.

## Settings

Every tuned value is in `config/field2d.cfg`, which is also the built-in default: the method, the tolerance, the limits of iterations, cells and lines, the fineness and the grading of the mesh, the factor of the box, the frequency and the percentage that makes a difference a warning. An unknown key or a value out of range is refused with `PHOENIX-FIELD-001`.

## What the tests check

* The capacitor of parallel plates, alone and with two dielectrics in series, is exact on any mesh (the unit tests of `grid.rs`).
* The capacitance of a coaxial line of circular conductors, `2 pi eps / ln(b/a)` (a text on transmission lines such as Pozar, *Microwave Engineering*; the number of the equation is not verified here), with the circles drawn as steps of the mesh: the error is required to fall as the steps get smaller.
* The impedance of a centred stripline of 2 micrometres of thickness against the exact formula of zero thickness of S. B. Cohn, *Characteristic impedance of the shielded-strip transmission line*, IRE Transactions on Microwave Theory and Techniques, July 1954 (the page numbers are not verified here), within a percent and a half.
* A wide microstrip in vacuum against the formula of Hammerstad and Jensen (1980) as implemented in the stack-up crate, within a percent, and a microstrip on a laminate within a few percent of it.
* The change of the answer when the mesh is refined by 2 and by 4 must shrink.
* Conjugate gradients and over-relaxation give the same line; a pair far apart behaves as two lines and a coplanar line with far ground copper tends to the microstrip.
* The same input gives the same numbers, and a limit of cells or of iterations that is too small gives an error and no number.
* The reference cases of `phoenix oracle` (`docs/ORACLE.md`) repeat the parallel plates, the coaxial line and the stripline against their sources.

## Limits

* The result depends on the box: a closer wall lowers the impedance. The factor is a setting and has not been studied against a case with an open region.
* The conductors are rectangles, with square corners, and the singularity of the field at a corner is only resolved by the fineness of the mesh at the edge.
* The copper loss is not computed here (the stack-up crate has it), and the roughness, the skin effect and the dispersion are not part of the model.
* The solder mask over a microstrip or a coplanar line is left out, as in the closed formulas, and says so (`PHOENIX-FIELD-011`).
* A pair is taken as symmetric; the even and odd modes are those of two equal conductors.
* A coplanar line without a plane is an approximation, not the structure of the board.
* Nothing has been compared with another field solver (the case in `tests/oracle/field.json` is pending) or with a measurement.
