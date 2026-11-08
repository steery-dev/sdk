//! Every shape the protocol carries (PROTOCOL.md §3–§6). Times are milliseconds since 1970.
//! Every object rejects a field it does not define (§2).

pub mod door;
pub mod harness;
pub mod host;
pub mod hosts;
pub mod process;

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The runtime's first request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct InitializeParams {
    /// The protocol versions the runtime speaks.
    pub protocol: Vec<u32>,
    /// The runtime that asks.
    pub runtime: Runtime,
    /// The person's home folder.
    pub home: PathBuf,
    /// The runtime's config folder.
    pub config: PathBuf,
    /// The values the person gave for the settings the program declared, and nothing else.
    #[serde(default)]
    pub settings: BTreeMap<String, String>,
}

/// The runtime a program runs under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    /// Its release: `6.9.6`.
    pub release: String,
    /// The operating system it runs on, as Rust names it: `linux`, `macos`.
    pub os: String,
    /// The processor architecture, as Rust names it: `x86_64`, `aarch64`.
    pub arch: String,
}

/// What a program is: the answer to `initialize`. A driver answers `harness`. An adapter
/// answers `machine`. A program never answers both.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Initialized {
    /// The one protocol version that the program speaks, from the versions that the runtime
    /// offered.
    pub protocol: u32,
    /// Who the program is.
    pub program: ProgramInfo,
    /// A driver's harness: its levels, identity, facts and hooks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness: Option<Box<HarnessInfo>>,
    /// An adapter's machine: what it provides, and that kind's facts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine: Option<Machine>,
    /// The settings the program needs a person to give once.
    #[serde(default)]
    pub settings: Vec<Setting>,
}

impl Initialized {
    /// Checks the rules that the shapes cannot express (PROTOCOL.md §3):
    ///
    /// 1. The program's label, short name, tint and mark stay within their bounds.
    /// 2. A program answers only the object of its kind: `harness` for a driver, `machine`
    ///    for an adapter.
    /// 3. Each level of a driver comes after the level that it needs.
    /// 4. A driver names `live` exactly when it has a door.
    /// 5. A driver names its executables.
    /// 6. The hook events of a driver are unique.
    ///
    /// The error is the first broken rule, in one line. The id checks itself ([`ProgramId`]).
    pub fn check(&self) -> Result<(), String> {
        self.program.check()?;
        match (self.program.kind, &self.harness, &self.machine) {
            (Kind::Harness, Some(harness), None) => harness.check(),
            (Kind::Machine, None, Some(machine)) => machine.check(),
            (Kind::Harness, _, _) => Err("a driver must answer `harness`, not `machine`".into()),
            (Kind::Machine, _, _) => Err("an adapter must answer `machine`, not `harness`".into()),
        }
    }
}

impl ProgramInfo {
    /// The longest `label`, in characters.
    pub const LABEL_MAX: usize = 40;
    /// The longest `short`, in characters.
    pub const SHORT_MAX: usize = 3;
    /// The longest `mark`, in bytes.
    pub const MARK_MAX: usize = 4096;

    fn check(&self) -> Result<(), String> {
        let line = |text: &str, max: usize| {
            !text.trim().is_empty()
                && text.trim() == text
                && !text.chars().any(char::is_control)
                && text.chars().count() <= max
        };
        if !line(&self.label, Self::LABEL_MAX) {
            return Err(format!(
                "the program's `label` is not one line of 1 to {} characters",
                Self::LABEL_MAX
            ));
        }
        if !line(&self.short, Self::SHORT_MAX) {
            return Err(format!(
                "the program's `short` is not 1 to {} characters",
                Self::SHORT_MAX
            ));
        }
        if let Some(tint) = &self.tint {
            let hex = tint.strip_prefix('#').unwrap_or_default();
            if hex.len() != 6 || !hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
                return Err(format!(
                    "the program's `tint` {tint:?} is not `#rrggbb` in lowercase"
                ));
            }
        }
        if let Some(mark) = &self.mark {
            let path = mark.trim();
            if path.is_empty()
                || mark.len() > Self::MARK_MAX
                || !path.starts_with(['M', 'm'])
                || !mark.bytes().all(|b| {
                    b.is_ascii_digit()
                        || b" \t\n,.-+eE".contains(&b)
                        || b"MmZzLlHhVvCcSsQqTtAa".contains(&b)
                })
            {
                return Err(format!(
                    "the program's `mark` is not an SVG path's `d` of at most {} bytes",
                    Self::MARK_MAX
                ));
            }
        }
        Ok(())
    }
}

