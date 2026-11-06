# =============================================================================
# EH3D-PHOENIX-PCB - tools/i18n_audit.py
# Copyright (C) 2026 JuanenRac (Electro Hobby 3D) <electrohobby3d@gmail.com>
# GPL-3.0-or-later - see LICENSE
# =============================================================================
"""Audit of the eight languages: every table of texts in the Rust sources and every translated document.

The sources keep their texts in rows of eight strings (en, es, de, fr, it, ja, zh, pt). This module finds
every such row, whatever the table around it, and checks the rows against each other; it also checks the
documents that exist once per language (README, tutorials, glossary, migration, the diagnostic table).
`tools/test_i18n.py` runs the checks; `python -I tools/i18n_audit.py` prints the findings.
"""
from __future__ import annotations

import functools
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LANGS = ("en", "es", "de", "fr", "it", "ja", "zh", "pt")

# A text longer than this in a short field (a button, a bar, a label) is reported as a warning only.
SHORT_FIELD_LIMIT = int(os.environ.get("PHOENIX_I18N_LABEL_LIMIT", "48"))
# A row whose English text is at most this long counts as a label of the window.
LABEL_ENGLISH_LIMIT = 24

# Words of Brazilian Portuguese that European Portuguese does not use, with the European word. "baixar" is
# fine in the sense of lowering (a priority, a value), so a row whose English text lowers something is skipped.
BRAZILIAN = {
    "arquivo": "ficheiro",
    "arquivos": "ficheiros",
    "tela": "ecrã",
    "telas": "ecrãs",
    "usuário": "utilizador",
    "usuários": "utilizadores",
    "usuario": "utilizador",
    "celular": "telemóvel",
    "baixar": "transferir",
    "baixe": "transfira",
    "deletar": "apagar",
    "salvar": "guardar",
    "salve": "guarde",
    "mouse(?! bites)": "rato",
    "você": "o utilizador",
    "vocês": "os utilizadores",
    "gerenciar": "gerir",
    "gerenciamento": "gestão",
    "ônibus": "autocarro",
    "equipe": "equipa",
    "aplicativo": "aplicação",
    "aplicativos": "aplicações",
    "contato": "contacto",
    "planilha": "folha de cálculo",
    "digitar": "escrever",
    "digite": "escreva",
    "cadastro": "registo",
    "cadastrar": "registar",
    "estou fazendo": "estou a fazer",
    "sendo feito": "a ser feito",
}
# The senses in which the English text makes "baixar" correct in European Portuguese too.
LOWERING = re.compile(r"\b(lower|decrease|reduc|down|drop|sink|bring)", re.I)

# Strings that are the same in every language on purpose: names, units, codes, formats, commands.
SAME_EVERYWHERE = re.compile(
    r"""^(?:
        [\W\d_]*                                  # only symbols and digits
        |[A-Za-z0-9_.+\-/\\:*#%{}\[\]()<>=,;'"@&|^~ ]{0,3}      # a very short token
        |(?:PHOENIX|EH3D|EAGLE|KiCad|Gerber|Excellon|DXF|SPICE|Touchstone|IPC)[\w .\-/+]*
        |-{1,2}[a-z][\w\-]*(?:[ =]\S+)*           # an option of the command line
        |phoenix(?:\s+\S+)*                       # a command line
        |[\w.\-/]+\.(?:rs|json|toml|md|txt|csv|svg|png|kicad_pcb|kicad_sch|brd|sch|lbr|net|cir|s\d+p|gbr|drl|dxf|zip)
        |[A-Z][A-Z0-9_ /\-+.]{1,24}               # an abbreviation or a unit in capitals
    )$""",
    re.X,
)

