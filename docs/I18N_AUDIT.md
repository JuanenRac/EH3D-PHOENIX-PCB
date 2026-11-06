# The eight languages: what is checked, what was found and what is left

Every text a person reads exists in eight languages: English, Spanish, German, French, Italian, Japanese, Chinese and the Portuguese of Portugal. `tools/i18n_audit.py` finds every place that holds such texts and checks them against each other; `tools/test_i18n.py` runs the checks in the tests of the base, so a missing or wrong translation stops the CI.

```
python -I tools/i18n_audit.py                 # every finding, errors and warnings
python -I tools/i18n_audit.py --errors        # only the errors
python -I tools/i18n_audit.py --strict        # warnings fail too
python -I tools/i18n_audit.py --limit 40      # width of a label that is reported
```

## What is looked at

| Place | How it is found |
|---|---|
| The tables of titles and texts in the Rust sources (diagnostic codes, the help, the window, the reports) | every row of eight strings, in any table, and every `match` on the language with eight arms (also the ones that fill a structure field by field); the tests and the code after a test module are left out |
| The table of diagnostic codes | `docs/ERROR_CODES.md` has each code of the sources with the same English title, and the rows that carry the eight languages carry the same texts |
| The README of each language | `README_spa.md`, `README_fra.md`, `README_ita.md`, `README_deu.md`, `README_jpn.md`, `README_zho.md`, `README_por.md` against `README.md` |
| The tutorials, the glossary and the migration guide | `docs/tutorials/<lang>/`, `docs/glossary/<lang>/`, `docs/migration/<lang>/` against the English page |
| The table of terms | the rows of `docs/GLOSSARY.md` that give a term in the eight languages |

## What is checked

1. **Parity.** Every row has eight texts, none empty; every code of the sources is in the table of codes; every translated page exists for the seven languages and follows its original (the same headings, the same code blocks, tables with the same rows, the same lines and links apart from the link back to the English page).
2. **Not left in English.** A phrase identical to the English one in any language is an error; a single word that is a cognate (`Total`, `Name`, `Status`) is only a warning; names, abbreviations and units are on the list of `tools/i18n_exceptions.txt`.
3. **Markers.** The same `{n}`, `{}`, `%s` and `%d`, the same diagnostic codes, the same `--options` and the same paths in every language; the numbers of the English text are in every translation (the decimal comma of the Latin languages is read as a point); in a page the code spans and the numbers outside them are the same.
4. **The Portuguese of Portugal.** A list of Brazilian words with the European word (`arquivo` for `ficheiro`, `tela` for `ecrã`, `usuário` for `utilizador`, `celular` for `telemóvel`, `baixar` for `transferir`, `deletar` for `apagar`, `salvar` for `guardar` and others); `baixar` is accepted when the English text lowers something (a priority).
5. **Clean text.** No control character and no U+FFFD in a text or in a page.
6. **Width.** A label of at most 24 characters in English that is wider than 48 columns in any language (a wide Japanese or Chinese character counts as two) is a warning; the limit is set with `--limit` or the variable `PHOENIX_I18N_LABEL_LIMIT`.
7. **The language of the column.** A Chinese column with kana or without Chinese characters, a Latin column with Chinese characters, and a long text that reads as another Latin language (by the words that belong to one language only) are reported.

## What was found, and what was changed

| Where | Finding | Change |
|---|---|---|
| `README_spa.md`, `README_fra.md`, `README_ita.md`, `README_deu.md`, `README_jpn.md`, `README_zho.md`, `README_por.md` | the row "Planned function" of the project status did not carry the sentence about the bill of materials (`phoenix bom`) and the 3D assembly (`phoenix assembly3d`) that the English README has | the sentence was translated and put in each README |
| `README_por.md` | `arquivo` and `arquivos` for the archive of a release | `pacote comprimido` and `pacotes comprimidos` |
| `crates/eh3d-phoenix-eagle-deep/src/codes.rs` and `docs/ERROR_CODES.md` (`PHOENIX-EAGLEX-016`) | the Portuguese title said `tabela padrão` for the standard table (`padrão` is the Brazilian word for a default) | `tabela normalizada` in both |
| Counters of tests in the eight READMEs | the badge and the line "At a glance" kept a count from the first days of the repository | both give the number of tests of the code now |

The checker found nothing else wrong in the titles of the diagnostic codes of the sources (eight texts each, the same markers, the same title in the table of codes), in the terms of the glossary, in the translated pages (the tutorials and their index, the glossary, the migration guide and the READMEs, each in seven languages) or in the tables of the window, the help and the reports.

## What is left

* **Warnings that stay.** About sixty single words that are the same in English and in a Romance or Germanic language on purpose (`Version`, `Status`, `Name`, `Panel`, `Total`, `Cursor`, `Footprint` in German, `Points` in French). They are read and kept; they are not errors.
* **Two labels that are wider than the limit** (`ID_UNAVAILABLE` of the RF generator and of the router, in Italian, 53 columns): they are titles of codes in a report, not buttons, so no change.
* **The checker cannot judge a translation.** It finds a missing, empty, mixed-up or mis-marked text, a Brazilian word and a page that drifted from its original. It cannot tell a clumsy sentence from a good one and it does not check the meaning. A person who reads the language must read the texts that matter (the safety statement, the help) before a release.
* **Pages that exist only in English** are not looked at: `docs/*.md` other than the ones above, the changelog and the code comments.
* **The list of Brazilian words is a list.** A Brazilian word that is not on it passes; add it to `BRAZILIAN` in `tools/i18n_audit.py` when one is found.
