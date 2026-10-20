//! A driver's methods (PROTOCOL.md §4): observe, drive and live.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------------------
// Hooks, as `initialize` declares them (§3.1)

/// One moment of the harness the runtime reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Hook {
    /// The harness's own name for the moment, unique in the list.
    pub event: String,
    /// What the moment is.
    pub kind: HookKind,
    /// How the moment reaches the runtime.
    pub via: Via,
    /// The paragraphs a moment of this event can give the agent.
    #[serde(default)]
    pub says: Vec<Says>,
    /// The seconds the harness gives this hook before it goes on without it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waits: Option<u32>,
    /// Variables of the hook's process the driver reads.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<String>,
}

/// What a moment is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum HookKind {
    /// A conversation opened: no claim about a turn.
    SessionStart,
    /// A person's instruction went in: the agent works.
    Prompt,
    /// A call is about to run: the agent waits on it.
    BeforeCall,
    /// A call came back: the agent works.
    AfterCall,
    /// A turn ended.
    TurnEnd,
    /// A conversation ended.
    SessionEnd,
    /// The harness asks its person to allow a call.
    Permission,
    /// The harness tells its person something.
    Notification,
    /// A sub-agent starts: no claim about a turn.
    SubAgentStart,
}

/// How a moment reaches the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Via {
    /// The harness runs `<hook> <event>` with its document on standard input.
    Command,
    /// The plugin sends it over the door (§4.4).
    Door,
}

/// A paragraph a moment can give the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Says {
    /// Steery's paragraph for the conversation; at a `turn_end` it keeps the agent going,
    /// at a `sub_agent_start` it reaches the sub-agent.
    Context,
    /// That words wait for the agent.
    Notice,
    /// A line about the shell command a call runs.
    Calls,
    /// A paragraph for a sub-agent in the middle of its work.
    Working,
}

// ---------------------------------------------------------------------------------------
// Observe (§4.1)

/// `setup/plan`: what setting the harness up for Steery writes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Setup {
    /// Steery's MCP URL.
    pub mcp_url: String,
    /// The skill's Markdown.
    pub skill: String,
    /// The command line a hook runs; the hook's event is added as one more argument.
    pub hook: String,
    /// The runtime's loopback door URL, which a `plugin` door dials.
    pub door: String,
    /// The runtime's release.
    pub release: String,
    /// The seconds an MCP entry gives one call.
    pub timeout: u32,
}

/// The harness found here, and the entries that set it up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Planned {
    /// The harness, where it is installed here; absent where it is not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub found: Option<Found>,
    /// Why Steery does not set this harness up here, in one line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
    /// What the runtime writes into the harness, and nothing else.
    pub entries: Vec<Entry>,
    /// Lines for a person about the setup.
    #[serde(default)]
    pub notes: Vec<String>,
    /// How the harness is installed here, where it is not (`found` absent) and its
    /// driver can install it: its vendor's own installer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install: Option<Install>,
}

/// A harness's vendor's own installer, which the runtime runs only once a person agreed
/// (at a terminal, or by `steery setup --install`), shown to them as `about` and `argv`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Install {
    /// What runs, run as is with the person's environment and no standard input.
    pub argv: Vec<String>,
    /// One line a person reads before agreeing, holding the exact command line `argv`
    /// runs and where it installs the harness.
    pub about: String,
}

/// An installed harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Found {
    /// Its binary.
    pub binary: PathBuf,
    /// Its release.
    pub release: String,
    /// The folder it keeps its configuration and conversations in.
    pub root: PathBuf,
}

/// One place in one file the runtime writes, merges byte-exactly, records, and gives back
/// when the harness is removed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The absolute file.
    pub path: PathBuf,
    /// How the file reads.
    pub format: Format,
    /// The place in it: a JSON Pointer or a TOML key path; absent: the whole file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// What goes there; a `text` file's is a string.
    pub value: Value,
    /// What reaches the harness through it.
    pub carries: Vec<Part>,
}

/// How a setup file reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Json,
    Toml,
    Text,
}

/// What reaches a harness through a setup entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Part {
    /// Steery's MCP entry.
    Mcp,
    /// Steery's skill.
    Skill,
    /// The hooks that run `<hook> <event>`.
    Hooks,
    /// Steery's plugin, which dials the door.
    Plugin,
}

