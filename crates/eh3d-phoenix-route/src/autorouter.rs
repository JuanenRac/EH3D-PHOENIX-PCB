// SPDX-License-Identifier: GPL-3.0-or-later
//! The optional adapter that runs an external autorouter on a DSN file and returns the session it
//! writes. It is off unless the settings (`config/autorouter.cfg`) say `enabled = true`, and the
//! command line asks for the run on top of that.
//!
//! What the adapter does and does not do:
//!
//! * the program is the absolute path in the settings; it is started directly, never through a
//!   shell, with the arguments of the settings (`{dsn}` and `{ses}` stand for the two files), in a
//!   new folder inside the temporary folder of the settings, with an empty environment except for
//!   the variables the settings name;
//! * a time limit ends the program if it takes too long;
//! * the adapter itself opens no network connection, and addresses (`://`) are refused in the
//!   settings; the program that is run is the owner's choice and is not confined by this code;
//! * the settings hold no secret: a variable whose name suggests one cannot be passed on;
//! * only the files of the run are read and written, and the folder of the run is removed at the
//!   end unless the settings keep it.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::codes::{
    RouteError, AUTOROUTER_CONFIG_INVALID, AUTOROUTER_DISABLED, AUTOROUTER_LAUNCH_FAILED,
    AUTOROUTER_NO_OUTPUT, AUTOROUTER_TIMEOUT, OUTPUT_FAILED, SES_UNREADABLE,
};

/// The most arguments the settings may give.
const MAX_ARGS: usize = 64;
/// The longest time limit the settings may give, in seconds.
const MAX_TIMEOUT_S: u64 = 86_400;
/// Parts of a variable name that make it look like a secret.
const SECRET_WORDS: [&str; 6] = ["KEY", "TOKEN", "SECRET", "PASS", "CRED", "AUTH"];

/// The settings of the external autorouter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutorouterConfig {
    /// The safety switch. Off by default: nothing is started while it is off.
    pub enabled: bool,
    /// The absolute path of the program.
    pub executable: String,
    /// The arguments, one per `arg` line; `{dsn}` and `{ses}` are replaced by the two paths.
    pub args: Vec<String>,
    /// How long the program may run, in seconds.
    pub timeout_s: u64,
    /// The absolute path of the folder in which the folder of each run is made.
    pub temp_dir: String,
    /// Keep the folder of the run (the design, the session, the output of the program).
    pub keep_temp: bool,
    /// The largest session that is read, in bytes.
    pub max_session_bytes: u64,
    /// The names of the environment variables that are passed to the program.
    pub env: Vec<String>,
}

impl Default for AutorouterConfig {
    fn default() -> Self {
        AutorouterConfig {
            enabled: false,
            executable: String::new(),
            args: Vec::new(),
            timeout_s: 600,
            temp_dir: String::new(),
            keep_temp: false,
            max_session_bytes: 64 * 1024 * 1024,
            env: Vec::new(),
        }
    }
}

fn invalid(what: impl Into<String>) -> RouteError {
    RouteError::new(AUTOROUTER_CONFIG_INVALID, what)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(value)
}

fn is_address(text: &str) -> bool {
    text.contains("://") || text.starts_with("\\\\") || text.starts_with("//")
}

