# EH3D-PHOENIX-PCB: review of security and robustness

**State: review made by the project itself.** No external audit has been made and nothing here is a certification. The tests listed below show that the checks they make pass; they do not show that the program is secure, and files that nobody has thought of may still make it fail. The threat model is in [THREAT_MODEL.md](THREAT_MODEL.md); how to report a vulnerability is in [SECURITY.md](../SECURITY.md).

## What was assumed

| Who | Assumption |
|---|---|
| The files that are read (boards, schematics, libraries, Gerber and Excellon, DXF, netlists, scripts, parts tables, settings) | not trusted: they may be damaged, huge, deeply nested or made on purpose to make a reader fail |
| The person who runs the program | the user of the computer: they choose the files to read and the places to write; the program must not write anywhere else, over a file that exists or to a place that is not a file |
| The network | not used: the program opens no connection and starts no other program, except the optional adapter of an external autorouter that is off until it is turned on in the settings and named on the command line |

## What was looked at

The code of production (not the tests) of every crate that reads or writes a file: the readers of EAGLE, KiCad, Gerber and Excellon, DXF, SPICE-style netlists, scripts, parts tables, touchstone files and settings, and the writers of every command. The method was a search of the source for each of the hazards below, a reading of every place that the search found, and a test for each thing that was changed.

* a panic on data of a file (`unwrap`, `expect`, `panic!`, `unreachable!`, an index that is not checked, a division by zero);
* arithmetic that can overflow on data of a file, and `as` that cuts a number;
* a loop without a limit, a recursion without a depth limit, a memory allocation whose size is a number read from the file;
* a file read whole without a size limit;
* a place where a file is written: how it is created, whether it can write over a file that exists or follow a link, and what decides the name.

## What was found and what was done

| Id | Finding | Change | Test |
|---|---|---|---|
| SR-1 | The settings files of `inventory --config`, `corpus --settings`, `dxf-import --config`, `perf --config` and of the window were read without a size limit | read through `bounded::read_config_text`: at most 1 MiB, then PHOENIX-SEC-003 | `bounded` in the command line crate |
| SR-2 | The list of recent files of the start screen was read without a size limit | at most 1 MiB is read | `only_the_first_megabyte_of_the_list_of_recent_files_is_read` |
| SR-3 | The atomic write of `sync --out` and `netlist --out`, of the reports of the sanitizer and of the results of the simulator wrote its side file with a call that follows links and writes over a file | the side file is created exclusively; one that is already there stops the write (PHOENIX-SEC-002) and is not touched | one test in each of the three crates |
| SR-4 | The preferences of the window are saved through a side file that was created in the same way | created exclusively; a regular file left by an interrupted save is removed first, a link is never followed | `a_side_file_left_by_an_interrupted_save_does_not_block_the_next_one` |
| SR-5 | `FabSet::write_folder`, used by `phoenix export --out`, replaced the files of a folder that existed and followed links | it creates every file exclusively and, before writing anything, refuses a folder in which one of its files is already there | `a_file_that_is_already_in_the_folder_is_never_replaced` |
| SR-6 | The copies of `eagle-rate` replaced the files of an earlier run in the same folder | each copy is created exclusively | `a_copy_never_replaces_a_file_that_is_already_in_the_folder` |
| SR-7 | A name taken from a file (the name of a board, of a footprint, a file name of an export profile or of a CAM job) could be a name that Windows keeps for a device (`CON`, `NUL`, `AUX`, `COM1`...) and then it is not a file | such a name is refused in the profiles and the jobs and gets a `_` in front in the KiCad file names; `eh3d_phoenix_model::is_reserved_device_name` | one test in each of the three crates, and the tests of the function |
| SR-8 | The value of `--out`, `--dir`, `--into`, `--svg` or `--csv` could be a device path, a path of another computer, an alternate data stream (`name:stream`) or a part named like a device | `phoenix` refuses it before any command runs, with PHOENIX-SEC-001 and the exit code 2 (`eh3d_phoenix_model::output_path_problem`) | `every_output_option_refuses_the_paths_that_are_not_acceptable` |

Nothing else was changed: the search did not find, in the code of production of the readers, an `unwrap`, an `expect` or a `panic!` that data of a file can reach (the few that exist are on values fixed in the code, such as the size of an A4 sheet in nanometres, or on a step that was just checked), an allocation whose size is a number read from a file without a limit, or a recursion without a depth limit. That is a result of a search and of a reading, not a proof: the test of hostile input below is what looks for the rest.

## The limits that are in force

Every number of the first table is read from the settings that the repository ships, and the test `the_limits_of_the_document_are_the_ones_of_the_settings` fails when the document and the file say different things. A limit can be lowered or raised in the file; a line that is not a known key is refused.