# Words that tell the Latin-script languages apart; a text of eight words or more must look more like its own
# language than like any other one (a column of a table that holds another language is a mistake).
STOPWORDS = {
    "en": "the of and to is not for with that this are be or on in a an it by from no was has have can".split(),
    "es": "el la los las de del y que para con no es una un por se su al lo como más o en".split(),
    "de": "der die das und ist nicht für mit den dem ein eine einer von zu im auf oder wird sind als bei".split(),
    "fr": "le la les des du et est pas pour avec une un que qui dans sur ou au aux ne par sont ce".split(),
    "it": "il lo la le gli dei del della e è non per con una un che di da in su o si sono come al".split(),
    "pt": "o a os as de do da dos das e é não para com uma um que em no na nos nas por se ao como ou".split(),
}
# Only the words that belong to one language count.
STOPWORDS = {
    lang: [w for w in words if sum(w in other for other in STOPWORDS.values()) == 1] for lang, words in STOPWORDS.items()
}
CJK = re.compile(r"[\u3400-\u9fff]")
KANA = re.compile(r"[\u3040-\u30ff]")
CONTROL = re.compile(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f-\x9f]")
PLACEHOLDER = re.compile(r"\{[A-Za-z0-9_]*(?::[^{}]*)?\}|%[-+0#]*\d*(?:\.\d+)?[sdfgxXeEu]|%%")
NUMBER = re.compile(r"(?<![A-Za-z0-9_.])\d+(?:[.,]\d+)*(?![A-Za-z0-9])")
COMMAND = re.compile(r"`([^`\n]+)`")
OPTION = re.compile(r"(?<![\w-])--[a-z][a-z0-9\-]*")
CODE = re.compile(r"\b(?:PHOENIX|EH3D)-[A-Z0-9]+-\d{3}\b")
PATH = re.compile(r"\b(?:docs|crates|tools|tests|examples|config)/[\w./\-]+")


@dataclass
class Row:
    """A row of eight texts found in a source file."""

    file: str
    line: int
    key: str
    texts: list[str]
    code: str = ""

    def where(self) -> str:
        return f"{self.file}:{self.line} {self.key}"


@dataclass
class Finding:
    """A problem; `severity` is `error` or `warning`."""

    rule: str
    severity: str
    where: str
    message: str

    def __str__(self) -> str:
        return f"{self.severity.upper():7} {self.rule:14} {self.where}: {self.message}"


# --------------------------------------------------------------------------- reading the Rust sources
def _unescape(body: str) -> str:
    """The value of the body of a string literal of Rust."""
    out: list[str] = []
    i = 0
    while i < len(body):
        c = body[i]
        if c != "\\":
            out.append(c)
            i += 1
            continue
        i += 1
        if i >= len(body):
            break
        e = body[i]
        i += 1
        if e == "n":
            out.append("\n")
        elif e == "t":
            out.append("\t")
        elif e == "r":
            out.append("\r")
        elif e == "0":
            out.append("\0")
        elif e in "\\\"'":
            out.append(e)
        elif e == "x":
            out.append(chr(int(body[i : i + 2], 16)))
            i += 2
        elif e == "u":
            end = body.index("}", i)
            out.append(chr(int(body[i + 1 : end], 16)))
            i = end + 1
        elif e == "\n" or e == "\r":
            while i < len(body) and body[i] in " \t\r\n":
                i += 1
        else:
            out.append("\\" + e)
    return "".join(out)