impl Machine {
    fn check(&self) -> Result<(), String> {
        match self {
            Self::Hosts(HostFacts {
                unavailable: Some(why),
                ..
            }) if why.trim().is_empty() || why.contains('\n') => {
                Err("the adapter's `unavailable` is not one line".into())
            }
            _ => Ok(()),
        }
    }
}

impl HarnessInfo {
    fn check(&self) -> Result<(), String> {
        let mut named = self.levels.clone();
        named.sort();
        named.dedup();
        if named.is_empty() {
            return Err("the driver names no level".into());
        }
        if named.len() != self.levels.len() {
            return Err("the driver names a level twice".into());
        }
        if named[..] != Level::ALL[..named.len()] {
            return Err("the driver names a level without the level before it".into());
        }
        if named.contains(&Level::Live) != self.facts.door.is_some() {
            return Err("the driver names `live` without a door, or a door without `live`".into());
        }
        if self.identity.executables.iter().all(String::is_empty) {
            return Err("the driver names no executable".into());
        }
        let mut events: Vec<&str> = self.hooks.iter().map(|hook| hook.event.as_str()).collect();
        events.sort_unstable();
        if let Some(twice) = events.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(format!("the driver names the hook event `{}` twice", twice[0]));
        }
        Ok(())
    }
}

/// A program's id (PROTOCOL.md §3): lowercase kebab-case, at most 64 characters. For a
/// driver, it is the id of the harness, which `Session.harness` names. Only
/// [`ProgramId::try_from`] and [`str::parse`] make one, so each `ProgramId` is valid. On the
/// wire it is a plain string, and a bad string does not deserialize.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProgramId(String);

impl ProgramId {
    /// The longest id.
    pub const MAX: usize = 64;

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ProgramId {
    type Error = String;

    fn try_from(id: String) -> Result<Self, String> {
        let kebab = id.len() <= Self::MAX
            && id.split('-').all(|word| {
                !word.is_empty()
                    && word
                        .bytes()
                        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            });
        if kebab {
            Ok(Self(id))
        } else {
            Err(format!(
                "`{id}` is not lowercase kebab-case of at most {} characters",
                Self::MAX
            ))
        }
    }
}

impl std::str::FromStr for ProgramId {
    type Err = String;

    fn from_str(id: &str) -> Result<Self, String> {
        Self::try_from(id.to_owned())
    }
}

impl From<ProgramId> for String {
    fn from(id: ProgramId) -> Self {
        id.0
    }
}

impl AsRef<str> for ProgramId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for ProgramId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProgramId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<str> for ProgramId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for ProgramId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for ProgramId {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ProgramId".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": "^[a-z0-9]+(-[a-z0-9]+)*$",
            "maxLength": Self::MAX,
        })
    }
}

/// Who a program is, and how a person sees it. Each surface where Steery draws a harness or a
/// machine reads these fields and nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ProgramInfo {
    /// For a driver, the id that `Session.harness` names. For an adapter, the id that a
    /// machine names as its adapter.
    pub id: ProgramId,
    /// Driver or adapter.
    pub kind: Kind,
    /// The program's own release.
    pub release: String,
    /// Its name as a person reads it, one line of at most [`Self::LABEL_MAX`] characters:
    /// `Claude Code`, `Daytona`.
    pub label: String,
    /// Its name on a tile, one to [`Self::SHORT_MAX`] characters: `CC`.
    pub short: String,
    /// Its colour on a tile, `#rrggbb` in lowercase. Absent: Steery's neutral tint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tint: Option<String>,
    /// Its mark: the `d` of one SVG path in a 24 by 24 box, at most [`Self::MARK_MAX`] bytes.
    /// Surfaces fill the path with the even-odd rule. Absent: the tile shows `short`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<String>,
}

