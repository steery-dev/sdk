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
    /// The seconds that the harness waits for this hook before it continues without it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waits: Option<u32>,
    /// The variables of the hook's process that the driver reads.
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
    /// A person gave an instruction: the agent works.
    Prompt,
    /// A call is about to run: the agent waits on it.
    BeforeCall,
    /// A call returned: the agent works.
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
    /// The model of the conversation changed. This moment makes no claim about a turn, or
    /// about other state of the conversation.
    Changed,
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
    /// Steery's paragraph for the conversation. At a `turn_end`, it makes the agent
    /// continue. At a `sub_agent_start`, it goes to the sub-agent.
    Context,
    /// A notice that words wait for the agent.
    Notice,
    /// A line about the shell command a call runs.
    Calls,
    /// A paragraph for a sub-agent in the middle of its work.
    Working,
}

// ---------------------------------------------------------------------------------------
// Observe (§4.1)

/// `setup/plan`: what the runtime writes to configure the harness for Steery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Setup {
    /// Steery's MCP URL.
    pub mcp_url: String,
    /// The skill's Markdown.
    pub skill: String,
    /// The command line that a hook runs. The driver adds the hook's event as one more
    /// argument.
    pub hook: String,
    /// The runtime's loopback door URL, which a `plugin` door connects to.
    pub door: String,
    /// The runtime's release.
    pub release: String,
    /// The seconds that an MCP entry gives one call.
    pub timeout: u32,
    /// Folders where a harness can run here as if its person trusted them. This applies to
    /// a harness that runs only in a folder that a person trusted. The folders are those of
    /// a host that an adapter made for its conversations. Nobody opens the terminal of that
    /// host to answer the harness's prompt. Empty on a person's own computer, where the
    /// person trusts a folder themselves.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trusts: Vec<PathBuf>,
}

/// The harness that the driver found here, and the entries that configure it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Planned {
    /// The harness, where it is installed here; absent where it is not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub found: Option<Found>,
    /// Why Steery does not configure this harness here, in one line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
    /// What the runtime writes into the harness, and nothing else.
    pub entries: Vec<Entry>,
    /// Lines for a person about the setup.
    #[serde(default)]
    pub notes: Vec<String>,
    /// The vendor's own installer for the harness. Present only where the harness is not
    /// installed here (`found` absent) and its driver can install it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install: Option<Install>,
    /// What the harness lacks here that only its person can give, for example a sign-in or
    /// a setting. Steery asks for each need on one card, and the runtime gives the answer
    /// back by `needs/give`. Version 5 only: empty in an older version.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<Need>,
}

/// What a harness lacks on this host that only its person can give, and the ways that the
/// person can give it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Need {
    /// The id of this need, the same in every plan (`sign-in`). `needs/give` names the need
    /// by this id.
    pub id: String,
    /// The card's title: a short question its person answers (`Sign Claude Code in?`).
    pub title: String,
    /// What is missing and what it stops, in one line.
    pub line: String,
    /// Each way that the person can give it, with the driver's preferred way first. At
    /// least one.
    pub ways: Vec<Way>,
}

/// One way a person gives what a harness lacks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Way {
    /// The id of this way of its need, the same in every plan (`api-key`).
    pub id: String,
    /// What it does, in a few plain words (`Use your API key`).
    pub label: String,
    /// Its cost or risk, in one line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// The page that the person opens first: the page that makes a key, or an authorize
    /// page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<NeedLink>,
    /// The values that the person types. The runtime gives each one back by its `name`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
    /// When the state that this way holds ends (for example, a sign-in that the driver
    /// started for it), in milliseconds since 1970 (§2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<u64>,
}

/// A page that the person opens for a way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct NeedLink {
    /// The text of its button.
    pub label: String,
    /// Its address.
    pub url: String,
    /// The field that the return address of the page fills. Only where the page sends the
    /// browser back to a loopback address, and the app that opened the page receives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fills: Option<String>,
}