impl AutorouterConfig {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored; a missing line
    /// keeps the default; an unknown key, a bad value or a combination that cannot work is
    /// refused with its line number.
    pub fn parse(text: &str) -> Result<AutorouterConfig, RouteError> {
        let mut c = AutorouterConfig::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| invalid(format!("line {line}: not key = value")))?;
            let (key, value) = (key.trim(), unquote(value.trim()));
            let bad = || invalid(format!("line {line}: the value of {key} is not valid"));
            match key {
                "enabled" => c.enabled = parse_bool(value).ok_or_else(bad)?,
                "executable" => c.executable = value.to_owned(),
                "arg" => c.args.push(value.to_owned()),
                "timeout_s" => c.timeout_s = value.parse().map_err(|_| bad())?,
                "temp_dir" => c.temp_dir = value.to_owned(),
                "keep_temp" => c.keep_temp = parse_bool(value).ok_or_else(bad)?,
                "max_session_bytes" => c.max_session_bytes = value.parse().map_err(|_| bad())?,
                "env" => c.env.push(value.to_owned()),
                _ => return Err(invalid(format!("line {line}: unknown key {key}"))),
            }
        }
        c.validate()?;
        Ok(c)
    }

    /// Checks the values. A switched-off configuration only has to be well formed; a switched-on
    /// one must be able to run.
    pub fn validate(&self) -> Result<(), RouteError> {
        if self.timeout_s == 0 || self.timeout_s > MAX_TIMEOUT_S {
            return Err(invalid(format!(
                "timeout_s is 1 to {MAX_TIMEOUT_S} seconds"
            )));
        }
        if self.max_session_bytes == 0 {
            return Err(invalid("max_session_bytes is not positive"));
        }
        if self.args.len() > MAX_ARGS {
            return Err(invalid(format!("there are more than {MAX_ARGS} arguments")));
        }
        for text in std::iter::once(&self.executable)
            .chain(std::iter::once(&self.temp_dir))
            .chain(&self.args)
        {
            if is_address(text) {
                return Err(invalid(
                    "an address is not accepted: the adapter makes no connection",
                ));
            }
            if text.contains('\0') {
                return Err(invalid("a value has a null character"));
            }
        }
        for name in &self.env {
            let upper = name.to_ascii_uppercase();
            if name.is_empty()
                || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                || SECRET_WORDS.iter().any(|w| upper.contains(w))
            {
                return Err(invalid(format!(
                    "the variable {name:?} cannot be passed: the name is not plain or suggests a secret"
                )));
            }
        }
        if self.enabled {
            if !Path::new(&self.executable).is_absolute() {
                return Err(invalid("executable must be an absolute path when enabled"));
            }
            if !Path::new(&self.temp_dir).is_absolute() {
                return Err(invalid("temp_dir must be an absolute path when enabled"));
            }
            let has = |token: &str| self.args.iter().any(|a| a.contains(token));
            if !has("{dsn}") || !has("{ses}") {
                return Err(invalid(
                    "the arguments must name the design {dsn} and the session {ses}",
                ));
            }
        }
        Ok(())
    }
}

/// The command line of one run, with the files it uses. Nothing has been made yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// The program.
    pub executable: PathBuf,
    /// The arguments, with the paths put in.
    pub args: Vec<String>,
    /// The folder of the run, which the program starts in.
    pub work_dir: PathBuf,
    /// The design file the adapter writes.
    pub dsn: PathBuf,
    /// The session file the program is expected to write.
    pub ses: PathBuf,
}

impl Launch {
    /// The command as one line of text, for the report of a dry run.
    pub fn describe(&self) -> String {
        let mut out = self.executable.display().to_string();
        for arg in &self.args {
            out.push(' ');
            if arg.is_empty() || arg.chars().any(char::is_whitespace) {
                out.push_str(&format!("\"{arg}\""));
            } else {
                out.push_str(arg);
            }
        }
        out
    }
}

/// A name for the folder of a run that depends on the design only (FNV-1a of its text).
pub fn run_name(dsn: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in dsn.bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("phoenix-autoroute-{hash:016x}")
}

/// The command of a run, without starting or creating anything.
pub fn launch_plan(config: &AutorouterConfig, name: &str) -> Result<Launch, RouteError> {
    config.validate()?;
    if config.executable.is_empty() || config.temp_dir.is_empty() {
        return Err(invalid("executable and temp_dir are not set"));
    }
    if !config.args.iter().any(|a| a.contains("{dsn}")) {
        return Err(invalid("the arguments must name the design {dsn}"));
    }
    let work_dir = Path::new(&config.temp_dir).join(name);
    let dsn = work_dir.join("design.dsn");
    let ses = work_dir.join("design.ses");
    let args = config
        .args
        .iter()
        .map(|a| {
            a.replace("{dsn}", &dsn.to_string_lossy())
                .replace("{ses}", &ses.to_string_lossy())
        })
        .collect();
    Ok(Launch {
        executable: PathBuf::from(&config.executable),
        args,
        work_dir,
        dsn,
        ses,
    })
}

