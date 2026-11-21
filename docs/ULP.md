# EH3D-PHOENIX-PCB: the user programs of the reference tool

The crate `eh3d-phoenix-ulp` and the command `phoenix ulp` read the user programs of the reference tool (files with the extension `.ulp`: reports, bills of materials, exports and batch edits), say what each one uses and whether the list of allowed names covers it, and run the part of them that only computes and reads. This is the first three steps of the plan of `docs/adr/0011-ulp-interpreter-for-the-owner-list.md`: the compatibility report, the reader and the evaluator. **The sandbox of the fourth step, a separate low-privilege process, is not written**: see "What is not written".

**State: prototype.** It has been tried on small programs written for the tests and for `examples/ulp/`, on a board and a schematic built by the tests, and on text made by generators with a fixed seed. No program of a third party was used, none has been compared with the original tool, and the list of allowed names is a first list that waits for the programs that are really used.

```
phoenix ulp check  [--config FILE] [--json] [--lang CODE] <program>...
phoenix ulp report [--config FILE] [--support FILE] [--json] [--lang CODE] <program>...
phoenix ulp run    [--config FILE] [--board FILE] [--schematic FILE] [--expect FILE] [--lang CODE] <program>
```

`check` and `report` read files and write nothing. `check` reads each program and lists what is wrong with its shape; the exit code is 0 when every program was read without a problem, 1 when some has one and 2 when the arguments or a file are unusable. `report` says what each program uses and what the list says about it; the exit code is 0 when every program is covered, 1 when some is partly or not covered and 2 as above. `--json` prints the result with its keys in a fixed order, the same for the same program; `--lang` chooses one of the eight languages of the suite for the words and the titles of the codes.

`run` runs one program and prints what the program prints (and nothing else) to the standard output. The design is given with `--board` (the JSON of the model, an EAGLE `.brd` or a KiCad `.kicad_pcb`) and `--schematic` (the JSON of the model or an EAGLE `.sch`). The exit code is 0 when the program ran and, with `--expect FILE`, its output is the one in the file (the line ends of the file are normalised); 2 when the verification did not pass, that is, the output differs (`PHOENIX-ULP-041`) or the program ended with `exit(n)` and a number other than zero; 1 when it could not be run: a wrong argument, a file that cannot be read, a program with problems of reading (`PHOENIX-ULP-039`), an error of the run or a limit. A run that ends in an error prints nothing to the standard output, so a partial result is never taken for a result.

## The reader

The reader is a lexer and a parser of the shape of a program: directives (`#include`, `#usage`, `#require`), declarations of `char`, `int`, `real`, `string` and the types of the objects of the design (names that start with `UL_`), functions the program defines, the statements `if`, `for`, `while`, `do`, `switch`, `case`, `default`, `break`, `continue`, `return` and blocks, expressions with the operators of C, member access, indexes and calls, and the statements that a call opens with a block (a walk over the objects of the design, an output file, a dialog and its layouts). A mistake is a problem with its line and column and the reading goes on from the next `;` or `}`, so one run shows several problems. The reader keeps what it could read and drops the part with the problem.

It is bounded, and every bound is a line of `config/ulp.cfg`:

| Key | Default | What it bounds |
|---|---|---|
| `max_source_bytes` | 1048576 | the size of a program; a larger one is refused before it is looked at |
| `max_tokens` | 200000 | the tokens of a program |
| `max_depth` | 100 | the nesting of blocks, parentheses and expressions; the reader calls itself once for each level (the chains of operators, members, indexes and calls may be eight times as long) |
| `max_statements` | 50000 | the statements, with declarations and functions |
| `max_string_chars` | 65536 | a text in quotes |
| `max_name_chars` | 128 | a name |
| `max_problems` | 50 | the problems kept for a program; the rest are counted |

A program that goes beyond the size, the tokens, the nesting or the statements is an error and nothing of it is kept. A file that is not valid UTF-8 is read with the letters that cannot be read replaced, because old programs are often not UTF-8 and only their comments hold such letters.

