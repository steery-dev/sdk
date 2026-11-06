//! The plugin door, version 1 (PROTOCOL.md §4.4). It carries WebSocket text frames between
//! Steery's plugin in a harness and the runtime's loopback listener. The field `t` tags each frame.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::harness::{LiveState, Record, SessionRef, Voice};

/// The door version this crate speaks.
pub const DOOR: u32 = 1;
/// The path that a plugin connects to on the runtime's loopback listener.
pub const PATH: &str = "/door";
/// The longest frame that each side reads.
pub const FRAME_MAX: usize = 16 << 20;
/// The most bytes of records that one `entries` frame carries. It is also the largest record
/// that a frame sends with its value. A frame sends a larger record as its id alone.
pub const RECORDS_MAX: usize = 8 << 20;
/// The variable that names the `compact` ask, which a one-shot run answers before its own turn.
pub const COMPACT_VAR: &str = "STEERY_COMPACT";
/// The file beside that ask where the run writes its answer frame.
pub const COMPACTED_FILE: &str = "compacted.json";
/// The shortest time between two `live` frames of one message.
pub const LIVE_EVERY_MS: u64 = 16;
/// The most bytes of one `live` frame. A longer frame sends its parts cut to fit. The next
/// frame continues from the cut.
pub const LIVE_FRAME_MAX: usize = 64 << 10;

/// Close codes the runtime sends.
pub mod close {
    /// The first frame was not a readable hello, or names no plugin driver.
    pub const HELLO: u16 = 4400;
    /// The runtime does not speak the hello's door version.
    pub const VERSION: u16 = 4409;
    /// A newer connection from the same process and conversation replaced this one.
    pub const REPLACED: u16 = 4410;
}

/// A frame the plugin sends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "t", rename_all = "snake_case", deny_unknown_fields)]
pub enum PluginFrame {
    /// The first frame. The plugin sends it within 5 s.
    Hello {
        /// The door version the plugin speaks.
        door: u32,
        /// The id of a driver whose door is `plugin`.
        harness: String,
        /// The harness process.
        pid: u32,
        /// The conversation that this copy is in. For a sub-agent, the sub-agent's own id.
        session_id: String,
        mode: DoorMode,
        /// The conversation that the copy of a sub-agent works in.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lead: Option<SessionRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<LiveState>,
        /// The folder that the conversation runs in, as the harness has it now.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<PathBuf>,
    },
    /// The whole state, when it changes.
    State { state: LiveState },
    /// A moment of a hook with `via: door`: the harness's own payload. The runtime reads it
    /// with `hook/read`, as it reads the document of a command hook. The runtime reads each
    /// moment `id` once.
    Moment {
        id: String,
        /// The `Hook.event` it is.
        event: String,
        /// The harness's hook payload, as the plugin got it.
        document: Value,
    },
    /// A turn that the copy itself ran ended.
    Settle {
        id: String,
        /// The turn asked its person something.
        asked: bool,
        /// The conversation was parked.
        parked: bool,
        /// The turn waits on a Steery request.
        waiting: bool,
        /// The plugin already continued this turn once.
        continued: bool,
    },
    /// The answer to an ask: `ok` with the answer of the op, or `refused` with the line of the
    /// harness.
    Answer {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ok: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        refused: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// The records of the harness after `after`, in the order that the harness appended them.
    /// The plugin sends them only on a connection that the runtime welcomed with `entries`.
    Entries {
        /// The id of the record before the first record in this frame. Absent: these records
        /// start at the first record of the conversation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after: Option<String>,
        records: Vec<Record>,
    },
    /// The copy cannot continue from `welcome.entries.after`: this connection carries no
    /// entries.
    Lost { why: String },
    /// The message of the agent while the agent writes it. The plugin sends it only on a
    /// connection that the runtime welcomed with `live`.
    Live(Live),
}

/// One assistant message while the agent writes it, as the harness has it now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Live {
    /// The harness's key for the message. It stays the same from the first frame to the last.
    /// It is the identity of the reply, as `words.message` names it in the record.
    pub message: String,
    /// The time of the newest piece, in milliseconds since 1970 on the harness's clock.
    pub at: u64,
    /// The time of the first piece of this message, on the same clock.
    pub first_at: u64,
    /// What changed, per voice.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<LivePart>,
    /// The tool that a call names while the agent forms the call, before the agent sends its
    /// input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forming: Option<String>,
    /// The message ended: every part of it came before this frame or in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<LiveEnd>,
}

/// The text of one voice in a live message, from `from` to the end. The whole text of the
/// voice is its non-blank blocks, joined with `"\n"` as the fold joins them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LivePart {
    /// `agent` or `thought`.
    pub voice: Voice,
    /// Where `text` starts in the whole text of the voice, in Unicode scalar values. `text`
    /// replaces all the text held from that point on.
    pub from: u32,
    pub text: String,
}

