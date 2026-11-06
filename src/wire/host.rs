//! The host protocol (PROTOCOL.md §6): the channel from control to `steery host serve` on a
//! remote host. Control sends requests. `serve` answers them and sends notifications.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Initialized, Runtime};

/// The most bytes of one connection that one side sent and the receiver did not credit yet.
pub const WINDOW: u64 = 256 << 10;
/// The most bytes in one `host/data`.
pub const CHUNK: usize = 64 << 10;
/// The most bytes in one `host/fs/read` answer.
pub const READ_MAX: u64 = 4 << 20;

/// `host/hello`: the channel's first request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "HostHello")
)]
#[serde(deny_unknown_fields)]
pub struct Hello {
    /// The protocol versions control speaks.
    pub protocol: Vec<u32>,
    /// Control's runtime.
    pub runtime: Runtime,
}

/// The answer to `host/hello`: what `serve` is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "HostWelcome")
)]
#[serde(deny_unknown_fields)]
pub struct Welcome {
    /// The one protocol version that `serve` speaks, from the versions that control offered.
    pub protocol: u32,
    /// The release of `serve`.
    pub release: String,
    /// The operating system it runs on, as Rust names it.
    pub os: String,
    /// The processor architecture, as Rust names it.
    pub arch: String,
    /// The host's home folder.
    pub home: PathBuf,
    /// The host's Steery folder.
    pub steery: PathBuf,
    /// The port of the loopback listener of `serve`.
    pub port: u16,
    /// The environment that `serve` runs in. Control gives this environment to a process
    /// that it starts on the host. In the same way, control gives its own environment to a
    /// process that it starts on its own computer.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub env: std::collections::BTreeMap<String, String>,
}

/// `host/fs/stat`: what is at each path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FsStat {
    pub paths: Vec<PathBuf>,
}

/// What `host/fs/stat` found, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FsStats {
    /// `null` where nothing is at the path.
    pub stats: Vec<Option<Stat>>,
}

/// What is at a path. `serve` does not follow a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Stat {
    pub kind: FileKind,
    pub size: u64,
    pub mtime: u64,
    /// The Unix mode bits.
    pub mode: u32,
    pub dev: u64,
    pub ino: u64,
}

/// What a path is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    File,
    Dir,
    Link,
    /// Anything else: a fifo, a socket, a device.
    Other,
}

/// `host/fs/read`: bytes of a file. `serve` reads the size of the file once and reads only
/// within that size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FsRead {
    pub path: PathBuf,
    /// The byte to start from. Absent: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    /// At most this many bytes. Absent: to the end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<u64>,
    /// Only this many bytes at the end of the file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail: Option<u64>,
}

/// The bytes that `host/fs/read` read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FsBytes {
    /// Base64.
    pub bytes: String,
    /// The size of the file when `serve` read it.
    pub size: u64,
    /// The modification time of the file when `serve` read it.
    pub mtime: u64,
}

/// `host/fs/list`: what a folder holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FsList {
    pub path: PathBuf,
}

/// What `host/fs/list` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FsEntries {
    pub entries: Vec<FsEntry>,
}

/// One thing in a folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FsEntry {
    pub name: String,
    pub kind: FileKind,
}

/// `host/fs/write` (replaces the file atomically) and `host/fs/append`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FsWrite {
    pub path: PathBuf,
    /// Base64.
    pub bytes: String,
    /// The mode of a file that the request makes.
    pub mode: u32,
}

/// `host/fs/mkdir`: make a folder and its parents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FsMkdir {
    pub path: PathBuf,
    pub mode: u32,
}

/// `host/fs/remove`: remove a file, or, where `recursive` is true, a folder and all that it
/// holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FsRemove {
    pub path: PathBuf,
    pub recursive: bool,
}

/// What `host/programs` answers: the programs of the host, each as it answered `initialize`.
/// For each program that does not run, it also says why `serve` cannot start it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Programs {
    /// The operating-system adapter of the host.
    pub os: Initialized,
    /// The drivers of the host that run.
    pub drivers: Vec<Initialized>,
    /// The drivers of the host that `serve` cannot start, each with why.
    pub missing: Vec<Missing>,
}

/// A program of the host that `serve` cannot start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Missing {
    /// `os`, or a driver's id.
    pub program: String,
    /// Why, in one line.
    pub line: String,
}

/// `host/call`: call a method of a program on the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "HostCall")
)]
#[serde(deny_unknown_fields)]
pub struct Call {
    /// `os`, or a driver's id.
    pub program: String,
    pub method: String,
    pub params: Value,
}

/// The program `os` in `host/call`.
pub const OS: &str = "os";

/// `host/notified`: a program's notification, from the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Notified {
    pub program: String,
    pub method: String,
    pub params: Value,
}

/// `host/exited`: a program of the host exited. It starts again on its next call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Exited {
    /// `os`, or a driver's id.
    pub program: String,
}

/// `host/opened`: the listener of the host accepted a connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Opened {
    /// `h<n>`.
    pub conn: String,
}

/// `host/dial`: open a connection on the host to a loopback port or a unix socket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Dial {
    /// `c<n>`.
    pub conn: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket: Option<PathBuf>,
}

/// `host/data`: the bytes of a connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "HostData")
)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub conn: String,
    /// Base64, at most [`CHUNK`] bytes.
    pub bytes: String,
}

/// `host/credit`: the receiver passed on this many bytes of a connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Credit {
    pub conn: String,
    pub credit: u64,
}

/// `host/close`: one side of a connection ends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "HostClose")
)]
#[serde(deny_unknown_fields)]
pub struct Close {
    pub conn: String,
    pub how: CloseHow,
}

/// How one side of a connection ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CloseHow {
    /// That side stops writing. The other side can still send.
    Write,
    /// The connection ends at once in both directions.
    Reset,
}