<!-- limits:start -->
| Reader | Settings | Key | Value |
|---|---|---|---|
| KiCad files | config/kicad.cfg | max_file_bytes | 134217728 |
| KiCad files | config/kicad.cfg | max_depth | 64 |
| KiCad files | config/kicad.cfg | max_nodes | 5000000 |
| KiCad files | config/kicad.cfg | max_string_bytes | 1048576 |
| KiCad files | config/kicad.cfg | max_sheets | 256 |
| KiCad files | config/kicad.cfg | max_sheet_depth | 8 |
| KiCad project | config/kicad.cfg | max_project_bytes | 4194304 |
| Gerber and Excellon | config/gerber_import.cfg | max_file_bytes | 67108864 |
| Gerber and Excellon | config/gerber_import.cfg | max_files | 64 |
| Gerber and Excellon | config/gerber_import.cfg | max_commands | 5000000 |
| Gerber and Excellon | config/gerber_import.cfg | max_apertures | 4096 |
| Gerber and Excellon | config/gerber_import.cfg | max_objects | 2000000 |
| Gerber and Excellon | config/gerber_import.cfg | max_macro_primitives | 256 |
| Gerber and Excellon | config/gerber_import.cfg | arc_max_segments | 4096 |
| Gerber and Excellon | config/gerber_import.cfg | coordinate_limit_nm | 1073741824 |
| DXF | config/dxf.cfg | max_file_bytes | 16777216 |
| DXF | config/dxf.cfg | max_entities | 500000 |
| DXF | config/dxf.cfg | max_block_depth | 8 |
| DXF | config/dxf.cfg | max_blocks | 10000 |
| Simulator | config/sim.cfg | max_file_bytes | 4194304 |
| Simulator | config/sim.cfg | max_lines | 200000 |
| Simulator | config/sim.cfg | max_elements | 20000 |
| Simulator | config/sim.cfg | max_unknowns | 1500 |
| Simulator | config/sim.cfg | max_subckt_depth | 16 |
| Simulator | config/sim.cfg | max_include_depth | 4 |
| Simulator | config/sim.cfg | max_include_files | 16 |
| Simulator | config/sim.cfg | max_tran_steps | 1000000 |
| Scripts | config/script.cfg | max_script_bytes | 1048576 |
| Scripts | config/script.cfg | max_lines | 20000 |
| Scripts | config/script.cfg | max_commands | 10000 |
| Scripts | config/script.cfg | max_steps | 1000000 |
| Scripts | config/script.cfg | max_loop_iterations | 10000 |
| Scripts | config/script.cfg | max_block_depth | 16 |
| Scripts | config/script.cfg | max_text_bytes | 262144 |
| Scripts | config/script.cfg | max_file_bytes | 16777216 |
| Parts tables | config/parts.cfg | max_files | 200 |
| Parts tables | config/parts.cfg | max_file_bytes | 16777216 |
| Parts tables | config/parts.cfg | max_parts | 200000 |
| Libraries (editor) | config/libedit.cfg | max_file_bytes | 268435456 |
| Footprint families | config/ipc.cfg | max_family_bytes | 1048576 |
| Footprint families | config/ipc.cfg | max_file_bytes | 67108864 |
| Sanitizer | config/sanitizer.cfg | max_files | 200000 |
| Sanitizer | config/sanitizer.cfg | max_depth | 32 |
| Sanitizer | config/sanitizer.cfg | max_file_bytes | 268435456 |
| Router | config/route.cfg | max_cells | 200000 |
| Router | config/route.cfg | max_expansions | 3000000 |
| 3D assembly | config/assembly3d.cfg | max_file_bytes | 16777216 |
| 3D assembly | config/assembly3d.cfg | max_step_entities | 2000000 |
| Own corpus | config/corpus.cfg | max_file_bytes | 8388608 |
| Own corpus | config/corpus.cfg | max_entries | 2000 |
| Own corpus | config/corpus.cfg | max_depth | 8 |
<!-- limits:end -->

Two limits are fixed in the code and have no line in the settings; the test `the_limits_fixed_in_the_code_are_the_ones_of_the_document` compares them with the document.

<!-- code-limits:start -->
| Reader | Limit | Value |
|---|---|---|
| EAGLE XML (`Limits::default`) | max_bytes | 536870912 |
| EAGLE XML (`Limits::default`) | max_depth | 256 |
| EAGLE XML (`Limits::default`) | max_nodes | 20000000 |
| Settings files of the command line | MAX_CONFIG_BYTES | 1048576 |
<!-- code-limits:end -->

## How the files are written

The commands write new files and new folders and refuse a name that exists; only the three that go through a side file (second row) replace a result of the same name. The way is one of these; the test `the_code_of_production_writes_only_in_the_ways_of_the_document` searches the source for the calls that do not follow them and fails on one that is not in the list below.