## The report

For each program the report gives its size, its SHA-256, the number of statements and functions, what it uses, the problems of the reading, a note for each name that needs one, a verdict, and whether the evaluator can run the program.

What a program uses is listed by kind and name, with how many times and the line of the first time: the directives, the kinds of statement, the types, the functions it calls, the members of objects it reads (`E.name`, `B.elements(E)`) and the named values it names (`UL_WIRE`, `PI`, `argv`). A name the program defines itself (a function, a variable, a parameter, the variable of a walk) is not a use of the language.

The list of names is the data file `config/ulp_support.cfg`, one `kind name status` for each line, and `--support FILE` reads another. A name that ends in `*` stands for every name that starts with the rest, and an exact name wins over one with a star. The status is one of four:

| Status | Meaning | Note |
|---|---|---|
| `allowed` | read access to the model and pure computation | none |
| `limited` | allowed inside the limits of the sandbox: a private folder for the files, a size limit, no command of the operating system | `PHOENIX-ULP-022` |
| `pending` | not decided or not written yet | `PHOENIX-ULP-021` |
| `denied` | the sandbox never lets it through | `PHOENIX-ULP-020` |

A name that the list does not know has the note `PHOENIX-ULP-019`.

The verdict follows from the names and from the reading:

* **covered**: the program was read without a problem and every name it uses is allowed or limited;
* **partly covered**: the program was read, no name is denied, and some name is pending or not on the list;
* **not covered**: a name is denied (running a command of the operating system is), or the program could not be read, or a limit stopped the reading.

The verdict says what the list would let through. Whether the evaluator runs each name is said apart, because a name can be allowed by the list and not be written in the evaluator (the clock, the sort, the dialogs) and a name that is `limited` (a file) is refused by the evaluator, which opens none. In the JSON: `"evaluator": "read_only"`, `"sandbox": "not_written"`, `"ran": false` (the report never runs a program), `"runnable"` (true when the program was read without a problem and every name it uses is one that the evaluator runs; it does not say that the run will end well), `"not_run"` (the names the evaluator does not run) and `"runs"` in each use. The text of the report says the same in the eight languages. No document of this repository claims that the language as a whole is supported.

## The evaluator

`phoenix ulp run` reads the program (a program with any problem of reading is not run, `PHOENIX-ULP-039`) and evaluates it on a thread with a stack made for the depth that the settings allow. It runs:

* **Values and types:** `int` and `char` (a 64-bit whole number; an overflow is an error, `PHOENIX-ULP-031`), `real` (never infinite and never not-a-number: a result that would be is an error), `string`, arrays of these that grow when an element beyond the end is written, and handles on the objects of the design. An assignment converts to the declared type (a real to an int truncates; a text is never converted to a number or the other way round).
* **Expressions:** the operators of C (arithmetic, comparison, `&&` and `||` that stop early, bit operators, shifts, `?:`, the comma, `++` and `--` before and after, the assignments `=`, `+=`, `-=` and the like), a text joined with `+` (to a text or to the code of a character), the index of an array and of a text (the character), the member of an object.
* **Statements:** declarations, blocks, `if`, `for`, `while`, `do`, `switch` with `case` and `default`, `break`, `continue`, `return`, and the functions the program defines (they may call themselves; an array is passed as a copy; a variable declared outside any function is seen by all of them).
* **Walks over the design, read only:** `board(B)` and `schematic(S)` bind a variable to the design that `--board` and `--schematic` gave (`PHOENIX-ULP-036` when it was not given), and `X.walk(V)` runs its block for each object. `break` and `continue` work inside a walk. The objects, their walks and their members:

| Object | Walks | Members |
|---|---|---|
| board | `elements`, `signals`, `layers`, `wires`, `vias`, `polygons`, `packages` | `name` |
| element (a placed part) | `pads` (placed) | `name` (the reference), `value`, `package` (a package), `x`, `y`, `angle` (degrees), `mirror` (1 on the bottom side) |
| package (a footprint) | `pads` (relative to its origin) | `name` |
| pad | | `name` (the number), `x`, `y`, `shape` (`circle`, `rect` or `oblong`), `diameter` (a circle; 0 otherwise), `width`, `height`, `drill`, `signal` (the net of a placed pad; an empty text if none) |
| signal (a net) | `wires`, `vias`, `polygons` | `name` |
| layer | | `name`, `number` (its place in the order of the layers of the board, from 1) |
| wire (a piece of a track) | | `x1`, `y1`, `x2`, `y2`, `width`, `layer` (its number), `signal`, `length` (the straight distance between its ends) |
| via | | `x`, `y`, `drill`, `diameter`, `signal` |
| polygon (a zone) | `vertices` | `layer`, `signal`, `area` (a real number in square nanometres, from the outline) |
| vertex | | `x`, `y` |
| schematic | `parts`, `nets`, `sheets` | `name` |
| part | | `name` (the reference), `value` |
| net of the schematic | | `name` |
| sheet | | `name`, `number`, `width`, `height` |

A length is a nanometre (the unit of the model) and an angle is a real number of degrees. The place of a placed pad is the origin of its part plus the place of the pad (its x mirrored on the bottom side) turned by the angle of the part; a turn by a multiple of a quarter is exact and any other uses the sine and the cosine of the machine, rounded to the nanometre. The objects are visited in the order of the model (by identity), so the same design gives the same order every time.

* **Built-in functions, pure:** `abs`, `acos`, `asin`, `atan`, `ceil`, `cos`, `exp`, `floor`, `frac`, `log`, `log10`, `max`, `min`, `pow`, `round`, `sin`, `sqrt`, `tan` (angles in radians; `ceil`, `floor` and `round` give a whole number, rounding half away from zero); `u2mm`, `u2mic`, `u2mil`, `u2inch` and `mm2u`, `mic2u`, `mil2u`, `inch2u` (from and to nanometres); `strlen`, `strlwr`, `strupr` (they return a new text), `strstr`, `strrstr`, `strchr`, `strrchr` (the place in characters, -1 when it is not found), `strsub`, `strjoin`, `strtod`, `strtol`, `strsplit` (fills an array of texts, returns how many), `sprintf` (writes its first argument, a text variable, and returns the length); `filedir`, `filename`, `fileext`, `filesetext` (text operations on a name, they touch no file). The constants `PI`, `INT_MAX`, `INT_MIN`, `CHAR_MAX`, `CHAR_MIN`, `REAL_MAX`, `REAL_MIN`, `REAL_EPSILON` and `EOF`.
* **The output:** `printf` formats like the C library for `%d %i %u %c %s %f %F %e %E %g %G %x %X %o` and `%%`, with the flags `- + space 0 #`, a width and a precision, and collects the text in memory; nothing is written until the run ends well. `exit(n)` ends the run with the number `n`.

The sine, the cosine, the tangent, the exponential and the logarithms come from the mathematical library of the machine, so their last digit can differ between machines; everything else is exact, and the same program, the same design and the same limits give the same bytes on the same machine.

### The limits of a run

Every limit is a line of `config/ulp.cfg` with its range; a program that goes beyond one stops with its code and prints nothing.

| Key | Default | Range | Code | What it bounds |
|---|---|---|---|---|
| `max_steps` | 1000000 | 1 to 10000000000 | `PHOENIX-ULP-025` | every statement and every part of an expression counts one; an endless loop ends here |
| `max_memory_bytes` | 16777216 | 1024 to 1073741824 | `PHOENIX-ULP-026` | the text and the array elements that the variables hold, and a text made by joining |
| `max_eval_depth` | 200 | 1 to 2000 | `PHOENIX-ULP-027` | the nesting of statements, expressions and calls (a function that calls itself without end ends here) |
| `max_output_bytes` | 1048576 | 0 to 1073741824 | `PHOENIX-ULP-028` | what the program prints |
| `max_array_elements` | 100000 | 1 to 10000000 | `PHOENIX-ULP-026` | the elements of one array |