def tokens(source: str) -> list[tuple[str, str, int]]:
    """Strings, punctuation and words of a Rust source, without comments: (kind, value, line)."""
    result: list[tuple[str, str, int]] = []
    i, n, line = 0, len(source), 1
    while i < n:
        c = source[i]
        if c == "\n":
            line += 1
            i += 1
        elif c.isspace():
            i += 1
        elif source.startswith("//", i):
            while i < n and source[i] != "\n":
                i += 1
        elif source.startswith("/*", i):
            depth = 1
            i += 2
            while i < n and depth:
                if source.startswith("/*", i):
                    depth += 1
                    i += 2
                elif source.startswith("*/", i):
                    depth -= 1
                    i += 2
                else:
                    if source[i] == "\n":
                        line += 1
                    i += 1
        elif c == "r" and re.match(r'r#*"', source[i:]):
            hashes = re.match(r"r(#*)\"", source[i:]).group(1)
            start = i + 2 + len(hashes)
            end = source.index('"' + hashes, start)
            result.append(("str", source[start:end], line))
            line += source.count("\n", i, end)
            i = end + 1 + len(hashes)
        elif c == "b" and source.startswith('b"', i):
            end = i + 2
            while source[end] != '"':
                end += 2 if source[end] == "\\" else 1
            line += source.count("\n", i, end)
            i = end + 1
        elif c == '"':
            end = i + 1
            while source[end] != '"':
                end += 2 if source[end] == "\\" else 1
            result.append(("str", _unescape(source[i + 1 : end]), line))
            line += source.count("\n", i, end)
            i = end + 1
        elif c == "'":
            m = re.match(r"'(?:\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)|[^\\'])'", source[i:])
            if m:
                i += m.end()
            else:
                i += 1
        elif c.isalnum() or c == "_":
            m = re.match(r"[A-Za-z0-9_]+", source[i:])
            result.append(("word", m.group(0), line))
            i += m.end()
        else:
            result.append(("punct", c, line))
            i += 1
    return result


def rows_of(path: Path) -> list[Row]:
    """Every `[ "..", ".." x8 ]` of a Rust file, with the key that goes before it."""
    source = path.read_text(encoding="utf-8")
    toks = tokens(source)
    rel = path.relative_to(ROOT).as_posix()
    found: list[Row] = []
    # The constants that hold a diagnostic code, to name the rows that are keyed by one.
    codes: dict[str, str] = {}
    for n in range(len(toks) - 6):
        if toks[n][1] == "const" and toks[n + 2][1] == ":" and toks[n + 3][1] == "&" and toks[n + 4][1] == "str" and toks[n + 5][1] == "=" and toks[n + 6][0] == "str":
            if CODE.fullmatch(toks[n + 6][1]):
                codes[toks[n + 1][1]] = toks[n + 6][1]
    # Rows after the module of tests of a file are examples for the tests, not texts of the program.
    limit = 10**9
    for n in range(len(toks) - 6):
        if toks[n][1] == "#" and toks[n + 1][1] == "[" and toks[n + 2][1] == "cfg" and toks[n + 4][1] == "test" and toks[n + 6][1] == "]" and toks[n + 7][1] == "mod":
            limit = toks[n][2]
            break
    for i, (kind, value, line) in enumerate(toks):
        if kind != "punct" or value != "[" or line >= limit:
            continue
        texts: list[str] = []
        j = i + 1
        while j < len(toks) and toks[j][0] == "str":
            texts.append(toks[j][1])
            j += 1
            if j < len(toks) and toks[j] == ("punct", ",", toks[j][2]):
                j += 1
            else:
                break
        if len(texts) != 8 or j >= len(toks) or toks[j][1] != "]":
            continue
        key = ""
        k = i - 1
        # The key is the string or the word before the comma that precedes the bracket, or the field name.
        if k >= 0 and toks[k][1] == ",":
            k -= 1
            if k >= 0 and toks[k][0] in ("str", "word"):
                key = toks[k][1]
                if toks[k][0] == "word" and k >= 2 and toks[k - 1][1] == ":" and toks[k - 2][1] == ":":
                    key = toks[k - 3][1] + "::" + key
        elif k >= 0 and toks[k][1] == ":" and k >= 1:
            key = toks[k - 1][1]
        elif k >= 0 and toks[k][1] == "=":
            back = k - 1
            while back >= 0 and k - back < 14 and toks[back][1] != ":":
                back -= 1
            key = toks[back - 1][1] if back >= 1 and toks[back][1] == ":" else ""
        elif k >= 0 and toks[k][1] == ">" and k >= 1 and toks[k - 1][1] == "=":
            key = "arm " + (toks[k - 2][1] if k >= 2 else "")
        found.append(Row(rel, line, key or "?", texts, key if CODE.fullmatch(key) else codes.get(key, "")))
    found.extend(arms_of(toks, rel, limit))
    return found