/// `sessions/list`: the harness's conversations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct List {
    /// Only those updated since.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seen_since: Option<u64>,
    /// Transcripts the harness's own moments named, which may lie outside its folders.
    #[serde(default)]
    pub transcripts: Vec<PathBuf>,
}

/// The conversations `sessions/list` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Listed {
    /// Each one.
    pub sessions: Vec<Session>,
    /// Ids whose every transcript is no conversation: a sub-agent, a run with no person.
    #[serde(default)]
    pub excluded: Vec<String>,
    /// No file named its conversation ambiguously: one absent is gone.
    pub complete: bool,
}

/// One conversation in the harness's store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Session {
    /// The harness's id for it.
    pub external_id: String,
    /// The folder it runs in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    /// Its title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// When it began.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    /// When it last changed.
    pub updated_at: u64,
    /// Every file of it on this host, oldest first: at least one where the harness
    /// keeps transcript files (`HarnessInfo.transcript`), none where it does not.
    pub transcripts: Vec<PathBuf>,
    /// The conversation it was forked from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// It was started in the harness's app.
    #[serde(default)]
    pub app: bool,
    /// The sub-agents working in it.
    #[serde(default)]
    pub sub_agents: Vec<SubAgent>,
}

/// A sub-agent working in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SubAgent {
    /// The harness's id for it.
    pub external_id: String,
    /// The sub-agent that started it; absent: the conversation's own agent did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead: Option<String>,
    /// Its name or role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The first line of what it was asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    /// When it started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<u64>,
    /// The id of the call that started it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_by: Option<String>,
    /// Every file of it, oldest first.
    pub transcripts: Vec<PathBuf>,
}

/// `sessions/locate`: one conversation by its id, however old.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Locate {
    /// The conversation.
    pub external_id: String,
}

/// The conversation `sessions/locate` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Located {
    /// It.
    pub session: Session,
}

/// `sessions/read`: a conversation's events after a cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Read {
    /// The conversation.
    pub external_id: String,
    /// One of its sub-agents, read instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_agent: Option<String>,
    /// Where the last read or fold ended; absent: from the beginning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// Read only the newest records within this many bytes of the end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail: Option<u64>,
}

/// The events `sessions/read` read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Events {
    /// Where this read ended, for the next: opaque, between whole records.
    pub cursor: String,
    /// Each one, in order.
    pub events: Vec<Event>,
    /// Events remain past this answer.
    pub more: bool,
    /// The transcript was rewritten: these start again from the beginning.
    pub reset: bool,
    /// These start at the conversation's beginning.
    pub whole: bool,
    /// The id of the newest record such that every record written before it was read:
    /// the plugin door's `welcome.entries.after` (§4.4). Absent after a `tail` read, or
    /// where no record with an id was read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// `sessions/fold`: the events of records the harness's plugin sent over its door (§4.4),
/// folded as `sessions/read` folds the transcript's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Fold {
    /// The conversation.
    pub external_id: String,
    /// One of its sub-agents, folded instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_agent: Option<String>,
    /// Where the last fold ended; absent: the records start at the conversation's first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// The records after it, in the order the harness appended them.
    pub records: Vec<Record>,
    /// Milliseconds since the newest record arrived.
    pub quiet: u64,
}

/// One record of a conversation as the harness keeps it: its id, and its value verbatim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Record {
    /// The harness's id for it.
    pub id: String,
    /// The record; absent where it is over [`super::door::RECORDS_MAX`]: the driver reads
    /// it from the transcript by `id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

/// The events `sessions/fold` folded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Folded {
    /// Where this fold ended, for the next fold or a `sessions/read`: opaque.
    pub cursor: String,
    /// Each one, in order.
    pub events: Vec<Event>,
    /// How many of the leading records these events say; the rest come again next fold.
    pub consumed: u32,
}

