// SPDX-License-Identifier: GPL-3.0-or-later
//! The density levels, the dimensions of a variant, the set of families that come with the
//! program and the errors of looking one up.

use std::collections::BTreeMap;
use std::path::Path;

use eh3d_phoenix_ipclib::codes::{FAMILY_UNKNOWN, FILE_UNUSABLE, VARIANT_UNKNOWN};
use eh3d_phoenix_ipclib::family::Variant;
use eh3d_phoenix_ipclib::{Density, FamilySet, IpcSettings};

fn variant(values: &[(&str, f64)]) -> Variant {
    Variant {
        values: values
            .iter()
            .map(|(k, v)| ((*k).to_string(), *v))
            .collect::<BTreeMap<String, f64>>(),
        ..Variant::default()
    }
}

#[test]
fn a_density_level_has_a_key_and_the_letter_that_ends_a_footprint_name() {
    assert_eq!(Density::ALL, [Density::A, Density::B, Density::C]);
    let keys: Vec<&str> = Density::ALL.iter().map(|d| d.key()).collect();
    assert_eq!(keys, vec!["A", "B", "C"]);
    let letters: Vec<char> = Density::ALL.iter().map(|d| d.letter()).collect();
    assert_eq!(letters, vec!['M', 'N', 'L']);
}

#[test]
fn a_level_is_named_by_either_set_of_letters_in_either_case() {
    for (text, level) in [
        ("A", Density::A),
        ("m", Density::A),
        ("B", Density::B),
        (" n ", Density::B),
        ("c", Density::C),
        ("L", Density::C),
    ] {
        assert_eq!(Density::parse(text), Some(level), "{text:?}");
    }
    for text in ["", "D", "most", "AB", "1"] {
        assert_eq!(Density::parse(text), None, "{text:?}");
    }
    // the letters and the keys give the same level back
    for level in Density::ALL {
        assert_eq!(Density::parse(level.key()), Some(level));
        assert_eq!(Density::parse(&level.letter().to_string()), Some(level));
    }
}

#[test]
fn the_levels_are_ordered_from_the_most_copper_to_the_least() {
    assert!(Density::A < Density::B && Density::B < Density::C);
}

#[test]
fn a_dimension_is_the_value_of_the_variant_or_the_default_or_an_error() {
    let v = variant(&[("pitch", 0.5), ("body", 3.0)]);
    assert_eq!(v.get("pitch"), Some(0.5));
    assert_eq!(v.get("height"), None);
    assert_eq!(v.get_or("body", 9.0), 3.0);
    assert_eq!(v.get_or("height", 1.25), 1.25);
    assert_eq!(v.need("pitch").unwrap(), 0.5);
    let error = v.need("height").unwrap_err();
    assert_eq!(error.detail, "the variant has no value height");
    assert_eq!(error.code, "PHOENIX-IPC-001");
}

#[test]
fn a_variant_with_no_values_has_none_of_them() {
    let v = Variant::default();
    assert_eq!(v.get("pitch"), None);
    assert_eq!(v.get_or("pitch", 0.65), 0.65);
    assert!(v.need("pitch").is_err());
}

#[test]
fn the_families_of_the_program_have_lower_case_names_and_ordered_variants() {
    let set = FamilySet::builtin().unwrap();
    assert!(!set.families.is_empty());
    let mut names: Vec<&str> = set.families.iter().map(|f| f.family.as_str()).collect();
    for name in &names {
        assert_eq!(*name, name.to_lowercase(), "{name}");
        assert!(!name.is_empty());
    }
    // no name appears twice
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(names.len(), before);
    for family in &set.families {
        let variants = family.variant_names();
        assert!(!variants.is_empty(), "{}", family.family);
        // the names come in the order of the map
        let mut sorted = variants.clone();
        sorted.sort_unstable();
        assert_eq!(variants, sorted, "{}", family.family);
        for name in variants {
            assert!(family.variant(name).is_ok(), "{}/{name}", family.family);
        }
    }
}

#[test]
fn a_family_is_found_by_name_and_an_unknown_name_lists_the_known_ones() {
    let set = FamilySet::builtin().unwrap();
    let first = set.families[0].family.clone();
    assert_eq!(set.get(&first).unwrap().family, first);
    let error = set.get("no-such-family").unwrap_err();
    assert_eq!(error.code, FAMILY_UNKNOWN);
    assert!(error
        .detail
        .starts_with("no-such-family; the families are "));
    for family in &set.families {
        assert!(error.detail.contains(&family.family));
    }
    // an empty set lists nothing
    let empty = FamilySet::default().get("chip").unwrap_err();
    assert_eq!(empty.code, FAMILY_UNKNOWN);
    assert_eq!(empty.detail, "chip; the families are ");
}

#[test]
fn an_unknown_variant_names_the_family_and_what_it_has() {
    let set = FamilySet::builtin().unwrap();
    let family = &set.families[0];
    let error = family.variant("no-such-variant").unwrap_err();
    assert_eq!(error.code, VARIANT_UNKNOWN);
    assert_eq!(
        error.detail,
        format!(
            "{}/no-such-variant; the family has {}",
            family.family,
            family.variant_names().join(", ")
        )
    );
}

#[test]
fn a_folder_that_is_not_there_is_an_unusable_file() {
    let settings = IpcSettings::default();
    let error = FamilySet::from_dir(Path::new("this-folder-does-not-exist-anywhere"), &settings)
        .unwrap_err();
    assert_eq!(error.code, FILE_UNUSABLE);
    assert!(error.detail.contains("this-folder-does-not-exist-anywhere"));
}
