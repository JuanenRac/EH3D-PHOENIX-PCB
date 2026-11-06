# =============================================================================
# EH3D-PHOENIX-PCB - tools/test_i18n.py
# Copyright (C) 2026 JuanenRac (Electro Hobby 3D) <electrohobby3d@gmail.com>
# GPL-3.0-or-later - see LICENSE
# =============================================================================
"""The eight languages: the tables of texts in the Rust sources and the translated documents agree with each other.

The checks live in `i18n_audit.py`; the first group below proves that each check catches what it is for, on rows
written here, and the second group runs them on the real tables and documents of the repository.
"""
from __future__ import annotations

import unittest

import i18n_audit as audit
from i18n_audit import Row, check_row


def row(*texts: str, key: str = "K") -> Row:
    return Row("file.rs", 1, key, list(texts))


GOOD = row(
    "File {n} is not valid",
    "El fichero {n} no es válido",
    "Die Datei {n} ist ungültig",
    "Le fichier {n} n'est pas valide",
    "Il file {n} non è valido",
    "ファイル {n} は無効です",
    "文件 {n} 无效",
    "O ficheiro {n} não é válido",
)


def with_pt(text: str) -> Row:
    return row(*GOOD.texts[:7], text)


def rules_of(findings: list[audit.Finding]) -> set[str]:
    return {f.rule for f in findings}


class CheckerTests(unittest.TestCase):
    def test_a_good_row_has_no_finding(self) -> None:
        self.assertEqual(check_row(GOOD), [])

    def test_an_empty_text_is_found(self) -> None:
        texts = list(GOOD.texts)
        texts[3] = "  "
        self.assertIn("empty", rules_of(check_row(row(*texts))))

    def test_a_control_character_and_a_replacement_character_are_found(self) -> None:
        texts = list(GOOD.texts)
        texts[2] = "Die Datei {n}\x07 ist ungültig"
        self.assertIn("control", rules_of(check_row(row(*texts))))
        texts[2] = "Die Datei {n} ist ung�ltig"
        self.assertIn("replacement", rules_of(check_row(row(*texts))))

    def test_a_phrase_left_in_english_is_an_error_but_a_cognate_word_is_not(self) -> None:
        texts = list(GOOD.texts)
        texts[1] = texts[0]
        found = check_row(row(*texts))
        self.assertIn(("same-as-en", "error"), {(f.rule, f.severity) for f in found})
        word = row("Total", "Total", "Summe", "Total", "Totale", "合計", "总计", "Total")
        self.assertNotIn("error", {f.severity for f in check_row(word)})
        same_everywhere = row("Hash", "Hash", "Hash", "Hash", "Hash", "Hash", "Hash", "Hash")
        self.assertIn("error", {f.severity for f in check_row(same_everywhere)})
        self.assertEqual(check_row(same_everywhere, {"Hash"}), [])

    def test_a_placeholder_that_changes_is_found(self) -> None:
        texts = list(GOOD.texts)
        texts[5] = "ファイル {m} は無効です"
        self.assertIn("marker", rules_of(check_row(row(*texts))))
        texts[5] = "ファイルは無効です"
        self.assertIn("marker", rules_of(check_row(row(*texts))))

    def test_a_printf_marker_counts_but_a_percent_before_a_word_does_not(self) -> None:
        a = row("Value %d of %s", "Valor %d de %s", "Wert %d von %s", "Valeur %d de %s", "Valore %d di %s", "値 %d / %s", "值 %d / %s", "Valor %d de %s")
        self.assertEqual(check_row(a), [])
        b = row("Equivalent %", "% equivalente", "% gleichwertig", "% équivalents", "% equivalenti", "同等の割合", "等价比例", "% equivalentes")
        self.assertNotIn("marker", rules_of(check_row(b)))

    def test_a_code_an_option_and_a_path_must_be_kept(self) -> None:
        a = row(
            "See PHOENIX-SIM-016 with --lang in docs/ERROR_CODES.md",
            "Véase PHOENIX-SIM-016 con --lang en docs/ERROR_CODES.md",
            "Siehe PHOENIX-SIM-016 mit --lang in docs/ERROR_CODES.md",
            "Voir PHOENIX-SIM-016 avec --lang dans docs/ERROR_CODES.md",
            "Vedi PHOENIX-SIM-016 con --lang in docs/ERROR_CODES.md",
            "PHOENIX-SIM-016 を --lang で docs/ERROR_CODES.md に",
            "见 docs/ERROR_CODES.md 中的 PHOENIX-SIM-016 与 --lang",
            "Ver PHOENIX-SIM-016 com --lang em docs/ERROR_CODES.md",
        )
        self.assertEqual(check_row(a), [])
        texts = list(a.texts)
        texts[2] = texts[2].replace("--lang", "--sprache")
        self.assertIn("marker", rules_of(check_row(row(*texts))))
        texts = list(a.texts)
        texts[3] = texts[3].replace("PHOENIX-SIM-016", "PHOENIX-SIM-017")
        self.assertIn("marker", rules_of(check_row(row(*texts))))

    def test_a_number_that_is_lost_is_a_warning(self) -> None:
        a = row("Angle of 45 degrees", "Ángulo de 45 grados", "Winkel von 45 Grad", "Angle de 45 degrés", "Angolo di 45 gradi", "45度の角度", "45度角", "Ângulo de 45 graus")
        self.assertEqual(check_row(a), [])
        texts = list(a.texts)
        texts[6] = "直角"
        self.assertIn("numbers", rules_of(check_row(row(*texts))))

    def test_brazilian_portuguese_is_found_and_european_is_not(self) -> None:
        for word in ("O arquivo foi salvo", "A tela está vazia", "O usuário digitou", "Baixe o celular", "Deletar a pasta"):
            self.assertIn("pt-brazil", rules_of(check_row(with_pt(word))), word)
        for word in ("O ficheiro foi guardado", "O ecrã está vazio", "O utilizador escreveu", "Transfira o telemóvel", "Apagar a pasta"):
            self.assertNotIn("pt-brazil", rules_of(check_row(with_pt(word))), word)

    def test_baixar_is_fine_when_the_english_text_lowers_something(self) -> None:
        lower = row("Lower priority", "Bajar prioridad", "Priorität senken", "Baisser la priorité", "Abbassa priorità", "優先度を下げる", "降低优先级", "Baixar prioridade")
        self.assertNotIn("pt-brazil", rules_of(check_row(lower)))

    def test_a_text_in_the_wrong_language_is_found(self) -> None:
        texts = list(GOOD.texts)
        texts[6] = texts[5]  # Japanese in the column of Chinese
        self.assertIn("language", rules_of(check_row(row(*texts))))
        texts = list(GOOD.texts)
        texts[1] = "ファイルは無効です"
        self.assertIn("language", rules_of(check_row(row(*texts))))
        long_en = "The file is not valid and it cannot be read with the settings that are in force"
        es = "El archivo no es válido y no se puede leer con los ajustes que están en vigor"
        de = "Die Datei ist ungültig und kann mit den geltenden Einstellungen nicht gelesen werden"
        base = [long_en, es, de, es, es, "ファイルは無効で、現在の設定では読み込めません", "文件无效，无法用当前设置读取", es]
        self.assertIn("language", rules_of(check_row(row(*base))))

    def test_a_long_label_is_a_warning_and_the_width_counts_wide_characters_twice(self) -> None:
        short = row("Open", "Abrir", "Öffnen", "Ouvrir", "Apri", "開く", "打开", "Abrir")
        self.assertEqual(check_row(short), [])
        texts = list(short.texts)
        texts[5] = "とても長いボタンの文字列がここにあって画面の幅を超えてしまいます"
        found = check_row(row(*texts))
        self.assertEqual([(f.rule, f.severity) for f in found], [("long-label", "warning")])

    def test_a_row_of_options_is_not_a_row_of_texts(self) -> None:
        options = row("--dir", "--config", "--out", "--density", "--family", "--name", "--lang", "--param")
        self.assertEqual(check_row(options), [])