ARM_NAMES = ("En", "Es", "De", "Fr", "It", "Ja", "Zh", "Pt")


def _arm_end(toks: list[tuple[str, str, int]], start: int) -> int:
    """The index after the expression of an arm that starts at `start`: the comma at depth zero or the closing brace."""
    depth = 0
    for n in range(start, len(toks)):
        value = toks[n][1] if toks[n][0] == "punct" else ""
        if value in "([{" and value:
            depth += 1
        elif value in ")]}" and value:
            if depth == 0:
                return n
            depth -= 1
        elif value == "," and depth == 0:
            return n
    return len(toks)


def arms_of(toks: list[tuple[str, str, int]], rel: str, limit: int) -> list[Row]:
    """The texts written as eight arms of a `match` on the language: `Lang::En => format!("..")` gives one row,
    and `Lang::En => Words { field: "..", .. }` gives one row for each field."""
    starts: list[int] = []
    for n in range(len(toks) - 6):
        if toks[n][1] in ("Lang", "Language") and toks[n + 1][1] == ":" and toks[n + 2][1] == ":" and toks[n + 3][1] in ARM_NAMES and toks[n + 4][1] == "=" and toks[n + 5][1] == ">" and toks[n][2] < limit:
            starts.append(n)
    rows: list[Row] = []
    for k in range(len(starts) - 7):
        group = starts[k : k + 8]
        if tuple(toks[g + 3][1] for g in group) != ARM_NAMES:
            continue
        ends = group[1:] + [_arm_end(toks, group[-1] + 6)]
        bodies = [toks[g + 6 : e] for g, e in zip(group, ends)]
        line = toks[group[0]][2]
        fields = [[(body[i][1], body[i + 2][1]) for i in range(len(body) - 2) if body[i][0] == "word" and body[i + 1][1] == ":" and body[i + 2][0] == "str"] for body in bodies]
        if all(len(f) >= 2 for f in fields) and len({tuple(name for name, _ in f) for f in fields}) == 1:
            for index, (name, _) in enumerate(fields[0]):
                rows.append(Row(rel, line, f"field {name}", [f[index][1] for f in fields]))
            continue
        texts = [next((t[1] for t in body[:6] if t[0] == "str"), None) for body in bodies]
        if all(t is not None for t in texts):
            rows.append(Row(rel, line, "match", [t for t in texts if t is not None]))
    return rows


def source_files() -> list[Path]:
    return sorted((ROOT / "crates").rglob("*.rs"))


@functools.cache
def all_rows() -> tuple[Row, ...]:
    rows: list[Row] = []
    for path in source_files():
        name = path.as_posix()
        if "/target/" in name or "/tests/" in name or name.endswith("_tests.rs") or path.name == "tests.rs":
            continue
        rows.extend(rows_of(path))
    return tuple(rows)


# --------------------------------------------------------------------------- the checks of one row
def _placeholders(text: str) -> list[str]:
    return sorted(PLACEHOLDER.findall(text))


def _signature(text: str) -> tuple[list[str], list[str], list[str], list[str]]:
    """What must be the same in every language: placeholders, codes, options and paths."""
    return (_placeholders(text), sorted(CODE.findall(text)), sorted(OPTION.findall(text)), sorted(PATH.findall(text)))


def _numbers(text: str) -> list[str]:
    text = PLACEHOLDER.sub(" ", text)
    text = CODE.sub(" ", text)
    return sorted(m.replace(",", ".") for m in NUMBER.findall(text))


def _word_in(text: str, word: str) -> bool:
    return re.search(r"(?<![\wÀ-ÿ])" + word + r"(?![\wÀ-ÿ])", text, re.I) is not None


