//! The methods of an operating system (PROTOCOL.md §5.1). Two values name a process: its pid
//! and its `process_start`, the start time that the kernel gives it.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Two readings of the start time of one process within this many milliseconds are the same
/// start.
pub const SAME_START_MS: u64 = 10_000;

/// `process/start`: start a process detached, in a session and process group of its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "ProcessStart")
)]
#[serde(deny_unknown_fields)]
pub struct Start {
    /// The program and its arguments.
    pub argv: Vec<String>,
    /// The whole environment of the process.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// The folder that the process runs in.
    pub cwd: PathBuf,
    /// The file that its standard input reads. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdin: Option<PathBuf>,
    /// The standard input of the process never ends, and `stdin` is absent. An adapter
    /// refuses `invalid` where the request sets both.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub held: bool,
    /// The file that the adapter appends the standard output of the process to.
    pub stdout: PathBuf,
    /// The file that the adapter appends the standard error of the process to.
    pub stderr: PathBuf,
    /// The label of the service whose share of the CPU the process runs in, where there is
    /// one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// The `<what>` of `STEERY_STARTED`. The adapter sets `STEERY_STARTED=<its own pid>:<what>`
    /// in the environment of the process. This value replaces any value that `env` gives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started: Option<String>,
    /// The file where the adapter writes how the process ended ([`Ended`]), at mode 0600. The
    /// adapter writes it when it reaps the process.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<PathBuf>,
}

/// How a process that `process/start` started ended, as its `status` file holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "ProcessEnded")
)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum Ended {
    /// It exited with this code.
    Code(i32),
    /// A signal ended it.
    Signal(i32),
}

/// The process that `process/start` started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Started {
    /// Its pid.
    pub pid: u32,
    /// Its start time, in milliseconds since 1970.
    pub process_start: u64,
}

/// `process/list`: the processes that run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "ProcessList")
)]
#[serde(deny_unknown_fields)]
pub struct List {
    /// Only these pids. Absent: every process.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pids: Option<Vec<u32>>,
}

/// The processes that `process/list` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "ProcessListed")
)]
#[serde(deny_unknown_fields)]
pub struct Listed {
    /// Each process that runs, with no zombie. Where no process has a pid in `pids`, that pid
    /// is absent.
    pub processes: Vec<Entry>,
}

/// One process that runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "ProcessEntry")
)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Its pid.
    pub pid: u32,
    /// The pid of its parent.
    pub ppid: u32,
    /// Its process group.
    pub pgid: u32,
    /// Its start time, in milliseconds since 1970.
    pub process_start: u64,
    /// The name of its executable.
    pub command: String,
    /// Its arguments, with its own name first. Absent where the adapter cannot read them.
    /// Then the process can be any program.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// The user it runs as.
    pub user: String,
    /// The folder that it works in now. Absent where the adapter cannot read it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    /// The process is stopped (Ctrl-Z, SIGSTOP).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stopped: bool,
}

/// `process/environ`: some variables of the environment of a process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Environ {
    /// Its pid.
    pub pid: u32,
    /// Its start time. The pid must still have this start time.
    pub process_start: u64,
    /// The variables to read.
    pub names: Vec<String>,
}

/// The variables that `process/environ` read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Values {
    /// Each variable that has a value, by name. A variable with no value is absent.
    pub values: BTreeMap<String, String>,
}

/// `process/files`: the files a process holds open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Files {
    /// Its pid.
    pub pid: u32,
    /// Its start time. The pid must still have this start time.
    pub process_start: u64,
    /// Only the files that it holds open to write.
    pub writing: bool,
}

/// The open files that `process/files` found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Paths {
    /// The absolute path of each file.
    pub paths: Vec<PathBuf>,
}

/// `process/listening`: the processes that listen on a TCP port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Listening {
    pub port: u16,
}

/// The processes that `process/listening` found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Listeners {
    pub processes: Vec<Listener>,
}

/// A process that listens on a port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Listener {
    pub pid: u32,
    pub process_start: u64,
    /// The name of its executable.
    pub command: String,
}

/// `process/signal`: send a signal to a process, only where the pid still has that start time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Signal {
    /// Its pid.
    pub pid: u32,
    /// Its start time. The pid must still have this start time.
    pub process_start: u64,
    /// The signal.
    pub signal: SignalName,
    /// Send the signal to its whole process group.
    pub group: bool,
}

/// A signal that the runtime sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SignalName {
    Int,
    Term,
    Kill,
    Cont,
}

/// Whether `process/signal` sent the signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Sent {
    /// False where the pid no longer has that start time.
    pub sent: bool,
}

/// `service/plan`: what installs the runtime as a service. The method is pure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ServicePlan {
    /// The name of the service.
    pub label: String,
    /// The binary of the runtime.
    pub binary: PathBuf,
    /// Its arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Its environment.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// The service manager gives the runtime the listener of this loopback port.
    pub port: u16,
    /// The share of the runtime, 1 to 100 percent, of what the computer grants its user.
    pub cpu: u8,
    /// The file that the service appends its own output to.
    pub log: PathBuf,
}

/// What installs the runtime as a service. The runtime writes the files and runs the commands.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Plan {
    /// The files to write.
    pub files: Vec<PlannedFile>,
    /// The commands to run after the files, in order. When a command that must not fail
    /// fails, the runtime does not run the rest.
    pub commands: Vec<PlanCommand>,
}

/// One command of a [`Plan`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PlanCommand {
    /// The command and its arguments.
    pub argv: Vec<String>,
    /// When this command fails, the plan continues.
    #[serde(default)]
    pub may_fail: bool,
}

/// One file of a [`Plan`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PlannedFile {
    /// Its absolute path.
    pub path: PathBuf,
    /// Its whole content.
    pub content: String,
    /// Its Unix mode bits.
    pub mode: u32,
}

/// `service/status`, `service/start`, `service/restart`, `service/remove`: one service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Service {
    /// The name of the service.
    pub label: String,
}

/// The state of a service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Status {
    /// Its unit is on disk.
    pub installed: bool,
    /// The service manager runs it.
    pub running: bool,
    /// The process that the service runs as.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_start: Option<u64>,
    /// The percent of one core the computer grants the runtime's user.
    pub granted: u32,
}