### Errors of a run

An error has a code, the line and the column, and a detail: `PHOENIX-ULP-029` a type that the operation does not accept, `030` a division by zero, `031` a number out of range, `032` a name that is not defined, `033` an index out of range, `034` something the sandbox does not allow, `035` something the evaluator does not run yet, `036` a design that was not given, `037` wrong arguments, `038` a change to the design, `040` a format that does not match its arguments.

### What the evaluator does not do

* **No file, no program, no network, no clock, no environment.** The names the list marks `denied` (`system`, `sleep`) and the ones it marks `limited` because they need a file or a folder (`output`, `fileread`, `fileglob`, `filesize`, `filetime`, `fileerror`, `#include`) are refused with `PHOENIX-ULP-034`: the evaluator gives no limit that opens anything. The names that are `pending` or allowed but not written (`time` and the other functions of the clock, `sort`, `lookup`, `status`, `country`, `language`, `xml*`, `project`, `library`, the dialogs `dlg*`, `argc` and `argv`, the constants `UL_*`) are refused with `PHOENIX-ULP-035`; none returns a made-up value.
* **No write to the model.** The design is given as shared references that the evaluator cannot change, and an assignment to a member of an object is `PHOENIX-ULP-038`. The programs that edit the design as one reversible step (the third step of the record 0011) are not written.
* **No claim of equivalence.** The behaviour was written from the public description of the language and tested with programs written here. It differs on purpose in the whole numbers (64 bits), in the unit of length (the nanometre of the model) and in the members of the objects (the list above); it has not been compared with the original tool.

## What is not written

* **The sandbox of the fourth step of the record 0011: a separate low-privilege process.** The evaluator runs in the process of the caller, confined by what it can do and by its limits, and not by the operating system; `phoenix ulp run` is the only way to run a program, it runs the one program that the person named, and the window runs none. The exact proof that is pending (decision D-18): tests that start the evaluator as a child process with low privileges and try, from a program, to read a user folder, to start a program, to open a socket and to exceed each limit of time, memory and operations, and that the child is stopped without taking the parent with it; and the log with the hash of the program, the parameters and the changes. Until they pass, nothing here is called a sandbox.
* The programs that edit the design as one reversible step (`docs/SCRIPT_API.md` does this for the second script language), the functions that need files, the clock or a window, and the comparison with the original tool, which needs the programs that are really used. Until then the list of names is a first list.

## Codes

`PHOENIX-ULP-001` to `PHOENIX-ULP-041` (`docs/ERROR_CODES.md`): 001 to 003, 018 and 024 belong to the settings, the command, the files and the list of names; 004 to 007 are the limits of the reader; 008 to 017 are what the reader finds in a program; 019 to 023 are the notes of the report; 025 to 028 are the limits of a run, 029 to 038 and 040 are the errors of a run, 039 is a program with problems of reading and 041 is a verification that did not pass. The titles are in the eight languages of the suite.

## Examples

`examples/ulp/` holds programs written for this project, each with the output that it gives (the file `.expected` next to it). The tests run every one and compare the bytes; you can do the same by hand:

```
phoenix ulp run examples/ulp/math_table.ulp --expect examples/ulp/math_table.expected
phoenix ulp run --board examples/tutorials/tut.brd examples/ulp/parts_list.ulp
phoenix ulp report examples/ulp/refused_system.ulp
```

## How it was made

The names and the meanings of the language were taken from its public description, and nothing of it is copied (`docs/adr/0006-clean-room.md`); no code under a copyleft licence was read to write the reader or the evaluator. The tests use small programs written for them that exercise the same constructs, a board and a schematic built in the test, and generators with a fixed seed: random pieces of the language, a good program with characters changed, deleted and added, the same program cut at every point, bytes that are not text, programs nested a hundred thousand levels deep, and programs made of good pieces that loop and grow. Every one must end in a result, a problem, a refusal or an error with its code, inside the budget of steps, memory and output, and never in a panic.