def check_row(row: Row, exceptions: set[str] | None = None) -> list[Finding]:
    """The findings of one row of eight texts."""
    out: list[Finding] = []
    exceptions = exceptions or set()
    en = row.texts[0]
    if all(t.startswith("-") for t in row.texts):  # a table of options of the command line, not of texts
        return out
    for lang, text in zip(LANGS, row.texts):
        if text.strip() == "":
            out.append(Finding("empty", "error", row.where(), f"empty text in {lang}"))
        if CONTROL.search(text):
            out.append(Finding("control", "error", row.where(), f"control character in {lang}"))
        if "�" in text:
            out.append(Finding("replacement", "error", row.where(), f"U+FFFD in {lang}"))
        if text != text.strip() and text.strip():
            if (text.startswith(" ") or text.endswith(" ")) != (en.startswith(" ") or en.endswith(" ")):
                out.append(Finding("spaces", "warning", row.where(), f"edge spaces differ in {lang}"))
    # (2) identical to English: an error for a phrase, a warning for one short word that is a cognate
    if en.strip() and en.strip() not in exceptions and not SAME_EVERYWHERE.match(en.strip()):
        same = [lang for lang, text in zip(LANGS[1:], row.texts[1:]) if text == en]
        short = len(en.split()) <= 2 and len(en) <= 16
        if same and (not short or len(same) >= 5):
            out.append(Finding("same-as-en", "error", row.where(), f"identical to English in {', '.join(same)}: {en[:50]!r}"))
        elif same:
            out.append(Finding("same-as-en", "warning", row.where(), f"identical to English in {', '.join(same)}: {en[:50]!r}"))
    # (3) the same placeholders, codes, options and paths; the same numbers
    base = _signature(en)
    base_numbers = _numbers(en)
    for lang, text in zip(LANGS[1:], row.texts[1:]):
        sig = _signature(text)
        for label, a, b in zip(("placeholders", "codes", "options", "paths"), base, sig):
            if a != b:
                out.append(Finding("marker", "error", row.where(), f"{label} differ in {lang}: {a} against {b}"))
        # A number of the English text must be in the translation; a translation may write as a digit what English spells out.
        lost = [n for n in base_numbers if n not in _numbers(text)]
        if lost:
            out.append(Finding("numbers", "warning", row.where(), f"numbers {lost} of the English text are not in {lang}"))
    # (4) European Portuguese
    pt = row.texts[7]
    for word, fix in BRAZILIAN.items():
        if word.startswith("baixar") and LOWERING.search(en):
            continue
        if _word_in(pt, word):
            out.append(Finding("pt-brazil", "error", row.where(), f"{word!r} is Brazilian; use {fix!r}"))
    # (7) the text is in the language of its column
    for lang, text in zip(LANGS, row.texts):
        if SAME_EVERYWHERE.match(text.strip()) or " " not in en.strip():
            continue
        if lang == "ja" and not (KANA.search(text) or CJK.search(text)):
            out.append(Finding("language", "warning", row.where(), f"the text in ja has no Japanese character: {text[:40]!r}"))
        elif lang == "zh" and (not CJK.search(text) or KANA.search(text)):
            out.append(Finding("language", "error", row.where(), f"the text in zh is not Chinese: {text[:40]!r}"))
        elif lang not in ("ja", "zh") and CJK.search(text):
            out.append(Finding("language", "error", row.where(), f"the text in {lang} has Chinese or Japanese characters: {text[:40]!r}"))
        elif lang in STOPWORDS:
            words = re.findall(r"[^\W\d_]+", text.lower())
            if len(words) >= 8:
                scores = {k: sum(w in v for w in words) for k, v in STOPWORDS.items()}
                best = max(scores, key=lambda k: scores[k])
                if scores[best] > scores[lang] + 1 and scores[best] >= 3:
                    out.append(Finding("language", "warning", row.where(), f"the text in {lang} reads as {best}: {text[:50]!r}"))
    # (6) length of a label
    if len(en) <= LABEL_ENGLISH_LIMIT:
        for lang, text in zip(LANGS, row.texts):
            width = sum(2 if ord(ch) > 0x2E80 else 1 for ch in text)
            if width > SHORT_FIELD_LIMIT:
                out.append(Finding("long-label", "warning", row.where(), f"{lang} label is {width} columns wide"))
    return out