fn launch_failed(what: impl Into<String>) -> RouteError {
    RouteError::new(AUTOROUTER_LAUNCH_FAILED, what)
}

/// Runs the program on the design and returns the text of the session it wrote.
///
/// Fails with a code, and starts nothing, when the switch is off.
pub fn run(config: &AutorouterConfig, dsn_text: &str) -> Result<String, RouteError> {
    if !config.enabled {
        return Err(RouteError::new(
            AUTOROUTER_DISABLED,
            "enabled is false in the settings of the autorouter",
        ));
    }
    let launch = launch_plan(config, &run_name(dsn_text))?;
    if !launch.executable.is_file() {
        return Err(launch_failed(format!(
            "{} is not a file",
            launch.executable.display()
        )));
    }
    if !Path::new(&config.temp_dir).is_dir() {
        return Err(launch_failed(format!(
            "{} is not a folder",
            config.temp_dir
        )));
    }
    fs::create_dir(&launch.work_dir).map_err(|e| {
        RouteError::new(OUTPUT_FAILED, format!("{}: {e}", launch.work_dir.display()))
    })?;
    let result = run_in(config, &launch, dsn_text);
    if !config.keep_temp {
        let _ = fs::remove_dir_all(&launch.work_dir);
    }
    result
}

fn run_in(
    config: &AutorouterConfig,
    launch: &Launch,
    dsn_text: &str,
) -> Result<String, RouteError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&launch.dsn)
        .map_err(|e| RouteError::new(OUTPUT_FAILED, format!("{}: {e}", launch.dsn.display())))?;
    file.write_all(dsn_text.as_bytes())
        .map_err(|e| RouteError::new(OUTPUT_FAILED, format!("{}: {e}", launch.dsn.display())))?;
    drop(file);

    let out_file = |name: &str| -> Result<File, RouteError> {
        File::create(launch.work_dir.join(name))
            .map_err(|e| RouteError::new(OUTPUT_FAILED, format!("{name}: {e}")))
    };
    let mut command = Command::new(&launch.executable);
    command
        .args(&launch.args)
        .current_dir(&launch.work_dir)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::from(out_file("stdout.txt")?))
        .stderr(Stdio::from(out_file("stderr.txt")?));
    for name in &config.env {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut child = command
        .spawn()
        .map_err(|e| launch_failed(format!("{}: {e}", launch.executable.display())))?;
    let deadline = Instant::now() + Duration::from_secs(config.timeout_s);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(RouteError::new(
                        AUTOROUTER_TIMEOUT,
                        format!("more than {} seconds", config.timeout_s),
                    ));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(e) => return Err(launch_failed(e.to_string())),
        }
    };
    if !status.success() {
        let tail = fs::read_to_string(launch.work_dir.join("stderr.txt")).unwrap_or_default();
        let tail: String = tail.chars().take(300).collect();
        return Err(launch_failed(format!(
            "the program ended with {status}: {tail}"
        )));
    }
    let size = match fs::metadata(&launch.ses) {
        Ok(meta) => meta.len(),
        Err(_) => {
            return Err(RouteError::new(
                AUTOROUTER_NO_OUTPUT,
                format!("{} was not written", launch.ses.display()),
            ))
        }
    };
    if size > config.max_session_bytes {
        return Err(RouteError::new(
            SES_UNREADABLE,
            format!(
                "the session is {size} bytes, the limit is {}",
                config.max_session_bytes
            ),
        ));
    }
    fs::read_to_string(&launch.ses)
        .map_err(|e| RouteError::new(SES_UNREADABLE, format!("{}: {e}", launch.ses.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn absolute(name: &str) -> String {
        std::env::temp_dir()
            .join(name)
            .to_string_lossy()
            .into_owned()
    }

    fn enabled_text(exe: &str) -> String {
        format!(
            "enabled = true\nexecutable = {exe}\narg = -de\narg = {{dsn}}\narg = -do\narg = {{ses}}\ntimeout_s = 30\ntemp_dir = {}\n",
            absolute("")
        )
    }

    #[test]
    fn the_defaults_are_switched_off_and_safe() {
        let c = AutorouterConfig::parse("").unwrap();
        assert!(!c.enabled && !c.keep_temp);
        assert_eq!(c, AutorouterConfig::default());
        assert_eq!(c.timeout_s, 600);
        assert!(c.env.is_empty() && c.args.is_empty());
    }

    #[test]
    fn a_full_configuration_is_read() {
        let exe = absolute("router-program");
        let c = AutorouterConfig::parse(&enabled_text(&exe)).unwrap();
        assert!(c.enabled);
        assert_eq!(c.executable, exe);
        assert_eq!(c.args, ["-de", "{dsn}", "-do", "{ses}"]);
        assert_eq!(c.timeout_s, 30);
    }

    #[test]
    fn a_bad_configuration_is_refused_with_the_code() {
        let exe = absolute("router-program");
        let cases = [
            "enabled = maybe".to_owned(),
            "timeout_s = 0".to_owned(),
            "timeout_s = 99999999".to_owned(),
            "unknown_key = 1".to_owned(),
            "no equals sign".to_owned(),
            "executable = https://example.org/router".to_owned(),
            "arg = http://example.org/{dsn}".to_owned(),
            "env = API_TOKEN".to_owned(),
            "env = bad name".to_owned(),
            "enabled = true\nexecutable = router\ntemp_dir = x\narg = {dsn}\narg = {ses}"
                .to_owned(),
            enabled_text(&exe).replace("arg = {ses}", "arg = out.ses"),
            "max_session_bytes = 0".to_owned(),
        ];
        for text in cases {
            let e = AutorouterConfig::parse(&text).unwrap_err();
            assert_eq!(e.code, AUTOROUTER_CONFIG_INVALID, "{text}");
        }
    }

    #[test]
    fn the_command_has_the_paths_put_in() {
        let exe = absolute("router-program");
        let c = AutorouterConfig::parse(&enabled_text(&exe)).unwrap();
        let launch = launch_plan(&c, "run-1").unwrap();
        assert_eq!(launch.args[0], "-de");
        assert_eq!(launch.args[1], launch.dsn.to_string_lossy());
        assert_eq!(launch.args[3], launch.ses.to_string_lossy());
        assert!(launch.dsn.starts_with(&launch.work_dir));
        assert!(launch.describe().starts_with(&exe));
    }

    #[test]
    fn the_name_of_a_run_follows_the_design() {
        assert_eq!(run_name("(pcb a)"), run_name("(pcb a)"));
        assert_ne!(run_name("(pcb a)"), run_name("(pcb b)"));
        assert_eq!(run_name("").len(), "phoenix-autoroute-".len() + 16);
        assert_eq!(run_name(""), "phoenix-autoroute-cbf29ce484222325");
    }

    #[test]
    fn nothing_is_started_while_the_switch_is_off() {
        let e = run(&AutorouterConfig::default(), "(pcb a)").unwrap_err();
        assert_eq!(e.code, AUTOROUTER_DISABLED);
        let exe = absolute("router-program-that-is-not-there");
        let text = enabled_text(&exe).replace("enabled = true", "enabled = false");
        let off = AutorouterConfig::parse(&text).unwrap();
        assert_eq!(run(&off, "(pcb a)").unwrap_err().code, AUTOROUTER_DISABLED);
    }

    #[test]
    fn a_program_that_is_not_there_is_a_launch_failure_before_anything_is_made() {
        let exe = absolute("router-program-that-is-not-there");
        let c = AutorouterConfig::parse(&enabled_text(&exe)).unwrap();
        let e = run(&c, "(pcb a)").unwrap_err();
        assert_eq!(e.code, AUTOROUTER_LAUNCH_FAILED);
        let launch = launch_plan(&c, &run_name("(pcb a)")).unwrap();
        assert!(!launch.work_dir.exists());
    }
}
