# EH3D-PHOENIX-PCB: the user programs of the reference tool

The crate `eh3d-phoenix-ulp` and the command `phoenix ulp` read the user programs of the reference tool (files with the extension `.ulp`: reports, bills of materials, exports and batch edits) and say what each one uses and whether the list of allowed names covers it. This is the first part of the plan of `docs/adr/0011-ulp-interpreter-for-the-owner-list.md`: the compatibility report and the reader. **Nothing of a program is run**: the evaluator and the sandbox of the plan are not written yet.

**State: prototype.** It has been tried on small programs written for the tests and on text made by a generator with a fixed seed. No program of a third party was used, none has been compared with the original tool, and the list of allowed names is a first list that waits for the programs that are really used.

```
phoenix ulp check  [--config FILE] [--json] [--lang CODE] <program>...
phoenix ulp report [--config FILE] [--support FILE] [--json] [--lang CODE] <program>...
```

The command reads files and writes nothing. `check` reads each program and lists what is wrong with its shape; the exit code is 0 when every program was read without a problem, 1 when some has one and 2 when the arguments or a file are unusable. `report` says what each program uses and what the list says about it; the exit code is 0 when every program is covered, 1 when some is partly or not covered and 2 as above. `--json` prints the result with its keys in a fixed order, the same for the same program; `--lang` chooses one of the eight languages of the suite for the words and the titles of the codes.

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

For each program the report gives its size, its SHA-256, the number of statements and functions, what it uses, the problems of the reading, a note for each name that needs one, and a verdict.

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

The verdict says what the list *would* let through if there were an evaluator. The report says so in its own text (`"evaluator": "not written"` and `"ran": false` in the JSON), and no document of this repository claims that the language as a whole is supported.

## What is not written

* The evaluator: the built-in functions, the objects of the design mapped to the model through reversible operations, a program that edits as one step of the history (`docs/SCRIPT_API.md` does this for the second script language).
* The sandbox: a separate low-privilege process, imported programs disabled until they are confirmed, files only in a private folder with a size limit, limits of time, memory and operations, and a log with the hash of the program, the parameters and the resulting changes.
* The comparison with the original tool, which needs the programs that are really used. Until then the list of names is a first list.

## Codes

`PHOENIX-ULP-001` to `PHOENIX-ULP-024` (`docs/ERROR_CODES.md`): 001 to 003, 018 and 024 belong to the settings, the command, the files and the list of names; 004 to 007 are the limits; 008 to 017 are what the reader finds in a program; 019 to 023 are the notes of the report. The titles are in the eight languages of the suite.

## How it was made

The names and the meanings of the language were taken from its public description, and nothing of it is copied (`docs/adr/0006-clean-room.md`). The tests use small programs written for them that exercise the same constructs, and a generator with a fixed seed that tries random pieces of the language, a good program with characters changed, deleted and added, the same program cut at every point, bytes that are not text and programs nested a hundred thousand levels deep, which must end in a problem or in a refusal and never in a panic.