/// One value a person types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Field {
    /// Its name in `needs/give`'s `values`.
    pub name: String,
    /// What a person reads above it.
    pub label: String,
    /// Where its value comes from, in one line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// A secret: Steery never shows it again after the person types it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub secret: bool,
}

/// `needs/give`: what a person gave for one need, by one of its ways.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Give {
    /// The need's `id`.
    pub need: String,
    /// The way's `id`.
    pub way: String,
    /// Each field of the way that the person filled, by name.
    #[serde(default)]
    pub values: BTreeMap<String, String>,
}

/// The result of a [`Give`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Given {
    /// The need is met.
    Done {
        /// The result, in one line (`Signed in as ada@example.com`).
        line: String,
        /// Variables that the runtime keeps sealed for this harness on this machine. From
        /// now on, the runtime adds them to every process of the harness on every host.
        /// Empty where the given values stay in this host.
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        env: BTreeMap<String, String>,
    },
    /// One more step: Steery asks for this way next (for example, the link and code of a
    /// sign-in that started).
    Next(Way),
}

/// The harness vendor's own installer. The runtime shows `about` and `argv` to a person.
/// It runs the installer only after the person agrees, at a terminal or by
/// `steery setup --install`. On a host that an adapter made from no source of its
/// person's, the runtime runs the installer by itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Install {
    /// The command that runs. The runtime runs it as it is, with the person's environment
    /// and no standard input.
    pub argv: Vec<String>,
    /// One line that a person reads before they agree. It holds the exact command line that
    /// `argv` runs, and where it installs the harness.
    pub about: String,
}

/// An installed harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Found {
    /// Its executable.
    pub binary: PathBuf,
    /// Its release.
    pub release: String,
    /// The folder it keeps its configuration and conversations in.
    pub root: PathBuf,
}

/// One place in one file that the runtime writes. The runtime merges it byte-exactly,
/// records it, and gives it back when it removes the harness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The absolute file.
    pub path: PathBuf,
    /// The format of the file.
    pub format: Format,
    /// The place in it: a JSON Pointer or a TOML key path; absent: the whole file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// The value at that place. For a `text` file, the value is a string.
    pub value: Value,
    /// What reaches the harness through it.
    pub carries: Vec<Part>,
}

/// The format of a setup file.
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
    /// Steery's plugin, which connects to the door.
    Plugin,
}

/// `sessions/list`: the harness's conversations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct List {
    /// Only the conversations that changed since this time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seen_since: Option<u64>,
    /// Transcripts that the harness's own moments named. They can be outside the harness's
    /// folders.
    #[serde(default)]
    pub transcripts: Vec<PathBuf>,
    /// The `cursor` of an earlier answer: answer only what changed since that listing
    /// (version 10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// The conversations `sessions/list` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Listed {
    /// Each one. With `after`: each one that is new or changed since that listing.
    pub sessions: Vec<Session>,
    /// Ids where no transcript is a conversation: for example, a sub-agent or a run with no
    /// person. With `after`: each one that was not excluded in that listing.
    #[serde(default)]
    pub excluded: Vec<String>,
    /// True when no file names its conversation ambiguously. Then a conversation that is
    /// not in the list is gone.
    pub complete: bool,
    /// The name of this listing, for the `after` of the next call (version 10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// The `after` of the call: this answer gives only what changed since that listing.
    /// Absent: it gives everything (version 10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    /// With `after`: the ids that the listing of `after` had, as a conversation or as
    /// excluded, and that this listing has as neither (version 10).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gone: Vec<String>,
}

/// One conversation in the harness's store.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// Every file of it on this host, oldest first. At least one where the harness keeps
    /// transcript files (`HarnessInfo.transcript`), and none where it does not.
    pub transcripts: Vec<PathBuf>,
    /// The conversation that this one is a fork of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// It started in the harness's app.
    #[serde(default)]
    pub app: bool,
    /// The sub-agents that work in it.
    #[serde(default)]
    pub sub_agents: Vec<SubAgent>,
}