/// How a live message ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LiveEnd {
    pub stop: LiveStop,
}

/// Why a live message ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum LiveStop {
    /// The model finished.
    Stop,
    /// The model reached its output limit.
    Length,
    /// The model calls a tool next.
    ToolUse,
    /// The person or the harness stopped it.
    Aborted,
    /// The model's provider failed.
    Error,
}

/// Who is at a plugin's harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DoorMode {
    /// A person at a terminal.
    Tui,
    /// A program that draws for a person.
    Rpc,
    /// A one-shot run.
    Json,
    /// A one-shot run.
    Print,
}

impl DoorMode {
    /// True when a person, or a program that draws for a person, is at the harness.
    pub fn interactive(self) -> bool {
        matches!(self, Self::Tui | Self::Rpc)
    }
}

/// A frame the runtime sends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "t", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeFrame {
    /// The runtime accepted the hello. `entries`: this runtime takes the records of the
    /// harness. `live`: it takes `live` frames.
    Welcome {
        door: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        entries: Option<DoorEntries>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        live: bool,
    },
    /// The answer to a moment or a settle.
    Answer { id: String, ok: Spoken },
    /// Something the runtime asks of the copy.
    Ask(Ask),
}

/// Where a copy that the runtime welcomed starts its `entries` frames.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DoorEntries {
    /// The id of the last record that the runtime holds. Absent: start from the first record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// The runtime's answer to a moment or a settle.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Spoken {
    /// A paragraph for the agent. After a settle, the plugin uses it to make the agent continue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub say: Option<String>,
}

/// What the runtime asks of a copy, by `op`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Ask {
    /// Words into the conversation. The copy answers with [`DoorSaid`] when the harness holds
    /// the words. When the runtime asks again under the same `id` while the harness holds
    /// them, the copy answers when the harness took them.
    Say {
        id: String,
        words: String,
        mode: DoorSayMode,
        /// Only when the conversation is idle. Else the copy refuses with `harness-busy` and
        /// gives nothing to the harness.
        idle: bool,
        /// The words of one throw go in only once.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        throw: Option<String>,
    },
    /// A model and thinking. The copy answers with [`super::harness::Picked`].
    Pick {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thinking: Option<String>,
    },
    /// Stop the turn that runs now. The copy answers with [`DoorStopped`]. Without `say`, this
    /// is the person's pause.
    Stop {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        say: Option<String>,
    },
    /// Force the words of `say` in now. The harness holds these words and did not take them yet.
    ///
    /// 1. The copy aborts the turn that runs now. It does not park the conversation.
    /// 2. The harness takes the words as its next input.
    /// 3. The copy gives again, once, the words that the harness let go with the abort.
    ///
    /// The copy answers with [`DoorStopped`].
    Force { id: String, say: String },
    /// The state of the conversation, in the words of its own model. The copy answers with
    /// [`super::harness::Text`].
    Recap { id: String },
    /// Forget everything before a cut, and keep `summary` in its place. The copy answers with
    /// [`DoorCompacted`].
    Compact { id: String, before: u64, summary: String, least: u64 },
}

/// When door words go in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DoorSayMode {
    /// Into the turn that runs now.
    Steer,
    /// After that turn.
    Queue,
}

/// What the say of a door did. `queued_at` is when the harness held the words. `taken_at` is
/// when the harness took them into its turn. Both are milliseconds since 1970 on the harness's
/// clock. `queued_at` without `taken_at`: the harness holds the words and did not take them
/// yet. Neither: the harness took the words, and the answer gives no time.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DoorSaid {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queued_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taken_at: Option<u64>,
}

/// What a door's stop did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DoorStopped {
    /// The copy stopped a turn.
    pub stopped: bool,
    /// The background jobs that still run.
    #[serde(default)]
    pub running: Vec<String>,
}

/// What a door's compact did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DoorCompacted {
    /// Milliseconds since 1970: when the context starts after the ask.
    pub from: u64,
    /// The estimated count of tokens that the harness forgot. 0 when it forgot nothing.
    pub forgot: u64,
}

/// The door's own refusal words. The door also speaks [`crate::refusal::OTHER_TURN`]: the
/// turn that runs now is not a turn that the given `say` started.
pub mod refusal {
    /// The copy releases the conversation. It took nothing.
    pub const ENDING: &str = "ending";
    /// The runtime asked a say again: the harness dropped its words unseen. No part of them
    /// went in.
    pub const NOT_TAKEN: &str = "not-taken";
}