| Way | Where |
|---|---|
| The file is created with `create_new` (the call fails when something is at the name, a link included) | the commands of the command line, the export folders, the package of manufacturing files, the library editor, the parts base, the printing, the sanitizer when it applies a plan, the files that a script writes |
| A side file created with `create_new`, then a rename over the old file; one side file that is already there stops the write | `sync --out`, `netlist --out`, the reports of the sanitizer, the results of the simulator (the old result of the same name is replaced on purpose: a run is repeated) |
| A side file that an interrupted save left is removed, and the file is then created with `create_new` and renamed | the preferences of the window |

These calls do not follow those ways and are on purpose:

* the error file of the window program (`gui_main.rs`) and the list of recent files (`launcher_session.rs`): small files of the window in its own folder of settings;
* the save of a file by the window (`session.rs`): it writes over a file only when the window itself saved it before, and makes a copy first;
* the files of the external autorouter (`autorouter.rs`): created in a folder that was made new for the run.

The value of `--out`, `--dir`, `--into`, `--svg` and `--csv` is looked at before any command runs (SR-8). A `..` in a path is accepted: the person who types it chooses where the output goes. In the places that were reviewed, a name that comes from a file is not used as a path: the names of the files that are written are made from letters, digits and a few signs (SR-7). The path where a package of the manufacturing files is written to (`--eh3dproj`, `--package`) is written by the shared library of EH3D-COMMON and was not reviewed here.

## Network and processes

The test `no_source_file_opens_the_network_or_starts_a_program_outside_the_list` reads every source file of the workspace and fails on a use of `std::net`, of a socket, of `Command::new`, of `std::process` or of the name of a library of networking that is not in an explicit list with its reason, and on an item of the list that nothing uses. The list holds the adapter of the external autorouter (`crates/eh3d-phoenix-route/src/autorouter.rs`: off unless `enabled = true` in `config/autorouter.cfg` and `--execute` on the command line, started directly without a shell and with an empty environment), the exit code of the two programs, and the number of the process in the name of the temporary folder of four tests. Another test fails when `Cargo.lock` holds a library of networking, and one fails when the shipped `config/autorouter.cfg` does not say `enabled = false`.

## Hostile input

The test `no_reader_panics_or_takes_too_long_on_a_changed_file_of_the_corpus` takes the files of `tests/corpus`, `tests/dxf`, `examples` and `config`, changes each many times with a generator of pseudo-random numbers of the repository (a fixed seed, so the same files are made every time) and gives the result to every reader that takes such a file: EAGLE (XML, board, schematic, library, the deep check and the design rule files), KiCad (board, schematic, footprint, symbols, project), Gerber and Excellon, DXF, the board as JSON, the session of the autorouter, SPICE-style netlists, scripts, parts tables, touchstone files, footprint families, the fabrication package and the settings files. A change is one to three of these: cut the file, repeat a piece of it, change bytes, put an extreme number (the largest and smallest integers, `NaN`, `1e999`, numbers just outside the range of the design), nest 100 to 4100 levels deep, leave a piece out, replace a number by an extreme one. A reader may refuse a file; the test fails on a panic (caught with `catch_unwind`) and on a file that takes longer than the limit. The seed, the number of changed files for each reader (200), the largest size and the time limit are in `config/fuzz.cfg`; a key that is not there is refused.

## What was not covered

* **Memory.** The limits bound the size of the input and the number of items, not the memory that the program uses: the limits of the table can still ask for gigabytes on a computer with little memory. Lower them in the settings for such a computer.
* **Time of the algorithms.** The test of hostile input measures time on the changed files only. The routers, the zone filling, the simulator and the stack-up solvers on a large legitimate design are not bounded in time beyond the limits of the table.
* **The EAGLE reader has limits in the code only.** Its three limits are not in a settings file.
* **Arithmetic.** The model keeps coordinates inside a checked range in integer nanometres, and the readers refuse what is outside it; the sums and products of the rules, the router and the zone filling were looked at in the places where the search pointed, not in every line of the workspace.
* **Links.** The commands that read a file follow links, as the system does. The ones that write do not follow a link at the name of the file, but a link in a folder of the path is followed.
* **Races.** A check that a file does not exist before a write is followed by a write that is exclusive; between the two another program on the same computer is not considered.
* **Libraries of other people.** The dependencies are pinned and checked in the build (see [DEPENDENCIES.md](DEPENDENCIES.md)); the code of EH3D-COMMON and of the dependencies was not reviewed.
* **The window.** The code of the graphical interface was read for the files it opens and writes, not for the behaviour of the graphics libraries it uses.
* **Hardware and machines.** Nothing here is about a machine, a controller or a power circuit.

## Codes

`PHOENIX-SEC-001` output path refused, `PHOENIX-SEC-002` the side file of a write is already there, `PHOENIX-SEC-003` settings file larger than its limit; see [ERROR_CODES.md](ERROR_CODES.md).
