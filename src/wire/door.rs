//! The plugin door, version 1 (PROTOCOL.md §4.4): WebSocket text frames between Steery's
//! plugin in a harness and the runtime's loopback listener, each tagged by `t`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::harness::{LiveState, Record, SessionRef, Voice};

/// The door version this crate speaks.
pub const DOOR: u32 = 1;
/// The path a plugin dials on the runtime's loopback listener.
pub const PATH: &str = "/door";
/// The longest frame either side reads.
pub const FRAME_MAX: usize = 16 << 20;
/// The most bytes of records one `entries` frame carries, and the largest record sent with
/// its value: a larger one goes as its id alone.
pub const RECORDS_MAX: usize = 8 << 20;
/// The variable naming the `compact` ask a one-shot run answers before its own turn.
pub const COMPACT_VAR: &str = "STEERY_COMPACT";
/// The file beside that ask its answer frame is written to.
pub const COMPACTED_FILE: &str = "compacted.json";
/// The shortest time between two `live` frames of one message.
pub const LIVE_EVERY_MS: u64 = 16;
/// The most bytes of one `live` frame: a longer one sends its parts cut to fit, and the next
/// frame goes on from there.
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
    /// The first frame, within 5 s.
    Hello {
        /// The door version the plugin speaks.
        door: u32,
        /// The id of a driver whose door is `plugin`.
        harness: String,
        /// The harness process.
        pid: u32,
        /// The conversation this copy is in; a sub-agent's own id where it is one.
        session_id: String,
        mode: DoorMode,
        /// The conversation a sub-agent's copy works in.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lead: Option<SessionRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<LiveState>,
        /// The folder the conversation runs in, as the harness holds it now.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<PathBuf>,
    },
    /// The whole state, when it changes.
    State { state: LiveState },
    /// A moment of a hook `via: door`: the harness's own payload, which the runtime reads
    /// with `hook/read` as it reads a command hook's document. A moment `id` is read once.
    Moment {
        id: String,
        /// The `Hook.event` it is.
        event: String,
        /// The harness's hook payload, as the plugin got it.
        document: Value,
    },
    /// A turn of the copy's own ended.
    Settle {
        id: String,
        /// The turn asked its person something.
        asked: bool,
        /// The conversation was parked.
        parked: bool,
        /// The turn waits on a Steery request.
        waiting: bool,
        /// The turn was already kept going once.
        continued: bool,
    },
    /// The answer to an ask: `ok` the op's answer, or `refused` with the harness's line.
    Answer {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ok: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        refused: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// The harness's records after `after`, in the order it appended them; sent only on a
    /// connection welcomed with `entries`.
    Entries {
        /// The id of the record before the first one here; absent: these start at the
        /// conversation's first record.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after: Option<String>,
        records: Vec<Record>,
    },
    /// The copy cannot continue from `welcome.entries.after`: this connection carries no
    /// entries.
    Lost { why: String },
    /// The agent's message as it is being written; sent only on a connection welcomed with
    /// `live`.
    Live(Live),
}

/// One assistant message being written, as the harness has it so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Live {
    /// The harness's key of the message, stable from its first frame to its last: the
    /// reply's identity, as `words.message` names it in the record.
    pub message: String,
    /// The newest piece, milliseconds since 1970 on the harness's clock.
    pub at: u64,
    /// The first piece of this message, on the same clock.
    pub first_at: u64,
    /// What changed, per voice.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<LivePart>,
    /// The tool a call being formed names, before its input is sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forming: Option<String>,
    /// The message ended: every part of it came before this frame or in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<LiveEnd>,
}

/// One voice's text of a live message from `from` on. The voice's whole text is its non-blank
/// blocks joined with `"\n"`, as the fold joins them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LivePart {
    /// `agent` or `thought`.
    pub voice: Voice,
    /// Where `text` starts in the voice's whole text, in Unicode scalar values. What was held
    /// from there on is replaced.
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
    /// A program drawing for a person.
    Rpc,
    /// A one-shot run.
    Json,
    /// A one-shot run.
    Print,
}

impl DoorMode {
    /// A person, or a program drawing for one, is at it.
    pub fn interactive(self) -> bool {
        matches!(self, Self::Tui | Self::Rpc)
    }
}

/// A frame the runtime sends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "t", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeFrame {
    /// The hello was taken. `entries`: this runtime takes the harness's records. `live`: it
    /// takes `live` frames.
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

/// Where a welcomed copy starts its `entries` frames.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DoorEntries {
    /// The id of the last record the runtime holds; absent: from the first record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// The runtime's answer to a moment or a settle.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Spoken {
    /// A paragraph for the agent; after a settle, the plugin keeps the agent going with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub say: Option<String>,
}

/// What the runtime asks of a copy, by `op`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Ask {
    /// Words into the conversation; answered with [`DoorSaid`] once the harness holds them,
    /// and, asked again under the same `id` while it holds them, once it took them.
    Say {
        id: String,
        words: String,
        mode: DoorSayMode,
        /// Only where the conversation is idle; else `harness-busy`, nothing handed over.
        idle: bool,
        /// One throw's words go in once.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        throw: Option<String>,
    },
    /// A model and thinking; answered with [`super::harness::Picked`].
    Pick {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thinking: Option<String>,
    },
    /// Stop the turn running; answered with [`DoorStopped`]. Without `say`, the person's
    /// pause.
    Stop {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        say: Option<String>,
    },
    /// Force the words of `say`, held by the harness and not yet taken, in now: the turn
    /// running is aborted, its conversation not parked, and the harness takes them as its
    /// next input; words it let go with the abort are handed again, once. Answered with
    /// [`DoorStopped`].
    Force { id: String, say: String },
    /// Where the conversation stands, in its own model's words; answered with
    /// [`super::harness::Text`].
    Recap { id: String },
    /// Forget everything before a cut in favour of `summary`; answered with [`DoorCompacted`].
    Compact { id: String, before: u64, summary: String, least: u64 },
}

/// When door words go in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DoorSayMode {
    /// Into the turn running.
    Steer,
    /// After it.
    Queue,
}

/// What a door's say did: when the harness held the words, and when it took them into
/// its turn, each milliseconds since 1970 on the harness's clock. `queued_at` without
/// `taken_at`: held, not yet taken. Neither: taken, not dated.
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
    /// A turn was stopped.
    pub stopped: bool,
    /// Background jobs still running.
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
    /// Estimated tokens forgotten; 0 where nothing was.
    pub forgot: u64,
}

/// Refusal words of the door's own.
pub mod refusal {
    /// The copy is letting the conversation go and took nothing.
    pub const ENDING: &str = "ending";
    /// The turn running is not one the given `say` began.
    pub const OTHER_TURN: &str = "other-turn";
    /// A say asked again: the harness let its words go unseen, and nothing of them is in.
    pub const NOT_TAKEN: &str = "not-taken";
}
