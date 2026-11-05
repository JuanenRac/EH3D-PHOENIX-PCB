# EH3D-PHOENIX-PCB: not yet

Good ideas that are not in the first version. Writing them here avoids implementing them early.

| Idea | Why not yet | Revisit when |
|---|---|---|
| Differential pairs, length tuning and advanced routing | Basic routing comes first | after the PCB editor works |
| Co-simulation of firmware with the circuit | Needs one emulator per processor family, starting with a single one | after circuit simulation |
| Full parametric mechanical CAD | Belongs to EH3D-PHOENIX-CAD and comes after its first steps | after the first mechanical steps |
| Electromagnetic co-simulation and dynamic heat maps | Needs the RF tools and the thermal laboratory | after both exist |
| Plugins loaded at run time | No real third-party need yet; an internal versioned interface comes first | when a third party asks for it |
| Native TLS on the server link | A documented HTTPS proxy is used first | after the secure configuration is tested |
| Rules per net class, per layer pair and per area | The first rule file has one value per rule | after the schematic editor has net classes
| Exact clearance of arcs, round-cornered pads and filled zones | Arcs are judged by their end points and pad shapes by two circles; exact curves need the geometry library that is still to be chosen | after the geometry library is chosen
| Hole-to-copper and hole-to-edge distances and thermal-relief checks | The first set of rules covers sizes, clearances, the edge, drill spacing, mask and silkscreen | when the manufacturing profiles exist
| Frequency-dependent (dispersive) models of the microstrip and coplanar lines | The first calculator is quasi-static and warns when the substrate is a large part of the wavelength | after the stack-up calculator is compared with a field solver |
| Even and odd modes, common-mode impedance and crosstalk of coupled lines | The pair has only the differential impedance of the IPC-2141 fit | after the stack-up calculator is compared with a field solver |
| Solder mask, etch factor and plane cut-outs in the field of a line | The first calculator leaves them out and says so | after the stack-up calculator is compared with a field solver |
