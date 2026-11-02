# EH3D-PHOENIX-PCB: threat model

Basic model. A row is a result only where its state says so; the others are plans. The rows are the threats to design against from the first line of code.

| Threat | Planned mitigation | Planned test | State | Residual risk |
|---|---|---|---|---|
| Imported scripts, output jobs and plugins found in a project | never run automatically; explicit confirmation; sandbox with an allow list, resource limits and an audit log; a child process alone is not treated as a boundary | negative tests for every denied capability; hostile script corpus; parser fuzzing | planned | not assessed: nothing is implemented |
| Third-party 3D models and component libraries | bounded parsers, licence and provenance record | fuzzing | planned | not assessed: nothing is implemented |
| Hostile or malformed files of other tools (EAGLE, KiCad, Gerber, STEP) | bounded readers, size and depth limits, diagnostics instead of panics | fuzzing, hostile samples, limit tests | planned | not assessed: nothing is implemented |
| Server reachable from an untrusted network (when the optional server is used) | TLS, scoped tokens, rate limits, no direct Internet exposure until tested | negative tests: revoked token, no scope, replay | planned | not assessed: nothing is implemented |
| Leak of private designs through diagnostics, logs or reports given to others | redacted mode and leak test | automated pattern search | prototype: the inventory report holds only counts, kinds, declared versions and a fingerprint, with a leak test over names and contents; no other report exists yet | the fingerprint reveals whether two corpora are identical; a declared version string is shown only when it is made of digits and dots |
| Tasks that exhaust resources | limits on CPU, memory, disk, time and concurrency | limit tests | planned | not assessed: nothing is implemented |
| Corrupt or malicious project or input files (oversized entries, path tricks, deep nesting, resource exhaustion) | name rules, size and depth limits, bounded readers | fuzzing and hostile samples | planned | not assessed: nothing is implemented |
| Temporary files, logs and reports that reveal private designs | no private data in outputs; redaction | automated pattern search | planned | not assessed: nothing is implemented |
| Compromised dependency or binary | pinned versions, licence and advisory checks, software bill of materials for releases | CI checks | planned | not assessed: nothing is implemented |
| Power loss or forced close during a save | atomic write: temporary file, then rename; old file never deleted first | interruption tests at controlled points | planned | not assessed: nothing is implemented |

Minimum negative tests once there is code: hostile files and resource limits; secrets absent from logs, errors and reports; pinned dependencies and licence review before publishing. Report a vulnerability privately as described in SECURITY.md, without attaching private designs.
