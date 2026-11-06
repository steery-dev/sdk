//! The methods of an adapter that makes hosts (PROTOCOL.md §5.2) and its `stream` channel
//! (§5.3). The adapter's own id for a host is the name of the host.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// `hosts/list`: the adapter's hosts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "HostsList")
)]
#[serde(deny_unknown_fields)]
pub struct List {
    /// Only the hosts that have all of these labels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<BTreeMap<String, String>>,
}

/// The hosts that `hosts/list` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "HostsListed")
)]
#[serde(deny_unknown_fields)]
pub struct Listed {
    /// Each host.
    pub hosts: Vec<Host>,
}

/// One host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Host {
    /// The adapter's id for the host.
    pub host: String,
    /// The state of the host, in the vendor's own word.
    pub state: String,
    /// The labels of the host.
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    /// When the vendor made the host.
    pub created_at: u64,
}

/// `hosts/ensure`: the host for `key`. The adapter makes the host where none exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Ensure {
    /// The key that finds the host again.
    pub key: String,
    /// What the adapter makes the host from. Absent: the adapter's default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// The labels of the host.
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}

/// What an adapter makes a host from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Source {
    /// A vendor's snapshot, by name.
    Snapshot(String),
    /// A container image, by reference.
    Image(String),
    /// The whole text of a Dockerfile. The vendor builds it in its owner's account. The same
    /// bytes give the same build. Only where the adapter's facts say `builds`.
    Dockerfile(String),
}

/// A host, as `hosts/ensure` and `hosts/fork` answer it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Named {
    /// The adapter's id for it.
    pub host: String,
}

/// The parameters of `hosts/start`, `stop`, `delete`, `archive`, `pause`, `fork` and
/// `keepalive`.
pub type Target = Named;

/// A host's state after `hosts/start`, `stop`, `delete`, `archive` or `pause`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct State {
    /// In the vendor's own word.
    pub state: String,
}

/// `hosts/expose`: a URL that every other host of the adapter can use to reach one port of a
/// host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Expose {
    /// The adapter's id for the host.
    pub host: String,
    /// The port inside the host.
    pub port: u16,
}

/// What `hosts/expose` answers. It stays the same while the host lives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Exposed {
    /// The URL that reaches the port.
    pub url: String,
    /// The headers to send with each request to `url`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
}

/// `hosts/put`: write a file into a host. Its result is [`super::Empty`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Put {
    /// The adapter's id for the host.
    pub host: String,
    /// The absolute path of the file inside the host.
    pub path: PathBuf,
    /// The whole content of the file, base64.
    pub content: String,
    /// The Unix mode bits of the file.
    pub mode: u32,
}

/// `hosts/channel`: a local channel to the standard input and output of `command` in a host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(rename = "HostsChannel")
)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    /// The adapter's id for the host.
    pub host: String,
    /// The command inside the host, with its program first.
    pub command: Vec<String>,
}

/// How a channel reaches its command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged, deny_unknown_fields)]
pub enum Reach {
    /// A local command whose standard input and output are the channel.
    Argv {
        argv: Vec<String>,
        /// The variables to add to the environment of the command.
        #[serde(default)]
        env: BTreeMap<String, String>,
    },
    /// The adapter carries the bytes itself, as `stream/*` messages with this id.
    Stream { stream: String },
}

/// `stream/write`: the bytes that the runtime sends to a channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct StreamWrite {
    /// The channel.
    pub stream: String,
    /// The bytes, base64.
    pub bytes: String,
}

/// `stream/close`: the runtime ends a channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct StreamClose {
    /// The channel.
    pub stream: String,
}

/// `stream/data`: the bytes of the command, which the adapter sends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct StreamData {
    /// The channel.
    pub stream: String,
    /// The bytes, base64.
    pub bytes: String,
}

/// `stream/closed`: the adapter says that a channel ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct StreamClosed {
    /// The channel.
    pub stream: String,
    /// The command's exit code, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit: Option<i32>,
}