def check_rows(rows: "list[Row] | tuple[Row, ...]", exceptions: set[str] | None = None) -> list[Finding]:
    out: list[Finding] = []
    for row in rows:
        out.extend(check_row(row, exceptions))
    return out


# --------------------------------------------------------------------------- the documents
def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def check_text_of_document(rel: str, text: str, lang: str) -> list[Finding]:
    out: list[Finding] = []
    if CONTROL.search(text.replace("\t", " ")):
        out.append(Finding("control", "error", rel, "control character"))
    if "�" in text:
        out.append(Finding("replacement", "error", rel, "U+FFFD"))
    if lang == "pt":
        stripped = re.sub(r"`[^`\n]*`", " ", text)
        stripped = re.sub(r"\(https?://[^)]*\)|\]\([^)]*\)", " ", stripped)
        for word, fix in BRAZILIAN.items():
            if _word_in(stripped, word):
                out.append(Finding("pt-brazil", "error", rel, f"{word!r} is Brazilian; use {fix!r}"))
    return out


def structure_of(text: str) -> dict[str, list[str]]:
    """What a translation must keep from its English original: headings count, code blocks, commands, codes, options."""
    body = re.sub(r"```.*?```", lambda m: m.group(0), text, flags=re.S)
    return {
        "headings": [h.split(" ")[0] for h in re.findall(r"(?m)^(#{1,6} )", body)],
        "fences": re.findall(r"(?m)^```", body),
        "codes": sorted(set(CODE.findall(text))),
        "options": sorted(set(OPTION.findall(text))),
        "commands": sorted(set(re.findall(r"`(phoenix [^`\n]+)`", text))),
    }


def _features(text: str) -> dict[str, int]:
    lines = text.splitlines()
    return {
        "rows": len([ln for ln in lines if ln.startswith("|")]),
        "lines": len([ln for ln in lines if ln.strip()]),
        "bullets": len([ln for ln in lines if re.match(r"\s*(?:[-*]|\d+\.) ", ln)]),
        "links": len(re.findall(r"\]\(", text)),
    }


def _spans(text: str) -> set[str]:
    """The code spans of a page without the words in capitals that stand for a value (FILE, FICHERO) and the `<...>` parts."""
    spans = set()
    for span in re.findall(r"`([^`\n]+)`", text):
        span = re.sub(r"<[^>]*>|\b[A-Z]{3,}\b|[^\x00-\x7f]+", "", span)
        spans.add(re.sub(r"\s+", " ", span).strip())
    return spans


def _digits(text: str) -> list[str]:
    """The numbers of a page outside the code spans, with the decimal comma of the Latin languages read as a point."""
    text = re.sub(r"`[^`]*`", " ", text)
    return sorted(m.replace(",", ".") for m in re.findall(r"(?<![A-Za-z0-9_.,])\d+(?:[.,]\d+)*(?![A-Za-z0-9_])", text))


def document_pairs() -> list[tuple[str, str, str]]:
    """(english file, translated file, language) for every document that exists in eight languages."""
    pairs: list[tuple[str, str, str]] = []
    readme = {"es": "README_spa.md", "de": "README_deu.md", "fr": "README_fra.md", "it": "README_ita.md", "ja": "README_jpn.md", "zh": "README_zho.md", "pt": "README_por.md"}
    for lang, name in readme.items():
        pairs.append(("README.md", name, lang))
    for folder, english in (("docs/tutorials", "docs/tutorials"), ("docs/glossary", "docs"), ("docs/migration", "docs")):
        for lang in LANGS[1:]:
            base = ROOT / folder / lang
            if not base.is_dir():
                continue
            for path in sorted(base.glob("*.md")):
                en_path = ROOT / english / path.name
                if folder == "docs/tutorials" and path.name == "README.md":
                    en_path = ROOT / "docs/tutorials/README.md"
                pairs.append((en_path.relative_to(ROOT).as_posix(), path.relative_to(ROOT).as_posix(), lang))
    return pairs


