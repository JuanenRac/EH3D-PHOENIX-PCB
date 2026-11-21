// SPDX-License-Identifier: GPL-3.0-or-later
//! What a run is given besides the program and the design: the arguments of the command line, the
//! clock, and the rules of the one folder where files may be read and written. Also the pure helpers
//! that go with them: the calendar (seconds since 1970 in UTC, the same on every machine), the
//! format of `t2string`, the name patterns of `fileglob` and the check that a name stays inside the
//! folder.
//!
//! Nothing here opens a process, reads the environment or writes to the design.

use std::path::{Component, Path, PathBuf};

use crate::codes::{RUN_ARGUMENTS, RUN_NOT_ALLOWED};
use crate::world::Fault;

/// What the caller gives a run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunEnv {
    /// The arguments of the program: `argv[0]` is the name of the program. Empty: `argc` is 0.
    pub argv: Vec<String>,
    /// The clock, in seconds since 1970-01-01 UTC. `None`: the clock of the machine, which is the
    /// one thing that makes two runs differ; give a number to get the same bytes every time.
    pub now: Option<i64>,
}

impl RunEnv {
    /// The seconds since 1970 that `time()` answers.
    pub fn clock(&self) -> i64 {
        self.now.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
        })
    }
}

/// A date and a time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Civil {
    /// The year.
    pub year: i64,
    /// The month, 1 to 12.
    pub month: i64,
    /// The day of the month, 1 to 31.
    pub day: i64,
    /// The hour, 0 to 23.
    pub hour: i64,
    /// The minute, 0 to 59.
    pub minute: i64,
    /// The second, 0 to 59.
    pub second: i64,
    /// The day of the week, 0 for Sunday to 6 for Saturday.
    pub weekday: i64,
}

/// The calendar date of a number of seconds since 1970 (UTC, proleptic Gregorian).
pub fn civil(t: i64) -> Civil {
    let days = t.div_euclid(86_400);
    let secs = t.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    Civil {
        year,
        month,
        day,
        hour: secs / 3600,
        minute: secs % 3600 / 60,
        second: secs % 60,
        weekday: (days + 4).rem_euclid(7),
    }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// A time as text by a format of the usual kind: `d dd ddd dddd M MM MMM MMMM yy yyyy h hh H HH m
/// mm s ss AP ap`, and text between single quotes as it is. A letter that is not a code is kept.
pub fn t2string(t: i64, format: &str) -> String {
    let c = civil(t);
    let chars: Vec<char> = format.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let h12 = if c.hour % 12 == 0 { 12 } else { c.hour % 12 };
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\'' {
            i += 1;
            while i < chars.len() && chars[i] != '\'' {
                out.push(chars[i]);
                i += 1;
            }
            i += 1;
            continue;
        }
        if (ch == 'A' || ch == 'a') && chars.get(i + 1) == Some(&if ch == 'A' { 'P' } else { 'p' })
        {
            let pm = c.hour >= 12;
            out.push_str(match (ch, pm) {
                ('A', false) => "AM",
                ('A', true) => "PM",
                (_, false) => "am",
                (_, true) => "pm",
            });
            i += 2;
            continue;
        }
        let mut n = 1;
        while i + n < chars.len() && chars[i + n] == ch {
            n += 1;
        }
        let two = |v: i64| format!("{v:02}");
        match (ch, n) {
            ('d', 1) => out.push_str(&c.day.to_string()),
            ('d', 2) => out.push_str(&two(c.day)),
            ('d', 3) => out.push_str(&DAYS[c.weekday as usize][..3]),
            ('d', 4) => out.push_str(DAYS[c.weekday as usize]),
            ('M', 1) => out.push_str(&c.month.to_string()),
            ('M', 2) => out.push_str(&two(c.month)),
            ('M', 3) => out.push_str(&MONTHS[c.month as usize - 1][..3]),
            ('M', 4) => out.push_str(MONTHS[c.month as usize - 1]),
            ('y', 2) => out.push_str(&two(c.year.rem_euclid(100))),
            ('y', 4) => out.push_str(&format!("{:04}", c.year)),
            ('h', 1) => out.push_str(&h12.to_string()),
            ('h', 2) => out.push_str(&two(h12)),
            ('H', 1) => out.push_str(&c.hour.to_string()),
            ('H', 2) => out.push_str(&two(c.hour)),
            ('m', 1) => out.push_str(&c.minute.to_string()),
            ('m', 2) => out.push_str(&two(c.minute)),
            ('s', 1) => out.push_str(&c.second.to_string()),
            ('s', 2) => out.push_str(&two(c.second)),
            _ => out.extend(std::iter::repeat_n(ch, n)),
        }
        i += n;
    }
    out
}