/// One thing a conversation did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    TurnBegan {
        at: u64,
        /// The harness's id for the turn.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        turn: Option<String>,
    },
    TurnEnded {
        at: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        turn: Option<String>,
        state: TurnState,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        failure: Option<Failure>,
    },
    Words {
        at: u64,
        /// Stable for that message.
        id: String,
        voice: Voice,
        text: String,
        /// The call a `tool` voice's text belongs to.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        call: Option<String>,
        /// What a person's words carried.
        #[serde(default)]
        clips: Vec<Clip>,
    },
    /// The harness took back the newest `turns` person instructions.
    Rollback {
        at: u64,
        turns: u32,
    },
    CallBegan {
        at: u64,
        call: Call,
    },
    CallEnded {
        at: u64,
        id: String,
        ok: bool,
        /// The length of the result's text.
        chars: u64,
        /// Its last 8 lines.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tail: Option<String>,
    },
    Tokens {
        at: u64,
        count: Count,
        /// The model response counted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        response: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        /// Every token read, cache reads and writes included.
        input: u64,
        /// Of `input`, read from a cache.
        cached: u64,
        /// Of `input`, written to a cache; absent: the harness does not say.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cache_write: Option<u64>,
        output: u64,
        /// Of `output`.
        reasoning: u64,
        /// The share of the context window left after it, 0 to 1.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context_left: Option<f64>,
    },
    Model {
        at: u64,
        model: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider: Option<String>,
        /// Absent: the harness records no thinking here.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thinking: Option<String>,
    },
    /// What the conversation said about one of its sub-agents.
    SubAgent {
        at: u64,
        external_id: String,
        /// The call that started it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        call: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<Execution>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// Every event before it is the parent's history, copied into this conversation.
    Fork {
        at: u64,
        parent: String,
    },
    /// The harness compacted the conversation's context: what came before is in its model's
    /// context only as the summary that replaced it. Never said of history a fork copied.
    Compacted {
        at: u64,
        /// The harness's id for its record of it: stable.
        id: String,
        /// Who made it compact.
        by: Compactor,
        /// The context's tokens just before it, as the harness counted them; absent: it
        /// records none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tokens_before: Option<u64>,
        /// The context's tokens just after it, summary included, as the harness counted
        /// them; absent: it records none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tokens_after: Option<u64>,
    },
}

/// How a turn ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TurnState {
    Finished,
    Failed,
    /// The harness wrote no end and its process is gone, or it was stopped.
    Interrupted,
}

/// Where a run stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Execution {
    Running,
    Finished,
    Failed,
    Interrupted,
}

/// How tokens are counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Count {
    /// One response's own count.
    Response,
    /// The running total of the whole conversation.
    Total,
}

/// Who made a harness compact a conversation's context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Compactor {
    /// The harness on its own. `reason` is why, where it records it: `threshold` (the
    /// context reached the harness's own limit), `overflow` (the model refused a call as
    /// over its limit), `idle` (it compacts a conversation left idle), else the harness's
    /// own word.
    Harness {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    /// The conversation's person asked it to (`/compact`).
    Person,
    /// Steery, through the door's `compact` ask (§4.4), by that ask's `id`.
    Steery { compaction: String },
}

/// Who said words in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Voice {
    /// What a person typed.
    Person,
    /// The agent's answer.
    Agent,
    /// Its reasoning or progress, where the harness writes it as text.
    Thought,
    /// The text of a call's result, given after its `call_ended`; a call's input is `call_began.call.input` and is never repeated as words.
    Tool,
    /// A harness's summary of earlier history.
    Summary,
}

/// Something a person's words carried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Clip {
    /// Bytes in the record, base64.
    Bytes {
        data: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// Bytes kept in a file here.
    File {
        path: PathBuf,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// A file the person named.
    Path {
        path: PathBuf,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// An address the harness passed on.
    Link { url: String },
}

/// A tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Call {
    /// The harness's id for it.
    pub id: String,
    /// The tool as the harness spells it; an MCP tool as `mcp__<server>__<tool>`.
    pub tool: String,
    /// Its input as the harness wrote it; `null` where it gave none.
    pub input: Value,
    /// What kind of tool it is.
    pub group: Group,
    /// The MCP server, where `group` is `mcp`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// The name of Steery's own tool, where the call is one: `requests_create`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steery: Option<String>,
    /// What it puts to the person inside the harness, where it waits on them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asks: Option<Asks>,
    /// The call it runs inside.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// The model response that made it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
}

/// What kind of tool a call is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum Group {
    Shell,
    FilesRead,
    FilesEdit,
    Search,
    Web,
    Browser,
    Code,
    Agents,
    Plan,
    Ask,
    Steery,
    Memory,
    Harness,
    Media,
    Custom,
    Mcp,
}