/// A sub-agent that works in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// The first line of the task that it got.
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

/// `sessions/locate`: one conversation by its id, of any age.
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
    /// The conversation.
    pub session: Session,
}

/// `sessions/read`: a conversation's events after a cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Read {
    /// The conversation.
    pub external_id: String,
    /// One of its sub-agents, to read in place of the conversation.
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
    /// Where this read ended, for the next read. Opaque, and always between whole records.
    pub cursor: String,
    /// Each one, in order.
    pub events: Vec<Event>,
    /// More events remain after this answer.
    pub more: bool,
    /// The harness rewrote the transcript: these events start again from the beginning.
    pub reset: bool,
    /// These events start at the conversation's beginning.
    pub whole: bool,
    /// The id of the newest record such that the driver read every record that the harness
    /// wrote before it. This is the plugin door's `welcome.entries.after` (§4.4). Absent
    /// after a `tail` read, or where the driver read no record with an id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// `sessions/fold`: the events of records that the harness's plugin sent over its door
/// (§4.4). The driver folds them as `sessions/read` folds the records of the transcript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Fold {
    /// The conversation.
    pub external_id: String,
    /// One of its sub-agents, to fold in place of the conversation.
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
    /// The record. Absent where it is over [`super::door::RECORDS_MAX`]: then the driver
    /// reads it from the transcript by `id`.
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
    /// How many of the first records these events describe. The other records come again
    /// in the next fold.
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
        /// The harness's key for the message of these words, where it is not `id`. It is the
        /// identity of the reply, as the `message` of a live door frame names it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        /// The call that the text of a `tool` voice belongs to.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        call: Option<String>,
        /// What a person's words carried.
        #[serde(default)]
        clips: Vec<Clip>,
    },
    /// The harness removed the newest `turns` instructions of the person.
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
        /// Version 6: the earlier calls of this conversation whose work this call waited
        /// on, each by its `call_began` id. Such a call returned while the work that it
        /// started continued to run: a command in the background, a service or a sub-agent.
        /// This call waited for that work, read from it, or polled it until it returned.
        /// Empty where this call waited on none, and in an older version.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        waited: Vec<String>,
    },
    Tokens {
        at: u64,
        count: Count,
        /// The model response that this count is for.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        response: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        /// Every token that the model read, with cache reads and cache writes.
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
    /// Every event before this one is the parent's history, which the fork copied into this
    /// conversation.
    Fork {
        at: u64,
        parent: String,
    },
    /// The harness compacted the context of the conversation. The model has the earlier
    /// history only as the summary that replaced it. The driver never gives this event for
    /// history that a fork copied.
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
    /// The harness wrote no end and its process is gone, or something stopped it.
    Interrupted,
}

/// The state of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Execution {
    Running,
    Finished,
    Failed,
    Interrupted,
}

/// How the harness counts tokens.
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
    /// The harness on its own. `reason` gives the cause, where the harness records it:
    ///
    /// - `threshold`: the context reached the harness's own limit.
    /// - `overflow`: the model refused a call as over its limit.
    /// - `idle`: the harness compacts a conversation that stayed idle.
    /// - any other value: the harness's own word.
    Harness {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    /// The conversation's person asked it to (`/compact`).
    Person,
    /// Steery, through the `compact` ask of the door (§4.4). `compaction` is the `id` of
    /// that ask.
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
    /// The agent's reasoning or progress, where the harness writes it as text.
    Thought,
    /// The text of a call's result, after its `call_ended`. The input of a call is in
    /// `call_began.call.input`, and the driver never repeats it as words.
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
    /// Bytes in a file on this host.
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
    /// An address that the harness gave.
    Link { url: String },
}