/// Whether `name` matches `pattern`, where `*` stands for any run of characters and `?` for one.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    let (mut pi, mut ni) = (0usize, 0usize);
    let (mut star, mut mark) = (None::<usize>, 0usize);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

fn denied(what: impl Into<String>) -> Fault {
    (RUN_NOT_ALLOWED, what.into())
}

/// The folder files are confined to, as it exists on disk.
pub fn root(dir: &str) -> Result<PathBuf, Fault> {
    if dir.is_empty() {
        return Err(denied(
            "no folder is allowed for files: set file_dir in the settings",
        ));
    }
    Path::new(dir)
        .canonicalize()
        .map_err(|e| denied(format!("the folder for files cannot be used: {e}")))
}

/// A relative name checked to stay inside the folder: no absolute path, no drive, no `..`, no
/// empty name. Returns the pieces joined to the folder. The file itself need not exist; a link
/// that leaves the folder is caught by [`confine`].
pub fn inside(root: &Path, name: &str) -> Result<PathBuf, Fault> {
    if name.is_empty() || name.contains('\0') {
        return Err((RUN_ARGUMENTS, "the name of a file is not valid".to_owned()));
    }
    let mut joined = root.to_path_buf();
    let mut pieces = 0;
    for comp in Path::new(name).components() {
        match comp {
            Component::Normal(p) => {
                joined.push(p);
                pieces += 1;
            }
            Component::CurDir => {}
            _ => {
                return Err(denied(format!(
                "`{name}` leaves the folder for files: only relative names inside it are allowed"
            )))
            }
        }
    }
    if pieces == 0 {
        return Err((RUN_ARGUMENTS, "the name of a file is not valid".to_owned()));
    }
    Ok(joined)
}

/// Checks that the real place of the parent of `path` (links resolved) is inside `root`.
pub fn confine(root: &Path, path: &Path) -> Result<(), Fault> {
    let parent = path.parent().unwrap_or(root);
    let real = parent
        .canonicalize()
        .map_err(|e| denied(format!("the folder of the file cannot be used: {e}")))?;
    if !real.starts_with(root) {
        return Err(denied("the file is outside the folder for files"));
    }
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return Err(denied("a link is not followed"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_calendar_is_right_on_known_dates() {
        let c = civil(0);
        assert_eq!((c.year, c.month, c.day, c.weekday), (1970, 1, 1, 4));
        let c = civil(951_782_400); // 2000-02-29
        assert_eq!((c.year, c.month, c.day, c.weekday), (2000, 2, 29, 2));
        let c = civil(1_700_000_000);
        assert_eq!((c.year, c.month, c.day), (2023, 11, 14));
        assert_eq!((c.hour, c.minute, c.second), (22, 13, 20));
        let c = civil(-1);
        assert_eq!(
            (c.year, c.month, c.day, c.hour, c.second),
            (1969, 12, 31, 23, 59)
        );
    }

    #[test]
    fn the_format_of_a_time_has_the_usual_codes() {
        assert_eq!(
            t2string(1_700_000_000, "yyyy-MM-dd hh:mm:ss"),
            "2023-11-14 10:13:20"
        );
        assert_eq!(
            t2string(1_700_000_000, "HH:mm dddd d MMMM yy"),
            "22:13 Tuesday 14 November 23"
        );
        assert_eq!(
            t2string(1_700_000_000, "h AP 'at' ddd MMM"),
            "10 PM at Tue Nov"
        );
        assert_eq!(t2string(0, "x yyyy"), "x 1970");
    }

    #[test]
    fn patterns_match_with_stars_and_marks() {
        assert!(glob_match("*.txt", "a.txt"));
        assert!(glob_match("a?c*", "abcdef"));
        assert!(!glob_match("*.txt", "a.txt.bak"));
        assert!(glob_match("*", ""));
        assert!(!glob_match("?", ""));
    }

    #[test]
    fn a_name_cannot_leave_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        assert!(inside(&root, "a/b.txt").is_ok());
        for bad in ["", "..", "../x", "a/../../x", "/etc/passwd", "\0"] {
            assert!(inside(&root, bad).is_err(), "{bad:?}");
        }
    }
}