/// What a call puts to the person inside the harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Asks {
    /// Questions to answer.
    Questions(Vec<Question>),
    /// A plan to approve.
    Plan(String),
}

/// A question a call puts to the person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Question {
    pub question: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    /// More than one option may be chosen.
    pub multiple: bool,
    pub options: Vec<Choice>,
}

/// One option of a [`Question`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

/// Why a turn failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Failure {
    /// The kind.
    pub kind: FailureKind,
    /// The harness's own words, one line.
    pub line: String,
    /// It came from the model service, not the harness.
    pub service: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    /// The harness's own code for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// When the service said it takes requests again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

/// The kind of a turn's failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// The harness could not sign in to its model service.
    SignIn,
    /// The service refused for a limit and takes requests again later.
    RateLimit,
    /// The prompt is over the model's limit.
    ContextFull,
    Other,
}

/// `hook/read`: what the harness handed its hook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HookRead {
    /// The harness's own name for the moment.
    pub event: String,
    /// The document the harness handed the hook, as it came.
    pub document: String,
    /// The hook's variables its entry names.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

/// The moment `hook/read` read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HookMoment {
    /// The moment; absent: the document is no moment Steery carries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moment: Option<Moment>,
}

/// A moment the harness stopped at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Moment {
    /// The hook event it is.
    pub event: String,
    /// The conversation it is in.
    pub external_id: String,
    /// The sub-agent whose moment it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_agent: Option<String>,
    /// When it happened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<u64>,
    /// The folder it runs in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    /// Its transcript, where the harness keeps it outside its own folders.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript: Option<PathBuf>,
    /// The call it is at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call: Option<Call>,
    /// At an `after_call`: the call succeeded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    /// At a `turn_end`: how the turn ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<TurnState>,
    /// At a `turn_end`: a hook already kept this turn going once.
    #[serde(default)]
    pub continued: bool,
    /// At a `session_end`: the person left at the prompt.
    #[serde(default)]
    pub person_exited: bool,
    /// At a `prompt`: the instruction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// The share of the context window left, 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_left: Option<f64>,
    /// At a `permission`: the harness's own ways to allow such calls from now on, in its
    /// order; `always: {suggestion: n}` is the n-th.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<Suggestion>,
}

/// One of the harness's own ways to allow such calls from now on, as a person reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Suggestion {
    /// What a person reads for it: `Always Bash(cargo check:*)`.
    pub label: String,
    /// One line on how far it reaches: `this project, for you`.
    pub about: String,
}

/// A conversation, by its driver and its id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SessionRef {
    pub harness: String,
    pub external_id: String,
}

/// `hook/answer`: what the hook prints back for an answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HookAnswer {
    /// The harness's own name for the moment.
    pub event: String,
    /// The document the harness handed the hook, as it came.
    pub document: String,
    /// The hook's variables its entry names.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// What the runtime decided.
    pub answer: Answer,
}

/// What the runtime decided at a moment; empty: nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Answer {
    /// A paragraph for the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub say: Option<String>,
    /// Let the call go, or block it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,
    /// The `STEERY_LEAD` value the conversation's later processes inherit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead: Option<String>,
}

/// Let a call go, or block it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Decision {
    Allow {
        /// What the agent reads.
        reason: String,
        /// Allow such calls from now on.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        always: Option<Always>,
    },
    Deny {
        /// What the agent reads.
        reason: String,
    },
}

/// What an allow covers from now on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Always {
    /// One of the moment's `suggestions`, by its index.
    Suggestion(u32),
    /// This tool, for the rest of the session.
    Tool,
    /// Every call, for the rest of the session.
    Everything,
}

/// What the hook prints back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HookAnswered {
    /// Its standard output; empty: print nothing.
    pub stdout: String,
    /// Its exit code.
    pub exit_code: i32,
    /// A line the hook adds to a file the harness named for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append: Option<Append>,
}

/// A line a hook adds to a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Append {
    pub path: PathBuf,
    pub text: String,
}

// ---------------------------------------------------------------------------------------
// Drive (§4.2)

/// `held`, `idle`: one conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Held {
    /// The conversation.
    pub external_id: String,
}

/// What `held` answers: who holds the conversation, or where the runtime looks to find out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum HeldBy {
    /// The driver knows.
    Holder(Holder),
    /// The driver knows only where to look.
    Look(Looked),
}