/// A tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Call {
    /// The harness's id for it.
    pub id: String,
    /// The tool, as the harness spells it. An MCP tool is `mcp__<server>__<tool>`.
    pub tool: String,
    /// Its input, as the harness wrote it. `null` where the harness gave none.
    pub input: Value,
    /// What kind of tool it is.
    pub group: Group,
    /// The MCP server, where `group` is `mcp`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// The name of Steery's own tool, where the call is one: `requests_create`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steery: Option<String>,
    /// What it asks the person inside the harness, where it waits for the person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asks: Option<Asks>,
    /// The call it runs inside.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// The model response that made it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
}

/// An enum of unit variants with `ALL`, every variant in the order declared, both from one
/// list: a variant added is in `ALL` at once.
macro_rules! listed {
    ($(#[$meta:meta])* $vis:vis enum $name:ident { $($variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        $vis enum $name { $($variant),+ }

        impl $name {
            /// Every variant, in the order of declaration.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];
        }
    };
}

listed! {
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
}

/// What a call asks the person inside the harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Asks {
    /// Questions to answer.
    Questions(Vec<Question>),
    /// A plan to approve.
    Plan(String),
}

/// A question that a call asks the person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Question {
    pub question: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    /// The person can choose more than one option.
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
    /// When the service takes requests again, as the service said.
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

/// `hook/read`: what the harness gave its hook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HookRead {
    /// The harness's own name for the moment.
    pub event: String,
    /// The document that the harness gave the hook, unchanged.
    pub document: String,
    /// The variables of the hook that its entry names.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

/// The moment that `hook/read` read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HookMoment {
    /// The moment. Absent: the document is not a moment that Steery uses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moment: Option<Moment>,
}

/// A moment where the harness stopped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Moment {
    /// The hook event of the moment.
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
    /// The call of the moment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call: Option<Call>,
    /// At an `after_call`: the call succeeded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    /// At a `turn_end`: how the turn ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<TurnState>,
    /// At a `turn_end`: a hook already made this turn continue once.
    #[serde(default)]
    pub continued: bool,
    /// At a `session_end`: the person exited at the prompt.
    #[serde(default)]
    pub person_exited: bool,
    /// At a `prompt`: the instruction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// The share of the context window left, 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_left: Option<f64>,
    /// The model that the conversation runs on now, where the moment gives it. This is
    /// state, never an event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The thinking level now, in the harness's own word, where the moment gives it. This
    /// is state, never an event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// At a `turn_end` whose `outcome` is `failed`: the cause, where the harness gives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<Failure>,
    /// At a `permission`: the harness's own ways to allow such calls from now on, in its
    /// order. `always: {suggestion: n}` is the n-th.
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
    /// One line about its scope: `this project, for you`.
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

/// `hook/answer`: what the hook prints for an answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HookAnswer {
    /// The harness's own name for the moment.
    pub event: String,
    /// The document that the harness gave the hook, unchanged.
    pub document: String,
    /// The variables of the hook that its entry names.
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
    /// Allow the call, or block it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,
    /// The `STEERY_LEAD` value that later processes of the conversation inherit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead: Option<String>,
}

/// Allow a call, or block it.
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

/// What the hook prints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HookAnswered {
    /// Its standard output; empty: print nothing.
    pub stdout: String,
    /// Its exit code.
    pub exit_code: i32,
    /// A line that the hook adds to a file that the harness named for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append: Option<Append>,
}

/// A line that a hook adds to a file.
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

/// What `held` answers: who holds the conversation, or where the runtime looks for the
/// holder.
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
    /// A stray process of the harness, which nothing claims, can hold the conversation with
    /// no transcript open.
    pub strays: bool,
    /// A process that holds or names the transcripts, and that one of these matches, is
    /// somewhere that the runtime cannot reach. The runtime never signals it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beyond: Vec<Beyond>,
}