def check_documents() -> list[Finding]:
    out: list[Finding] = []
    for en_rel, tr_rel, lang in document_pairs():
        en_path, tr_path = ROOT / en_rel, ROOT / tr_rel
        if not en_path.is_file():
            out.append(Finding("original", "error", tr_rel, f"the original {en_rel} does not exist"))
            continue
        en_text, tr_text = _read(en_path), _read(tr_path)
        out.extend(check_text_of_document(tr_rel, tr_text, lang))
        # The English page ends with the list of languages; the translations of the glossary do not repeat the table of terms.
        en_main = re.split(r"(?m)^## (?:Languages|The terms in the eight languages)\s*$", en_text)[0]
        a, b = structure_of(en_main), structure_of(tr_text)
        glossary = "glossary" in tr_rel
        for field in ("headings", "fences"):
            if glossary and field == "headings":
                continue
            if len(a[field]) != len(b[field]):
                out.append(Finding("structure", "error", tr_rel, f"{field}: {len(a[field])} in the original, {len(b[field])} here"))
        for field in ("codes", "options"):
            missing = sorted(set(a[field]) - set(b[field]))
            extra = sorted(set(b[field]) - set(a[field]))
            if missing:
                out.append(Finding("structure", "error", tr_rel, f"{field} missing: {missing[:6]}"))
            if extra:
                out.append(Finding("structure", "error", tr_rel, f"{field} not in the original: {extra[:6]}"))
        heads = lambda cmds: {" ".join(c.split()[:2]) for c in cmds}  # noqa: E731 - the metavariables are translated
        missing = sorted(heads(a["commands"]) - heads(b["commands"]))
        if missing:
            out.append(Finding("structure", "warning", tr_rel, f"commands missing: {missing[:4]}"))
        # The tables, the lines and the code spans (with the words that stand for a value left out) must be the same.
        feats_en, feats_tr = _features(en_main), _features(tr_text)
        for field in ("rows", "lines", "bullets", "links"):
            gap = feats_tr[field] - feats_en[field]
            if gap != 0 and not (gap == 1 and field in ("lines", "bullets", "links")):  # a translation links back to the English page
                out.append(Finding("structure", "error", tr_rel, f"{field}: {feats_en[field]} in the original, {feats_tr[field]} here"))
        spans_en = _spans(en_main)
        spans_tr = _spans(tr_text)
        lost_spans = sorted(spans_en - spans_tr)
        if lost_spans:
            out.append(Finding("structure", "error", tr_rel, f"code spans of the original that are missing: {lost_spans[:4]}"))
        digits_en = _digits(en_main)
        digits_tr = _digits(tr_text)
        lost_digits = sorted(n for n in set(digits_en) if digits_en.count(n) > digits_tr.count(n))
        if lost_digits:
            out.append(Finding("numbers", "error", tr_rel, f"numbers of the original that are missing: {lost_digits[:6]}"))
        outside = lambda t: re.sub(r"(?ms)^```.*?^```", "", t)  # noqa: E731 - commands and examples are not translated
        en_lines = outside(en_text).splitlines()
        same = [ln for ln in outside(tr_text).splitlines() if len(ln) > 60 and ln in en_lines and not ln.lstrip().startswith(("|", "```", "![", "[", "<", "-->"))]
        prose = [ln for ln in same if re.search(r"[A-Za-z]{4,} [a-z]{3,} [a-z]{3,} [a-z]{3,}", ln) and not re.search(r"[{};=]", ln)]
        if len(prose) > 2:
            out.append(Finding("untranslated", "error", tr_rel, f"{len(prose)} lines of prose identical to English, first: {prose[0][:60]!r}"))
    return out


