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
| Copper pours, thermal reliefs and clearance around other nets in the Gerber files | A zone is written as its outline; a pour needs the exact geometry library that is still to be chosen | after the geometry library is chosen |
| Polygon pads, round-cornered and chamfered pads, and pads turned by any angle in the Gerber files | The board model has only circle, rectangle and oblong pads and the export flashes a turned pad unrotated unless the angle is a right angle (`PHOENIX-EXPORT-001`) | when the model has pad shapes with outlines and a macro aperture is written |
| Blind and buried vias, back drilling and the blind and buried drill files | The export treats every via as a plated through hole; the model has layer pairs but no drill span rule | after the stack-up has net classes and via spans |
| Object attributes of the Gerber files (aperture functions and the net, component and pin names) | The shared writer writes file attributes only, and the viewers that were meant to check the output are not chosen | when the shared writer writes them and two independent viewers are chosen |
| Plating of slots, castellated holes and press-fit tolerances in the Excellon files | The model does not say which drilled pad is a slot or what its plating is; a drilled oblong pad is a slot only when the profile asks for it | after the pad model has a hole shape and a plating field |
| Pick and place and BOM formats of real machines and assemblers, and importing them | The two CSV files are plain and their columns are a profile setting; no real sample has been compared, and the rule for the rotation of a bottom part is a setting that nobody has confirmed with an assembler | when real samples exist (PPCB-010) |
| Opening the output in two independent Gerber and Excellon viewers and a fabrication review | Until then the state of the export stays at prototype (PPCB-009) | when two viewers are chosen and a manufacturer has looked at a sample |
| Touchstone version 2 keywords, a reference resistance per port and mixed-mode parameters | The reader and writer cover version 1.0 and 1.1 with one reference resistance | when files with those keywords are needed |
| Loss, parasitics and component tolerance in the ladder filters | The filters are ideal and the response is exact for ideal elements | after the stack-up calculator and the RF tools are compared with measurements |
| Noise circles, gain circles and stability circles | The first RF level gives figures (K, mu, gain, VSWR), not circles | after the figures are compared with a network analyser |
| Double stubs, tapered lines and multi-section broadband matching | The first level has the L network, the single stub and the quarter-wave section | after the first level is compared with measurements |
| De-embedding of fixtures and conversion of H and G parameters | Not needed for the first calculators; H and G files are read and refused when they have to be converted | when a measurement needs them |
| Drawing of the Smith chart and of the response curves | The RF tools give the coordinates and the circles; a drawing module belongs to the viewer | after the viewer exists |
| Writing KiCad files (schematics, boards and libraries) | The readers come first and the writer needs the oracle to say that KiCad opens the files (PPCB-013) | after the readers are compared with KiCad on a public corpus |
| KiCad graphics, buses, no-connect flags, the setup of a board, the filled copper of zones, keep-out zones and 3D models in the model | The model holds none of them; the readers count them as left out | when the model has them |
| KiCad pads with rounded corners, trapezoids and custom shapes, oval drills and the plating of a pad | The model has circle, rectangle and oblong pads and a drill diameter; the readers read the rest as the rectangle of its size or a round hole | with the pad shapes of the model |
| Nets of a KiCad schematic by the rules of KiCad (bus members, power flags, no-connect, buses of nets) | The nets are computed from wires, junctions, labels and power symbols and are not compared with KiCad | after the comparison with the oracle |
| Editing in the viewer (select to move, rotate, delete, properties) | The viewer is read only and exists to decide the interface stack | after the stack is decided on a real screen |
| Grid, rule list and ratsnest in the viewer | The prototype shows layers, nets, the measure and the position of the pointer; PPCB-014 also asks for these | after the prototype has been seen on a screen |
| A canvas on the graphics card (wgpu) and a cache of the triangles of the polygons | The prototype draws through the painter of egui on the processor; only the preparation of the data has been timed | when the frame time measured on a real screen misses the target of `docs/adr/0009-first-interface-stack.md` |
| Drawing of the symbols of a schematic and a view of the board from the bottom | A part is drawn as the box around its pins because the model holds no drawing of a symbol, and the board is drawn from the top only | after the schematic editor and the symbol library exist |
| Applying the plan of the library sanitizer (replace the duplicates by the kept item and relink the device sets and the placed parts, with a backup) | The sanitizer only analyses; the replacement must prove first that it changes neither the layout nor the rules: backup with a checked hash, same pads, same names, no turn, same positions and nets, same rule findings, read-back of the written libraries (`docs/SANITIZER.md`) | after the analysis has been run on real libraries and the owner has confirmed groups |
| A redacted form of the sanitizer report | The report names libraries and items because the owner needs them to confirm a group; a mode with only codes, counts and hashes is needed before a report leaves the machine | when a report has to leave the machine |
| Tolerances tuned on real libraries, texts, unusual pad shapes and electrical attributes in the score of the sanitizer | The defaults are guesses checked on libraries made by hand; the score looks at positions, sizes, outline and silkscreen only | after the analysis has been run on real libraries |