/// A process the runtime never signals ([`Look::beyond`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Beyond {
    /// An argument exactly equal to this. Absent: every process that the runtime finds this
    /// way.
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
        /// The proof: the harness's own registry, or that no process writes the transcript.
        proof: String,
    },
    /// A person's process.
    Person {
        /// How its live door treats a model pick; absent: it has none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        door: Option<Door>,
        /// How the runtime stops it.
        stop: HolderStop,
    },
}

/// How the runtime stops a holder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum HolderStop {
    /// Only the harness's owner can stop it. The value is an opaque token of at most 1024
    /// characters, or `null` where the owner runs no turn now.
    Owner(Option<String>),
    /// This process.
    Process {
        pid: u32,
        process_start: u64,
        /// The driver proved this is the holder.
        certain: bool,
        /// A few words that a person reads after "holds it".
        how: String,
    },
    /// The holder is somewhere that the runtime cannot reach, in one line.
    Beyond(String),
}

/// The maximum length of an owner token.
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
    /// Steery's plugin door. The runtime uses it directly and applies a pick itself. The
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
    /// This sample is true only if no process holds the conversation.
    pub if_unheld: bool,
}

/// `models`: the models that the harness runs here. Its parameters are [`super::Empty`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Models {
    /// Each listed model, with the harness's default first.
    pub models: Vec<Model>,
    /// The harness also runs models that it does not list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unlisted: Option<Unlisted>,
    /// The files that change the list when they change.
    #[serde(default)]
    pub watch: Vec<PathBuf>,
    /// Whether the harness's own account is signed in here at the time of this list, as
    /// the harness says. `true`: signed in. `false`: not signed in. Absent where the driver
    /// cannot tell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signed_in: Option<bool>,
}

/// What a model takes when the harness does not list it.
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

/// `mcp/list` (protocol 4): whose MCP servers to list. Absent `cwd`: the person's own
/// servers, which every conversation loads. Present: the servers that a conversation loads
/// when it starts in that folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct McpList {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
}

/// The MCP servers a harness loads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct McpServers {
    /// Each one, by its name in the harness.
    pub servers: Vec<McpServer>,
    /// False where servers can also come from places that the driver cannot read: plugins,
    /// the connectors of an account, configuration that an administrator manages.
    pub complete: bool,
    /// The files that change the list when they change.
    pub watch: Vec<PathBuf>,
}

/// One MCP server: its name and, for a server with a URL, that URL without its secrets.
/// Nothing else of an entry leaves the machine: no command, argument, environment, header,
/// token or OAuth field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct McpServer {
    pub server: String,
    /// Scheme, host, port and path only: no user, password, query or fragment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl McpServer {
    /// The server `server`, at `url` as the configuration of its harness spells it, if the
    /// harness reaches it over a URL. The function keeps only the scheme, host, port and
    /// path of `url`. It drops a `url` that has no `scheme://`.
    pub fn new(server: impl Into<String>, url: Option<&str>) -> Self {
        Self {
            server: server.into(),
            url: url.and_then(bare_url),
        }
    }
}

/// `url` without its user, password, query and fragment, or none where it names no
/// `scheme://host`. One whose host is followed by anything but a numeric port, or whose
/// path holds an `@` its authority does not, is none: a password with an unescaped `/`,
/// `?` or `#` would otherwise stand where the host does.
fn bare_url(url: &str) -> Option<String> {
    let (scheme, rest) = url.trim().split_once("://")?;
    if scheme.is_empty()
        || !scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        return None;
    }
    let rest = &rest[..rest.find(['?', '#']).unwrap_or(rest.len())];
    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let host = match authority.rsplit_once('@') {
        Some((_, host)) => host,
        None if path.contains('@') => return None,
        None => authority,
    };
    let port = match host.strip_prefix('[') {
        Some(inside) => inside.split_once(']')?.1,
        None => host.rsplit_once(':').map_or("", |(_, port)| port),
    };
    let port = port.strip_prefix(':').unwrap_or(port);
    if host.is_empty() || host.starts_with(':') || !port.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(format!("{scheme}://{host}{path}"))
}