/// `{look}`: where the runtime looks for a holder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Looked {
    pub look: Look,
}

/// Where the runtime looks for a conversation's holder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Look {
    /// The conversation's transcripts.
    pub transcripts: Vec<PathBuf>,
    /// A process running as the harness that nothing answers for may hold it without
    /// holding a transcript open.
    pub strays: bool,
    /// A process holding or naming the transcripts that one of these matches is
    /// somewhere the runtime cannot reach, and is never signalled.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beyond: Vec<Beyond>,
}

/// A process the runtime never signals ([`Look::beyond`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Beyond {
    /// An argument exactly equal to this; absent: every process found this way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arg: Option<String>,
    /// What the holder reads, `{"beyond": line}`.
    pub line: String,
}

/// Who holds a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Holder {
    /// Nobody.
    Nobody {
        /// What proves it: the harness's own registry, or no process writing the transcript.
        proof: String,
    },
    /// A person's process.
    Person {
        /// How its live door treats a model pick; absent: it has none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        door: Option<Door>,
        /// How it is stopped.
        stop: HolderStop,
    },
}

/// How a holder is stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum HolderStop {
    /// Only the harness's owner stops it: an opaque token of at most 1024 characters, or
    /// `null` where the owner runs no turn now.
    Owner(Option<String>),
    /// This process.
    Process {
        pid: u32,
        process_start: u64,
        /// The driver proved this is the holder.
        certain: bool,
        /// A few words a person reads after "holds it".
        how: String,
    },
    /// Somewhere the runtime cannot reach, said in one line.
    Beyond(String),
}

/// The longest an owner token is.
pub const OWNER_MAX: usize = 1024;

/// How a harness's own live door treats a model pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Door {
    /// It applies a pick itself.
    Picks,
    /// It keeps its own.
    Keeps,
}

/// How a driver reaches a live conversation (PROTOCOL.md §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum LiveDoor {
    /// Steery's plugin door: the runtime speaks it directly and applies a pick itself; the
    /// driver answers no live method.
    Plugin,
    /// The harness's own door, which applies a pick itself.
    Picks,
    /// The harness's own door, which keeps its own.
    Keeps,
}

/// What a turn does with a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Act {
    Continue,
    Fork,
    New,
    Hand,
}

/// What `idle` answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Sampled {
    /// What the driver can prove now; absent: it cannot say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<Sample>,
}

/// What a driver can prove of a conversation now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub state: Execution,
    pub observed_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// The harness's own line, where it failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Nobody is in the conversation at all.
    pub vacant: bool,
    /// It stands only if no process holds the conversation.
    pub if_unheld: bool,
}

/// `models`: the models the harness runs here. Its parameters are [`super::Empty`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Models {
    /// Each one listed, the harness's default first.
    pub models: Vec<Model>,
    /// The harness also runs models it does not list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unlisted: Option<Unlisted>,
    /// The files whose change changes the list.
    #[serde(default)]
    pub watch: Vec<PathBuf>,
}

/// What a model a harness does not list takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Unlisted {
    /// Its thinking levels, in the harness's own words.
    pub thinking: Vec<String>,
}

/// One model a harness runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Model {
    /// Its provider, as the harness spells it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Its id, as the harness spells it.
    pub model: String,
    /// Its name as a person reads it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The thinking levels it takes, in the harness's own words.
    #[serde(default)]
    pub thinking: Vec<String>,
    /// US dollars per million input tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
}

/// `start`: the launch plan of a turn. It starts nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Start {
    /// What the turn does.
    pub act: Act,
    /// The conversation continued or forked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    /// The folder it runs in.
    pub cwd: PathBuf,
    /// The turn's folder, which the runtime made.
    pub folder: PathBuf,
    /// Run a fork or a new conversation in a new worktree.
    pub worktree: bool,
    /// The model, as the harness spells it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// How hard it thinks, in the harness's own word.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// Folders outside `cwd` the turn reads.
    #[serde(default)]
    pub reads: Vec<PathBuf>,
}

