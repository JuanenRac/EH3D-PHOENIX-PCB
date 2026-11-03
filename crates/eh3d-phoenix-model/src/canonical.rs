//! Canonical JSON as in RFC 8785 (JSON Canonicalization Scheme).
//!
//! The writer lives in the shared crate `eh3d-container`, which every container format uses; this
//! module keeps the names the model has always had and the error type of the model, and the tests
//! below pin the vectors of the standard against the shared writer.
//!
//! * object keys sorted by UTF-16 code units, no insignificant whitespace;
//! * numbers in the ECMAScript shortest round-trip form (`1e+30`, `0.002`, never `2E-3`);
//! * strings with the minimal escapes (`\"`, `\\`, `\b`, `\t`, `\n`, `\f`, `\r`, other control
//!   characters as lowercase `\u00xx`; everything else literally, in UTF-8).
//!
//! Local rules on top: UTF-8 without a byte order mark, one final line feed, optional fields
//! written only when set (serde `skip_serializing_if` on the type), and integers that no double
//! equals refused (a JSON number is a double).

use serde::Serialize;
use serde_json::Value;

use crate::error::CanonicalError;

/// The canonical text of a JSON value, without the final line feed.
pub fn canonical_string(value: &Value) -> Result<String, CanonicalError> {
    Ok(eh3d_container::canonical_string(value)?)
}

/// The canonical text of a JSON value followed by one line feed, as bytes.
pub fn canonical_file_bytes(value: &Value) -> Result<Vec<u8>, CanonicalError> {
    Ok(eh3d_container::canonical_file_bytes(value)?)
}

/// The canonical file bytes of any serialisable value.
pub fn to_canonical_file_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, CanonicalError> {
    Ok(eh3d_container::to_canonical_file_bytes(value)?)
}