/// `start`: the launch plan of a turn. It starts nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Start {
    /// What the turn does.
    pub act: Act,
    /// The conversation that the turn continues or forks.
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
    /// The thinking level, in the harness's own word.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// Folders outside `cwd` that the turn reads.
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
    /// until something stops it.
    Held,
}

/// What a turn's `inbox` answers when the harness took more words.
pub const HEARD: &str = "heard";

/// `turn/read`: one line that a turn's process wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct TurnRead {
    /// The run that the line belongs to. A driver can keep state across its lines.
    pub turn: String,
    /// The stream it came on.
    pub stream: Stream,
    /// The line.
    pub line: String,
    /// The name of the delivery marker that the runtime put in the words.
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
    /// The conversation that the harness named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named: Option<String>,
    /// The harness took the words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took: Option<bool>,
    /// How the turn ends, unless a later line says otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<Ended>,
    /// The refusal word that a line of standard error means.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
    /// The events of the named conversation that this line gives. Each has the id that
    /// `sessions/read` gives the same fact.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<Event>,
    /// The title that the harness gave the named conversation.
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

/// `stop`: the harness's own way to end a turn. The runtime's signals come after it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Stop {
    /// The conversation.
    pub external_id: String,
    /// Who holds it, as `held` said.
    pub holder: Holder,
    /// The `id` of a `say` (version 7). Stop only the turn that its words began. Refuse
    /// `other-turn` where another turn runs. Absent: the person's stop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub say: Option<String>,
}

/// What the graceful step of `stop` did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Stopped {
    /// This step ended the turn. For a `process` holder, the runtime signals the process
    /// after this step where it still runs, whatever this field says.
    pub stopped: bool,
    /// The driver's one line of evidence: the turn it ended and the state it reached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof: Option<String>,
}

/// `receipt/mark`: a point in a conversation before the runtime sends words.
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

/// `receipt/find`: the input that carries a delivery marker, after a mark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ReceiptFind {
    /// The conversation.
    pub external_id: String,
    /// The mark that `receipt/mark` gave.
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

/// `reader`: a launch plan for a read that runs once.
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
    /// The thinking level, as the driver's `models` spells it for that model. Absent: the
    /// driver's own level for a read. A level that the harness refuses gives
    /// `selection-unsupported`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// The tools that it can use.
    pub tools: ReaderTools,
    /// The folder it runs in.
    pub cwd: PathBuf,
    /// The text that the reader reads.
    pub asked: String,
}

/// The tools that a reader can use.
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
    /// `words`: the runtime writes `asked` to its standard input.
    pub stdin: Stdin,
    /// Files that the runtime writes first, relative to `cwd`.
    #[serde(default)]
    pub files: Vec<ReaderFile>,
    /// How the runtime reads its output.
    pub read: ReadAs,
}

/// A file that a reader's plan writes first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ReaderFile {
    /// Relative to the reader's `cwd`.
    pub path: PathBuf,
    pub content: String,
}

/// How the runtime reads a reader's output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ReadAs {
    /// The whole output is the answer.
    Whole,
    /// The output goes to `reader/read`.
    Events,
}

/// `reader/read`: a reader's output, to read as its answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ReaderRead {
    pub output: String,
}

/// A reader's answer.
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

/// The command that `terminal` gave.
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
    /// When the words go into the conversation.
    pub mode: SayMode,
    /// The name of the say (version 7): the delivery that its words carry, else a name of
    /// the runtime's own. The driver names it in `turn.says` of the turn that took the words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// When the words of a `say` go into the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SayMode {
    /// Into the running turn.
    Steer,
    /// After the running turn.
    Queue,
    /// Only where nothing runs.
    Immediate,
}

/// Whether the driver saw the harness take the words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Said {
    /// False: the driver gave the words but has no confirmation.
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
    /// The thinking level, in the harness's own word.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

/// The model and thinking that a conversation runs on, as the driver read them back.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Picked {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

