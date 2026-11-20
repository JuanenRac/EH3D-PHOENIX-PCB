# 0011: an interpreter of the user programs of the reference tool, only for the programs on the owner's list

Status: accepted as a design; nothing of the interpreter is written. The record only prepares the work (phase F8b of the roadmap) and is pending a review by the project owner (to be confirmed).

## Context

The reference tool has a language for user programs (files with the extension `.ulp`) that reads and, in some cases, changes the design: reports, bills of materials, exports, batch edits. The owner has a list of such programs in use. Today the project has two other ways to automate a design: the command interpreter for script files (`phoenix script`, `docs/SCRIPT.md`) and the second script language (`phoenix-script/2`, `docs/SCRIPT_API.md`), which runs whole script files on a board, a schematic or a library as one reversible step. Neither runs an existing program of the other language.

An interpreter of a whole language is a large and open-ended job: the language has its own types, built-in functions, dialogs and file access, and a promise of "compatible" would be a promise nobody can keep.

## Options

* **Not to interpret the language;** rewrite each program of the owner in the second script language by hand. Safe, but every program is a manual job and nothing is checked.
* **A full interpreter** of the language. Large, never finished, and a risk for security, because the programs can run commands and write files.
* **An interpreter of the subset that the owner's programs use,** with a report that says, for each program, what it uses and whether the interpreter covers it. The scope is decided by the list, not by the language manual.

## Decision

The third option. The interpreter is written only for the programs on the owner's list, in this order:

1. **The compatibility report.** A static pass over a program: which built-in functions, which objects of the design (board, schematic, library), which output and which dialogs it uses, with a verdict per program: `covered`, `partly covered` (and what is missing) or `not covered`. It runs nothing and opens the way to decide what to write first.
2. **The reader.** Lexer, parser and a syntax tree in a crate of its own, with limits on size, depth and nesting like the other readers, and the stable codes `PHOENIX-ULP-<NNN>`.
3. **The evaluator,** with the built-ins that the report showed are needed, and the objects of the language mapped to the design model through reversible operations (a program that edits is one reversible step, as in `phoenix-script/2`).
4. **The sandbox.** Imported programs are disabled by default and run only after explicit confirmation, with an allow list that starts from nothing: read access to the model and pure computation are allowed; output only inside a private per-run folder with a size limit; operating-system commands, network, environment and user folders are denied; limits on time, memory and number of operations; a log with the hash of the program, the parameters and the resulting changes. The sandbox is a separate low-privilege process; a child process alone is not a boundary. The allow list is checked against the reference of the language and is a data file, not code.

A program outside the list is reported as `not covered` and is not run. No claim of compatibility with the language as a whole is made anywhere in the documentation.

## Reasons

* The size of the work is the size of the list, and it can be measured by the report.
* A report is useful before any interpreter exists: it tells which programs are simple to port by hand.
* Security does not depend on the interpreter being correct: the confinement is the process.

## Consequences

* A new crate (`eh3d-phoenix-ulp`) joins the workspace with the first step, with `config/ulp.cfg` for every limit and the text of the report in the eight languages.
* The documentation of the language of the reference tool is used as a reference for names and meanings only; nothing of it is copied into the repository (`docs/adr/0006-clean-room.md`), and no program of another author is stored.
* The programs of the owner are private; tests use small programs written here that exercise the same constructs.
* The requirement for the script interpreter (PPCB-019) keeps its status until the first step exists; this record does not change it.

## How it will be verified

The report is tested on written programs with known constructs; the reader with the corpus of invalid files and the limits; the evaluator by running each covered program and comparing the resulting board with the one the second script language gives for the same job; the sandbox by tests that try to read a user folder, start a program, open a socket and exceed each limit. Review date: when the owner's list is available.