/// A double in the ECMAScript number form that RFC 8785 asks for.
pub fn es_number_string(value: f64) -> Result<String, CanonicalError> {
    Ok(eh3d_container::canonical::es_number_string(value)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeterministicRandom, RandomSource};
    use serde_json::{json, Map};

    fn canon(text: &str) -> String {
        canonical_string(&serde_json::from_str::<Value>(text).unwrap()).unwrap()
    }

    /// The JSON escape `\uXXXX` built at run time.
    fn esc(hex: &str) -> String {
        let mut s = String::from("\\");
        s.push('u');
        s.push_str(hex);
        s
    }

    #[test]
    fn rfc_8785_section_3_2_4_example() {
        // Input of the RFC with its escapes: euro sign, "$", U+000F, line feed, "A", "'", "B",
        // a quote and a backslash given as escapes, then an escaped backslash, quote and slash.
        let string_body = format!(
            "{}${}{}A'{}{}{}\\\\\\\"\\/",
            esc("20ac"),
            esc("000F"),
            esc("000a"),
            esc("0042"),
            esc("0022"),
            esc("005c")
        );
        let input = format!(
            "{{\"numbers\": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001], \"string\": \"{string_body}\", \"literals\": [null, true, false]}}"
        );
        let expected = format!(
            "{{\"literals\":[null,true,false],\"numbers\":[333333333.3333333,1e+30,4.5,0.002,1e-27],\"string\":\"\u{20ac}${}\\nA'B\\\"\\\\\\\\\\\"/\"}}",
            esc("000f")
        );
        assert_eq!(canon(&input), expected);
    }

    #[test]
    fn rfc_8785_section_3_2_3_key_order_is_by_utf16_code_unit() {
        let input = format!(
            "{{\"{}\": 1, \"{}\": 2, \"{}\": 3, \"1\": 4, \"{}{}\": 5, \"{}\": 6, \"{}\": 7}}",
            esc("20ac"),
            "\\r",
            esc("fb33"),
            esc("d83d"),
            esc("de00"),
            esc("0080"),
            esc("00f6")
        );
        let expected = "{\"\\r\":2,\"1\":4,\"\u{80}\":6,\"\u{f6}\":7,\"\u{20ac}\":1,\"\u{1F600}\":5,\"\u{fb33}\":3}";
        let out = canon(&input);
        assert_eq!(out, expected);
        // The emoji (surrogate pair D83D DE00) sorts before U+FB33 in UTF-16, after it in UTF-8.
        assert!(out.find('\u{1F600}').unwrap() < out.find('\u{fb33}').unwrap());
    }

    #[test]
    fn rfc_8785_appendix_b_number_vectors() {
        let cases: [(u64, &str); 17] = [
            (0x0000_0000_0000_0000, "0"),
            (0x8000_0000_0000_0000, "0"),
            (0x0000_0000_0000_0001, "5e-324"),
            (0x7fef_ffff_ffff_ffff, "1.7976931348623157e+308"),
            (0x4340_0000_0000_0000, "9007199254740992"),
            (0xc340_0000_0000_0000, "-9007199254740992"),
            (0x4430_0000_0000_0000, "295147905179352830000"),
            (0x44b5_2d02_c7e1_4af5, "9.999999999999997e+22"),
            (0x44b5_2d02_c7e1_4af6, "1e+23"),
            (0x44b5_2d02_c7e1_4af7, "1.0000000000000001e+23"),
            (0x444b_1ae4_d6e2_ef50, "1e+21"),
            (0x3eb0_c6f7_a0b5_ed8d, "0.000001"),
            (0x3eb0_c6f7_a0b5_ed8c, "9.999999999999997e-7"),
            (0x41b3_de43_5555_5553, "333333333.3333332"),
            (0x41b3_de43_5555_5554, "333333333.33333325"),
            (0x41b3_de43_5555_5555, "333333333.3333333"),
            (0x41b3_de43_5555_5556, "333333333.3333334"),
        ];
        for (bits, expected) in cases {
            assert_eq!(
                es_number_string(f64::from_bits(bits)).unwrap(),
                expected,
                "{bits:#x}"
            );
        }
    }

    #[test]
    fn number_layout_boundaries() {
        for (v, s) in [
            (1.0, "1"),
            (-1.5, "-1.5"),
            (123456789012345680000.0, "123456789012345680000"),
            (1e21, "1e+21"),
            (1.5e21, "1.5e+21"),
            (0.000001, "0.000001"),
            (0.0000001, "1e-7"),
            (1.25e-7, "1.25e-7"),
            (12.4, "12.4"),
            (100.0, "100"),
            (0.5, "0.5"),
        ] {
            assert_eq!(es_number_string(v).unwrap(), s);
        }
        assert_eq!(es_number_string(f64::NAN), Err(CanonicalError::NotFinite));
        assert_eq!(
            es_number_string(f64::INFINITY),
            Err(CanonicalError::NotFinite)
        );
    }

    #[test]
    fn integers_that_no_double_equals_are_refused() {
        assert_eq!(
            canonical_string(&json!(9007199254740992_u64)).unwrap(),
            "9007199254740992"
        );
        assert!(matches!(
            canonical_string(&json!(9007199254740993_u64)),
            Err(CanonicalError::IntegerNotRepresentable(_))
        ));
        assert!(matches!(
            canonical_string(&json!(-9007199254740993_i64)),
            Err(CanonicalError::IntegerNotRepresentable(_))
        ));
        assert_eq!(canonical_string(&json!(-5)).unwrap(), "-5");
        // 2^60 is a double: written as the double is written, so a re-read gives the same text.
        assert_eq!(
            canonical_string(&json!(1_u64 << 60)).unwrap(),
            "1152921504606847000"
        );
        assert!(canonical_string(&json!((1_u64 << 60) + 1)).is_err());
    }

    #[test]
    fn strings_use_minimal_escapes() {
        let v = json!("a\u{8}\t\n\u{c}\r\u{1}\u{1f}\"\\/\u{7f}\u{e9}\u{1F600}");
        assert_eq!(
            canonical_string(&v).unwrap(),
            "\"a\\b\\t\\n\\f\\r\\u0001\\u001f\\\"\\\\/\u{7f}\u{e9}\u{1F600}\""
        );
    }

    #[test]
    fn file_bytes_end_with_one_line_feed_and_have_no_bom() {
        let bytes = canonical_file_bytes(&json!({"b": 1, "a": [true, null]})).unwrap();
        assert_eq!(bytes, b"{\"a\":[true,null],\"b\":1}\n");
        assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    }

    #[test]
    fn nested_objects_are_sorted_everywhere_and_empty_containers_work() {
        assert_eq!(
            canon(r#"{"z":{"b":[],"a":{}},"a":[{"y":1,"x":2}]}"#),
            r#"{"a":[{"x":2,"y":1}],"z":{"a":{},"b":[]}}"#
        );
    }

    #[derive(Serialize)]
    struct Item {
        id_nm: i64,
        name: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        note: Option<&'static str>,
    }

    #[test]
    fn typed_values_serialise_in_key_order_and_skip_unset_fields() {
        let a = to_canonical_file_bytes(&Item {
            id_nm: 5,
            name: "x",
            note: None,
        })
        .unwrap();
        assert_eq!(a, b"{\"id_nm\":5,\"name\":\"x\"}\n");
        let b = to_canonical_file_bytes(&Item {
            id_nm: 5,
            name: "x",
            note: Some("n"),
        })
        .unwrap();
        assert_eq!(b, b"{\"id_nm\":5,\"name\":\"x\",\"note\":\"n\"}\n");
    }

    fn random_value(rng: &mut DeterministicRandom, depth: u32) -> Value {
        let pick = rng.next_u64() % if depth == 0 { 5 } else { 7 };
        match pick {
            0 => Value::Null,
            1 => Value::Bool(rng.next_u64() & 1 == 0),
            2 => json!((rng.next_u64() % 2_000_001) as i64 - 1_000_000),
            3 => {
                let mantissa = (rng.next_u64() % 1_000_000_000) as f64 / 1000.0;
                let exp = (rng.next_u64() % 60) as i32 - 30;
                json!(mantissa * 10f64.powi(exp))
            }
            4 => {
                let alphabet = [
                    'a',
                    'Z',
                    '"',
                    '\\',
                    '\n',
                    '\u{1}',
                    '\u{e9}',
                    '\u{20ac}',
                    '\u{1F600}',
                    ' ',
                ];
                let len = rng.next_u64() % 6;
                Value::String(
                    (0..len)
                        .map(|_| alphabet[(rng.next_u64() % 10) as usize])
                        .collect(),
                )
            }
            5 => Value::Array(
                (0..rng.next_u64() % 4)
                    .map(|_| random_value(rng, depth - 1))
                    .collect(),
            ),
            _ => {
                let mut m = Map::new();
                for _ in 0..rng.next_u64() % 4 {
                    let key = format!(
                        "k{}{}",
                        rng.next_u64() % 5,
                        if rng.next_u64() & 1 == 0 {
                            "\u{20ac}"
                        } else {
                            "\u{1F600}"
                        }
                    );
                    m.insert(key, random_value(rng, depth - 1));
                }
                Value::Object(m)
            }
        }
    }

    #[test]
    fn write_read_write_is_stable_for_generated_documents() {
        let mut rng = DeterministicRandom::new(2024);
        for _ in 0..500 {
            let v = random_value(&mut rng, 4);
            let first = canonical_string(&v).unwrap();
            let reread: Value = serde_json::from_str(&first).unwrap();
            let second = canonical_string(&reread).unwrap();
            assert_eq!(first, second);
            // Insertion order never matters: reverse the top-level keys and compare.
            if let Value::Object(m) = &v {
                let mut reversed = Map::new();
                for (k, x) in m.iter().rev() {
                    reversed.insert(k.clone(), x.clone());
                }
                assert_eq!(canonical_string(&Value::Object(reversed)).unwrap(), first);
            }
        }
    }
}