/// A turn's launch plan, which the runtime starts through the operating-system adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Launch {
    /// The program and its arguments.
    pub argv: Vec<String>,
    /// The variables it adds to the environment.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// The folder it runs in.
    pub cwd: PathBuf,
    /// What its standard input carries.
    pub stdin: Stdin,
    /// It binds the unix socket `inbox` in the turn's folder.
    #[serde(default)]
    pub inbox: bool,
}

/// What a launch's standard input carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Stdin {
    /// The words.
    Words,
    /// Nothing.
    None,
    /// Nothing, and no end: the process takes its words through its live door and runs
    /// until stopped.
    Held,
}

/// What a turn's `inbox` answers when the harness took more words.
pub const HEARD: &str = "heard";

/// `turn/read`: one line a turn's process wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct TurnRead {
    /// The run the line belongs to; a driver may keep state across its lines.
    pub turn: String,
    /// The stream it came on.
    pub stream: Stream,
    /// The line.
    pub line: String,
    /// The delivery marker's name the runtime put in the words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<String>,
}

/// One of a process's output streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Stream {
    Stdout,
    Stderr,
}

/// What a turn's line says.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct TurnLine {
    /// The conversation the harness named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named: Option<String>,
    /// The harness took the words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took: Option<bool>,
    /// How the turn ends, unless a later line says otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<Ended>,
    /// The refusal word a line of standard error means.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
    /// The events of the named conversation this line says, each under the id
    /// `sessions/read` gives the same fact.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<Event>,
    /// The title the harness gave the named conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// How a turn ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Ended {
    /// The state.
    pub state: TurnState,
    /// The harness's own line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// `stop`: the harness's own way to end a turn; the runtime's signals come after.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Stop {
    /// The conversation.
    pub external_id: String,
    /// Who holds it, as `held` said.
    pub holder: Holder,
}

/// What `stop`'s graceful step did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Stopped {
    /// This step ended the turn. For a `process` holder the runtime signals it after,
    /// wherever it still runs, whatever this says.
    pub stopped: bool,
    /// The driver's one line of evidence: the turn it ended and the state it reached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof: Option<String>,
}

/// `receipt/mark`: a point in a conversation before words are sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ReceiptMark {
    /// The conversation.
    pub external_id: String,
}

/// The point `receipt/mark` took.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Marked {
    /// Opaque.
    pub mark: String,
}

/// `receipt/find`: the input carrying a delivery marker, after a mark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ReceiptFind {
    /// The conversation.
    pub external_id: String,
    /// As `receipt/mark` answered.
    pub mark: String,
    /// The delivery marker's name.
    pub delivery: String,
}

/// What `receipt/find` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    /// The harness took the words.
    pub found: bool,
    /// The owner token of the turn that took them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

/// `reader`: a launch plan for a one-shot read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Reader {
    /// The instructions; absent: the harness's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// The model, as the harness spells it; absent: the harness's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The thinking level, as the driver's `models` spells it for that model; absent: the
    /// driver's own for a read. One the harness refuses is `selection-unsupported`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// The tools it may use.
    pub tools: ReaderTools,
    /// The folder it runs in.
    pub cwd: PathBuf,
    /// The text read.
    pub asked: String,
}

/// The tools a reader may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ReaderTools {
    None,
    Read,
}

/// A reader's launch plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ReaderPlan {
    /// The program and its arguments.
    pub argv: Vec<String>,
    /// The variables it adds to the environment.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// The folder it runs in.
    pub cwd: PathBuf,
    /// `words`: `asked` goes on its standard input.
    pub stdin: Stdin,
    /// Files the runtime writes first, relative to `cwd`.
    #[serde(default)]
    pub files: Vec<ReaderFile>,
    /// How its output reads.
    pub read: ReadAs,
}

/// A file a reader's plan writes first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ReaderFile {
    /// Relative to the reader's `cwd`.
    pub path: PathBuf,
    pub content: String,
}

/// How a reader's output reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ReadAs {
    /// The whole output is the answer.
    Whole,
    /// The output goes to `reader/read`.
    Events,
}

/// `reader/read`: a reader's output, read as its answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ReaderRead {
    pub output: String,
}

/// A reader's answer, or a recap's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub text: String,
}

/// `terminal`: the command that opens a conversation in a terminal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Terminal {
    /// The conversation.
    pub external_id: String,
    /// The folder it opens in.
    pub cwd: PathBuf,
    /// Who holds it, as `held` said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder: Option<Holder>,
}