/// What a program connects Steery to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// One harness: a driver.
    Harness,
    /// One kind of machine: an adapter.
    Machine,
}

/// How much of the work with its harness a driver does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// It reads conversations and answers hooks (§4.1).
    Observe,
    /// It starts, stops and reads turns (§4.2).
    Drive,
    /// It reaches a running conversation through its door (§4.3).
    Live,
}

impl Level {
    /// Every level in order. Each level comes after the level that it needs.
    pub const ALL: [Level; 3] = [Level::Observe, Level::Drive, Level::Live];
}

/// A driver's harness, as `initialize` states it (§3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HarnessInfo {
    /// Its levels. Each level needs the level before it. The driver names `live` exactly when
    /// it gives `facts.door`.
    pub levels: Vec<Level>,
    /// The oldest release the driver speaks to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<Minimum>,
    /// The action that signs a person in: `run /login`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sign_in: Option<String>,
    /// How an agent in this harness learns its conversation's id.
    pub knowing: String,
    /// Who writes the lines of its sessions (version 9). Absent: the agent writes them, and
    /// Steery's skill and hooks tell the agent how.
    #[serde(default, skip_serializing_if = "Lines::is_agent")]
    pub lines: Lines,
    /// The operations of Steery's catalog that the harness hosts as its own tools, each with
    /// the name of its tool: `{"cards.put": "card"}` (version 9). Steery's skill and hooks
    /// name each of these operations by its tool name. They name every other operation as
    /// Steery spells it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub hosted: BTreeMap<String, String>,
    /// One line on how to read its transcript file. Absent: the harness keeps no transcript
    /// file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
    /// How the runtime recognizes the harness's processes.
    pub identity: Identity,
    /// The facts for the setup of the harness and for its MCP calls.
    pub setup: SetupFacts,
    /// Its one-shot reader. Absent: the driver has no reader.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reader: Option<ReaderFacts>,
    /// What the holder rule and offers read.
    pub facts: HarnessFacts,
    /// Every moment of the harness that the runtime reads.
    #[serde(default)]
    pub hooks: Vec<harness::Hook>,
}

/// Who writes the lines of a session: the record of what the session does (§3.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Lines {
    /// The agent, through Steery's tools.
    #[default]
    Agent,
    /// The harness itself. Steery tells its agent nothing about the lines.
    Harness,
}

impl Lines {
    /// Whether the agent writes the lines. An absent `lines` means that it does.
    pub fn is_agent(&self) -> bool {
        *self == Lines::Agent
    }
}

/// The oldest release of a harness a driver speaks to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Minimum {
    /// The release.
    pub release: String,
    /// The command that updates the harness.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update: Option<String>,
}

/// How the runtime recognizes a harness's processes (§3.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Identity {
    /// The file names that a harness process runs as.
    pub executables: Vec<String>,
    /// A path component that names its package, where an interpreter runs the harness.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    /// The variable in which the harness puts the id of its conversation, in its own process.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_variable: Option<String>,
    /// The keys in an MCP call's `_meta` where the harness names the conversation of the call
    /// (version 8). They apply where one MCP client serves every conversation that it runs,
    /// and no header can tell whose call it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_meta: Option<CallMeta>,
}

/// Where in an MCP call's `_meta` its harness names whose call it is (§3.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CallMeta {
    /// The key whose value is the id of the conversation: the session that Steery files the
    /// call under. For a call of a sub-agent, this is the id of its lead.
    pub session: String,
    /// The key whose value is the id of the sub-agent that makes the call, where that id
    /// differs from the id of the conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_agent: Option<String>,
}