class ReaderTests(unittest.TestCase):
    def test_the_reader_of_rust_finds_a_row_of_eight_strings_with_its_key(self) -> None:
        tokens = audit.tokens('const T: &[(&str, [&str; 8])] = &[("K-1", ["a", "b\\n", r"c", r#"d"e"#, "f", "g", "\\u{e9}", "h"])]; // ["x"]')
        strings = [value for kind, value, _ in tokens if kind == "str"]
        self.assertEqual(strings, ["K-1", "a", "b\n", "c", 'd"e', "f", "g", "é", "h"])

    def test_comments_and_lifetimes_do_not_confuse_the_reader(self) -> None:
        source = "fn f<'a>(x: &'a str) -> char { /* \"no\" */ let c = '\"'; \"yes\" }\n"
        strings = [value for kind, value, _ in audit.tokens(source) if kind == "str"]
        self.assertEqual(strings, ["yes"])

    def test_a_line_continuation_in_a_string_drops_the_next_indentation(self) -> None:
        source = 'const S: &str = "one \\\n        two";\n'
        strings = [value for kind, value, _ in audit.tokens(source) if kind == "str"]
        self.assertEqual(strings, ["one two"])

    def test_the_real_tables_are_found(self) -> None:
        rows = audit.all_rows()
        self.assertGreater(len(rows), 1400)
        coded = [r for r in rows if r.code]
        self.assertGreater(len(coded), 600)
        for r in rows:
            self.assertEqual(len(r.texts), 8, r.where())


class RepositoryTests(unittest.TestCase):
    """The tables and the documents of the repository itself."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.rows = audit.all_rows()
        cls.exceptions = audit.load_exceptions()

    def errors(self, findings: list[audit.Finding]) -> list[str]:
        return [str(f) for f in findings if f.severity == "error"]

    def test_every_table_of_texts_agrees_in_the_eight_languages(self) -> None:
        self.assertEqual(self.errors(audit.check_rows(self.rows, self.exceptions)), [])

    def test_the_table_of_terms_of_the_glossary_has_eight_texts_in_every_row(self) -> None:
        terms = audit.glossary_rows()
        self.assertGreater(len(terms), 20)
        self.assertEqual(self.errors(audit.check_rows(terms, self.exceptions)), [])

    def test_the_titles_of_the_codes_are_the_ones_of_the_table_of_error_codes(self) -> None:
        self.assertEqual(self.errors(audit.check_code_tables(self.rows)), [])

    def test_the_translated_documents_follow_their_originals(self) -> None:
        self.assertEqual(self.errors(audit.check_documents()), [])

    def test_the_documents_that_exist_in_eight_languages_are_all_there(self) -> None:
        pairs = audit.document_pairs()
        tutorials = {tr for _, tr, _ in pairs if tr.startswith("docs/tutorials/")}
        self.assertEqual(len(tutorials), 7 * 10)
        for folder in ("docs/glossary", "docs/migration"):
            self.assertEqual(len([tr for _, tr, _ in pairs if tr.startswith(folder)]), 7)
        self.assertEqual(len([tr for _, tr, _ in pairs if tr.startswith("README_")]), 7)

    def test_the_exceptions_are_texts_that_still_exist(self) -> None:
        english = {r.texts[0].strip() for r in list(self.rows) + audit.glossary_rows()}
        for text in self.exceptions:
            self.assertIn(text, english, text)


if __name__ == "__main__":
    unittest.main()
