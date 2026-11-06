//! The limits of the readers of EAGLE board and schematic files, read from `config/eagle_board.cfg`.
//!
//! They bound the work that is not bounded by the size of the file alone: the tests of rings
//! against each other grow with the square of their number. What goes over a limit is not read
//! and is counted in a `PHOENIX-EAGLE-022` note that names the limit.

/// The limits of the readers of EAGLE files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EagleLimits {
    /// The most closed rings of the outline layer (the outline and its openings) that are kept
    /// and tested for lying inside the largest one.
    pub max_outline_rings: usize,
    /// The most point-in-polygon tests (points of a restrict area times points of a pour) made
    /// while the restrict areas are given to the pours.
    pub max_restrict_checks: u64,
    /// The most tests of an end of a net against a wire of a bus made while the entries of the
    /// buses of a schematic are found.
    pub max_bus_checks: u64,
}

/// The most that a limit may be set to.
const CEILING: u64 = 1_000_000_000_000;

impl Default for EagleLimits {
    fn default() -> Self {
        EagleLimits {
            max_outline_rings: 10_000,
            max_restrict_checks: 100_000_000,
            max_bus_checks: 100_000_000,
        }
    }
}

impl EagleLimits {
    /// Reads the limits from a settings text of `key = value` lines. Blank lines and lines that
    /// start with `#` are ignored and a missing key keeps its default. An unknown key, a line
    /// that is not `key = value` and a value that is not a whole number from 1 to
    /// 1000000000000 are errors that name the line.
    pub fn parse(text: &str) -> Result<EagleLimits, String> {
        let mut limits = EagleLimits::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| format!("line {line}: not key = value"))?;
            let (key, value) = (key.trim(), value.trim());
            let number = || -> Result<u64, String> {
                let n: u64 = value
                    .parse()
                    .map_err(|_| format!("line {line}: {key} is not a whole number"))?;
                if n == 0 || n > CEILING {
                    return Err(format!("line {line}: {key} must be from 1 to {CEILING}"));
                }
                Ok(n)
            };
            match key {
                "max_outline_rings" => {
                    limits.max_outline_rings = usize::try_from(number()?)
                        .map_err(|_| format!("line {line}: {key} is too large"))?;
                }
                "max_restrict_checks" => limits.max_restrict_checks = number()?,
                "max_bus_checks" => limits.max_bus_checks = number()?,
                _ => return Err(format!("line {line}: unknown key {key}")),
            }
        }
        Ok(limits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_limits_are_the_default() {
        let shipped = include_str!("../../../config/eagle_board.cfg");
        assert_eq!(EagleLimits::parse(shipped), Ok(EagleLimits::default()));
    }

    #[test]
    fn a_line_changes_one_value_and_the_rest_keep_the_default() {
        let l = EagleLimits::parse("# note\n\nmax_bus_checks = 7\n").unwrap();
        assert_eq!(l.max_bus_checks, 7);
        assert_eq!(
            l.max_outline_rings,
            EagleLimits::default().max_outline_rings
        );
        assert_eq!(
            l.max_restrict_checks,
            EagleLimits::default().max_restrict_checks
        );
    }

    #[test]
    fn the_settings_are_strict() {
        for (text, line) in [
            ("max_bus_checks = 0", 1),
            ("max_bus_checks = 1000000000001", 1),
            ("\nmax_outline_rings = x", 2),
            ("max_restrict_checks = -1", 1),
            ("max_outline_rings", 1),
            ("\n\nwhat = 3", 3),
        ] {
            let e = EagleLimits::parse(text).unwrap_err();
            assert!(e.starts_with(&format!("line {line}:")), "{text}: {e}");
        }
    }
}