/// The facts for the setup of a harness and for its MCP calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SetupFacts {
    /// True: the harness reads Steery's skill from the entry that `setup/plan` lists. False:
    /// the MCP handshake carries the whole skill.
    pub reads_skills: bool,
    /// The harness cuts the description of an MCP tool before its model reads it. So
    /// `session_start` gives the whole escalation policies.
    pub cuts_tool_text: bool,
    /// The longest time that the harness waits for one MCP call, in seconds, when no entry can
    /// make this time longer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_bound: Option<u32>,
}

/// A driver's one-shot reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ReaderFacts {
    /// What the reader blocks.
    pub sealed: Sealed,
    /// Models that the reader runs, which `models` possibly does not list.
    #[serde(default)]
    pub known: Vec<String>,
    /// The reader runs the harness's own model when it gets no model.
    pub unnamed: bool,
}

/// What a reader blocks (true) or can still reach (false).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Sealed {
    /// Its tools.
    pub tools: bool,
    /// Its MCP servers.
    pub mcp: bool,
    /// Rules and instructions files.
    pub instructions: bool,
    /// Extensions and plugins.
    pub extensions: bool,
    /// Its saved history.
    pub history: bool,
}

/// A driver's facts: what the runtime's holder rule and offers read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HarnessFacts {
    /// A continue names a conversation by its transcript path, not by its id.
    pub by_file: bool,
    /// A continue runs only in the folder the conversation ran in.
    pub resumes_in_place: bool,
    /// A fork can run beside a live writer.
    pub forks_beside: bool,
    /// What a continue keeps without an instruction.
    pub keeps: Keeps,
    /// How the runtime reaches a live conversation. Absent: there is no live door.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub door: Option<harness::LiveDoor>,
    /// The door itself checks that the conversation is idle when it takes words.
    pub checks_idle: bool,
    /// `start` makes a new worktree for a fork or a new conversation when the call asks for
    /// one.
    pub worktrees: bool,
    /// Every act that the harness can do.
    pub acts: Vec<harness::Act>,
    /// The driver sends `sessions/touched` for every conversation that changes. The runtime
    /// then reads its conversations only when touched, and sweeps none of them (§4.1). A
    /// driver that picked a version older than [`crate::TOUCHES`] leaves it false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub touches: bool,
}

/// What a continue keeps without an instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Keeps {
    /// The model the conversation ran on.
    pub model: bool,
    /// How hard it thought.
    pub thinking: bool,
}

/// An adapter's machine: what it provides, and that kind's facts (§3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "provides", rename_all = "snake_case")]
pub enum Machine {
    /// An operating system: its processes and its service manager (§5.1).
    Processes(ProcessFacts),
    /// Hosts it makes, one for each conversation (§5.2).
    Hosts(HostFacts),
}

/// An operating system's facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ProcessFacts {
    /// The operating system.
    pub os: Os,
    /// The service manager that keeps the runtime in operation.
    pub service: ServiceManager,
}

/// An operating system an adapter provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Os {
    Macos,
    Linux,
}

/// A service manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ServiceManager {
    Launchd,
    Systemd,
}

/// The facts of an adapter that makes hosts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HostFacts {
    /// What survives `hosts/stop`.
    pub keeps: Survives,
    /// The only folder that survives, where `keeps` is `volume`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persist: Option<PathBuf>,
    /// The vendor's own idle timer.
    pub idle: Idle,
    /// The optional methods that it has, each by the last word of its name.
    #[serde(default)]
    pub optional: Vec<Optional>,
    /// It accepts `source: {dockerfile}` in `hosts/ensure`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub builds: bool,
    /// How `hosts/channel` reaches a host.
    pub channel: Channel,
    /// The operating system inside its hosts.
    pub os: Os,
    /// Why this computer cannot get a host from this adapter, with any settings. It is one
    /// line that names what to install or change. Absent: this computer can get a host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<String>,
}

/// What survives a host's stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Survives {
    /// The whole filesystem.
    Filesystem,
    /// The filesystem and the memory.
    Memory,
    /// Only the folder in `persist`.
    Volume,
}

/// The idle timer of the vendor behind an adapter that makes hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Idle {
    /// It stops a host after a time without activity.
    Activity,
    /// It stops a host a fixed time after it started.
    Ttl,
    /// It has none.
    None,
}