def diagnostic_table() -> dict[str, list[list[str]]]:
    """code -> the rows of docs/ERROR_CODES.md that name it: the cells after the code (the English title, then the
    other texts of the row). A code can have one row in the table of its crate and one in the table of languages."""
    table: dict[str, list[list[str]]] = {}
    for line in _read(ROOT / "docs/ERROR_CODES.md").splitlines():
        m = re.match(r"\|\s*((?:PHOENIX|EH3D)-[A-Z0-9]+-\d{3})\s*\|(.*)\|\s*$", line)
        if m:
            table.setdefault(m.group(1), []).append([c.strip() for c in m.group(2).split("|")])
    return table


def check_code_tables(rows: "list[Row] | tuple[Row, ...]") -> list[Finding]:
    """Every code of a table of the sources: the table of the documents has it with the same English title, the rows
    of the document that carry the eight languages carry the same texts, and no code is defined twice."""
    out: list[Finding] = []
    table = diagnostic_table()
    seen: dict[str, str] = {}
    for row in rows:
        if not row.code:
            continue
        if row.code in seen and seen[row.code] != row.where():
            out.append(Finding("code-twice", "error", row.where(), f"{row.code} also at {seen[row.code]}"))
        seen.setdefault(row.code, row.where())
        found = table.get(row.code)
        if found is None:
            out.append(Finding("code-doc", "error", row.where(), f"{row.code} is not in docs/ERROR_CODES.md"))
            continue
        if not any(cells[0] == row.texts[0] for cells in found):
            out.append(Finding("code-doc", "error", row.where(), f"{row.code}: the English title is not the one of docs/ERROR_CODES.md ({found[0][0]!r})"))
        for cells in found:
            if len(cells) >= 8:
                for lang, here, there in zip(LANGS, row.texts, cells):
                    if here != there:
                        out.append(Finding("code-doc", "error", row.where(), f"{row.code}: the text in {lang} differs from docs/ERROR_CODES.md"))
    return out


def glossary_rows() -> list[Row]:
    """The rows of the table of terms in the eight languages of docs/GLOSSARY.md."""
    rows: list[Row] = []
    lines = _read(ROOT / "docs/GLOSSARY.md").splitlines()
    for number, line in enumerate(lines, 1):
        if not line.startswith("|") or line.startswith("|---") or line.startswith("| en "):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) == 8 and number > next(n for n, ln in enumerate(lines, 1) if ln.startswith("| en |")):
            rows.append(Row("docs/GLOSSARY.md", number, "term", cells))
    return rows


def run_all() -> list[Finding]:
    rows = all_rows()
    findings = check_rows(rows, load_exceptions())
    findings += check_rows(glossary_rows(), load_exceptions())
    findings += check_code_tables(rows)
    findings += check_documents()
    return findings


def load_exceptions() -> set[str]:
    """Keys of rows that are identical to English on purpose (proper names)."""
    path = ROOT / "tools" / "i18n_exceptions.txt"
    if not path.is_file():
        return set()
    return {ln.strip() for ln in _read(path).splitlines() if ln.strip() and not ln.startswith("#")}


def main(argv: list[str]) -> int:
    """`--limit N` sets the width of a label that is reported, `--errors` hides the warnings, `--strict` makes them fail too."""
    global SHORT_FIELD_LIMIT
    if "--limit" in argv:
        SHORT_FIELD_LIMIT = int(argv[argv.index("--limit") + 1])
    findings = run_all()
    errors = [f for f in findings if f.severity == "error"]
    for f in findings:
        if f.severity == "error" or "--errors" not in argv:
            print(f)
    print(f"{len(all_rows())} rows, {len(errors)} errors, {len(findings) - len(errors)} warnings")
    return 1 if errors or ("--strict" in argv and findings) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