/// The command `terminal` gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct TerminalCommand {
    /// The command line, quoted for a POSIX shell.
    pub command: String,
    /// The URL that opens it in the harness's app.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    /// What a person reads about it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
}

// ---------------------------------------------------------------------------------------
// Live (§4.3)

/// `say`: words into a running conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Say {
    /// The conversation.
    pub external_id: String,
    /// The words.
    pub words: String,
    /// When they go in.
    pub mode: SayMode,
}

/// When said words go in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SayMode {
    /// Into the turn running.
    Steer,
    /// After the turn running.
    Queue,
    /// Only where nothing runs.
    Immediate,
}

/// Whether the harness was seen taking said words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Said {
    /// False: handed over unconfirmed.
    pub took: bool,
}

/// `pick`: a model and thinking for a running conversation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Pick {
    /// The conversation.
    pub external_id: String,
    /// The model, as the harness spells it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// How hard it thinks, in the harness's own word.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

/// The model and thinking a conversation runs on, as read back.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Picked {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

/// `recap`: a conversation's own model says where it stands. Its answer is a [`Text`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Recap {
    /// The conversation.
    pub external_id: String,
    /// What is asked of it.
    pub asked: String,
}

/// `session/changed`: a notification a driver with its own door sends unasked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SessionChanged {
    /// The conversation.
    pub external_id: String,
    /// Where it stands now.
    pub state: LiveState,
}

/// Where a running conversation stands, as a door says it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LiveState {
    /// A turn runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<LiveTurn>,
    /// The door checked the conversation idle; absent: it cannot say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle: Option<bool>,
    /// Words wait to go in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
    /// The person's editor holds text; absent: it cannot say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typing: Option<bool>,
    /// The harness is compacting its context.
    #[serde(default)]
    pub compacting: bool,
    /// The harness waits to retry a request.
    #[serde(default)]
    pub retrying: bool,
    /// Background work running in the process that ends with it: jobs it runs and results
    /// waiting to be told.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub jobs: u32,
    /// The model it runs on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// How hard it thinks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// The share of its context window left, 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_left: Option<f64>,
    /// The top-level calls running, oldest first.
    #[serde(default)]
    pub calls: Vec<LiveCall>,
    /// Top-level calls that ended lately, oldest first, at most 64.
    #[serde(default)]
    pub ended: Vec<LiveEnded>,
    /// The harness's own dialog open in front of the person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialog: Option<Dialog>,
    /// Turns Steery's words began that ended, at most 16.
    #[serde(default)]
    pub settled: Vec<LiveSettled>,
}

/// Whether a count is none, which its field leaves out.
fn is_zero(count: &u32) -> bool {
    *count == 0
}

impl LiveState {
    /// Idle: the door checked it idle, no turn runs, nothing is pending, nobody is
    /// typing, and it is neither compacting nor retrying.
    pub fn idle(&self) -> bool {
        self.idle == Some(true)
            && self.turn.is_none()
            && self.pending != Some(true)
            && self.typing == Some(false)
            && !self.compacting
            && !self.retrying
    }
}

/// The turn running.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LiveTurn {
    pub began_at: u64,
    /// The ids of `say`s it took as Steery's.
    #[serde(default)]
    pub says: Vec<String>,
}

/// A tool call running.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LiveCall {
    /// The harness's id for it.
    pub id: String,
    /// The tool.
    pub tool: String,
    /// When it began.
    pub since: u64,
    /// Calls running inside it.
    #[serde(default)]
    pub children: Vec<LiveChild>,
}

/// A call running inside another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LiveChild {
    pub id: String,
    pub tool: String,
}

/// A call that ended, at its true end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LiveEnded {
    pub id: String,
    pub at: u64,
}

/// A harness's own dialog in front of the person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Dialog {
    pub kind: DialogKind,
    pub title: String,
    pub since: u64,
}

/// What a harness's dialog asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DialogKind {
    Select,
    Confirm,
    Input,
    Editor,
    Custom,
}

/// A turn Steery's words began that ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LiveSettled {
    pub began_at: u64,
    pub at: u64,
    #[serde(default)]
    pub says: Vec<String>,
    /// It stopped on an error the harness did not get past.
    #[serde(default)]
    pub failed: bool,
    /// The harness's own line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