/// An optional method of an adapter that makes hosts, by the last word of its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Optional {
    Archive,
    Pause,
    Fork,
    Keepalive,
    Expose,
}

/// How `hosts/channel` reaches a host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// A local command whose standard input and output reach the host (§5.2).
    Argv,
    /// Bytes the adapter carries itself (§5.3).
    Stream,
}

/// A setting a program needs a person to give once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Setting {
    /// Its name, as `initialize` passes it.
    pub name: String,
    /// The runtime keeps it in its private config folder and never shows it again.
    #[serde(default)]
    pub secret: bool,
    /// The program does not run without it.
    #[serde(default)]
    pub required: bool,
    /// One line for a person: what it is and where to find it.
    pub about: String,
}

/// The empty object: the parameters of a method that takes none, and the result of a method
/// that says nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Empty {}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter(unavailable: Option<&str>) -> Initialized {
        let mut machine = serde_json::json!({"provides": "hosts", "keeps": "filesystem",
            "idle": "activity", "optional": [], "channel": "argv", "os": "linux"});
        if let Some(why) = unavailable {
            machine["unavailable"] = why.into();
        }
        serde_json::from_value(serde_json::json!({"protocol": 3,
            "program": {"id": "daytona", "kind": "machine", "release": "0.1.0", "label": "Daytona", "short": "Dt"},
            "machine": machine}))
        .unwrap()
    }

    /// An adapter that cannot work from this computer says why in one line (§3.2); absent,
    /// it can, and a line that is empty or more than one is not an answer.
    #[test]
    fn an_adapter_that_makes_hosts_says_in_one_line_why_it_is_unavailable() {
        assert_eq!(adapter(None).check(), Ok(()));
        let unavailable = adapter(Some("install an OpenSSH client"));
        assert_eq!(unavailable.check(), Ok(()));
        let Some(Machine::Hosts(facts)) = &unavailable.machine else {
            panic!("no host facts");
        };
        assert_eq!(
            facts.unavailable.as_deref(),
            Some("install an OpenSSH client")
        );
        for bad in ["", " ", "one\ntwo"] {
            assert!(adapter(Some(bad)).check().is_err(), "{bad:?}");
        }
    }

    /// Every surface draws a program from its label, short name, tint and mark alone, so a
    /// program says them within their bounds or is not read: a label of more than one line,
    /// an empty or long short name, a tint that is not `#rrggbb`, a mark that is not an SVG
    /// path or is longer than 4096 bytes.
    #[test]
    fn a_program_says_its_look_within_bounds() {
        let with = |edit: &dyn Fn(&mut ProgramInfo)| {
            let mut initialized = adapter(None);
            edit(&mut initialized.program);
            initialized.check()
        };
        assert_eq!(
            with(&|p| {
                p.tint = Some("#1f2937".into());
                p.mark = Some("M4 4h16v16H4z M10 10h4v4h-4z".into());
            }),
            Ok(())
        );
        for (bad, why) in [
            (
                &(|p: &mut ProgramInfo| p.label = "Two\nlines".into()) as &dyn Fn(&mut ProgramInfo),
                "a label of two lines",
            ),
            (
                &|p| p.label = " Daytona".into(),
                "a label with a leading space",
            ),
            (&|p| p.short = String::new(), "no short name"),
            (&|p| p.short = "Dayt".into(), "a short name of four"),
            (&|p| p.tint = Some("#1F2937".into()), "an uppercase tint"),
            (&|p| p.tint = Some("navy".into()), "a named colour"),
            (&|p| p.mark = Some("<svg/>".into()), "markup as a mark"),
            (
                &|p| p.mark = Some("4 4h16".into()),
                "a path without a move first",
            ),
            (
                &|p| p.mark = Some(format!("M0 0{}", " h1".repeat(2000))),
                "a mark over 4096 bytes",
            ),
        ] {
            assert!(with(bad).is_err(), "{why} is refused");
        }
    }
}