/// `session/changed`: a notification that a driver with its own door sends without a
/// request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SessionChanged {
    /// The conversation.
    pub external_id: String,
    /// Its state now.
    pub state: LiveState,
}

/// `sessions/touched`: a notification that a driver whose facts say `touches` sends without
/// a request, one for each change that it saw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SessionsTouched {
    /// The conversations whose thread, or a thread of one of whose sub-agents, changed.
    /// [`SessionsTouched::EVERY`] names every conversation that the driver listed.
    pub external_ids: Vec<String>,
}

impl SessionsTouched {
    /// The id that names every conversation that the driver listed.
    pub const EVERY: &'static str = "*";
}

/// The state of a running conversation, as a door gives it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LiveState {
    /// A turn runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<LiveTurn>,
    /// The door found the conversation idle. Absent: it cannot say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle: Option<bool>,
    /// Words wait to go into the conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
    /// The person's editor holds text; absent: it cannot say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typing: Option<bool>,
    /// The harness compacts its context now.
    #[serde(default)]
    pub compacting: bool,
    /// The harness waits to retry a request.
    #[serde(default)]
    pub retrying: bool,
    /// While it waits to retry: the try that failed, as the harness gave it. Absent where
    /// the harness said nothing about it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<LiveRetry>,
    /// Background work in the process that ends with the process: jobs that it runs, and
    /// results that the harness did not yet give the agent.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub jobs: u32,
    /// The model it runs on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The thinking level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// The share of its context window left, 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_left: Option<f64>,
    /// The top-level calls that run now, oldest first.
    #[serde(default)]
    pub calls: Vec<LiveCall>,
    /// Top-level calls that ended recently, oldest first, at most 64.
    #[serde(default)]
    pub ended: Vec<LiveEnded>,
    /// The harness's own dialog that is open in front of the person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialog: Option<Dialog>,
    /// Turns that Steery's words began and that ended, at most 16.
    #[serde(default)]
    pub settled: Vec<LiveSettled>,
}

/// Whether a count is none, which its field leaves out.
fn is_zero(count: &u32) -> bool {
    *count == 0
}

impl LiveState {
    /// Idle: the door found it idle, no turn runs, nothing is pending, nobody types, and
    /// the harness does not compact or retry.
    pub fn idle(&self) -> bool {
        self.idle == Some(true)
            && self.turn.is_none()
            && self.pending != Some(true)
            && self.typing == Some(false)
            && !self.compacting
            && !self.retrying
    }
}

/// The running turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LiveTurn {
    pub began_at: u64,
    /// The ids of the `say`s that it took as Steery's: for a driver, the `id` that each
    /// `say` gave.
    #[serde(default)]
    pub says: Vec<String>,
}

/// The try that a harness waits to retry, as it said it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LiveRetry {
    /// Which try of the request failed. The first try is 1.
    pub attempt: u32,
    /// How many tries it makes before it stops.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<u32>,
    /// What went wrong, in the harness's own words: at most 600 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// A running tool call.
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
    /// Calls that run inside it.
    #[serde(default)]
    pub children: Vec<LiveChild>,
    /// The background jobs that it waits on and that still run, oldest first, at most 8.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jobs: Vec<LiveJob>,
}

/// A call that runs inside another call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LiveChild {
    pub id: String,
    pub tool: String,
}

/// A background job that a call waits on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LiveJob {
    /// What the harness calls it, often its command: at most 120 characters.
    pub label: String,
    /// When it began.
    pub since: u64,
    /// Its newest line of output, or its stage: at most 200 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// When `output` last changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_at: Option<u64>,
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

/// A turn that Steery's words began and that ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LiveSettled {
    pub began_at: u64,
    pub at: u64,
    #[serde(default)]
    pub says: Vec<String>,
    /// It stopped on an error, and the harness did not continue after it.
    #[serde(default)]
    pub failed: bool,
    /// The harness's own line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
