//! Each method of the protocol (PROTOCOL.md §3–§6), once, as a [`Method`]. A [`Method`]
//! holds the name. For a request, it also holds the parameters, the result and how long the
//! runtime waits (§9). A program dispatches on the names. The runtime calls through the types.

use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::refusal;
use crate::wire::{self, harness, host, hosts, process};

/// A request of the protocol.
pub trait Method {
    /// The name on the wire.
    const NAME: &'static str;
    /// How long the runtime waits for the answer (PROTOCOL.md §9). For `host/call`, this is
    /// the time that it adds to the bound of the method that it carries ([`carried`]).
    const BOUND: Duration;
    /// The refusal words that the request can answer with (PROTOCOL.md §4–§6, §8). A refusal
    /// with one of these words is an answer that the protocol allows for this request. Any
    /// other refusal is a fault of the program.
    const REFUSES: &'static [&'static str];
    /// The first protocol version that has the request (PROTOCOL.md §10). A runtime sends it
    /// only to a program that picked this version or a newer one.
    const SINCE: u32 = 3;
    /// The parameters.
    type Params: Serialize + DeserializeOwned;
    /// The result.
    type Result: Serialize + DeserializeOwned;
}

const fn seconds(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

/// Each request, as one row: the const of its name, its marker, its parameters, result,
/// bound and refusal words. A request newer than version 3 also has the version that added
/// it. The macro also makes [`bound`], which reads the same rows by name.
macro_rules! requests {
    ($(
        $(#[$doc:meta])*
        $marker:ident, $name:ident = $value:literal: $params:ty => $result:ty, $bound:expr,
            [$($refuses:ident),*] $(since $since:literal)?;
    )*) => {
        $(pub const $name: &str = $value;)*

        $(
            $(#[$doc])*
            pub enum $marker {}

            impl Method for $marker {
                const NAME: &'static str = $name;
                const BOUND: Duration = $bound;
                const REFUSES: &'static [&'static str] = &[$(refusal::$refuses),*];
                $(const SINCE: u32 = $since;)?
                type Params = $params;
                type Result = $result;
            }
        )*

        /// How long the runtime waits for the request named `name` (PROTOCOL.md §9).
        /// `None` for a name that is not a request of the protocol.
        pub fn bound(name: &str) -> Option<Duration> {
            match name {
                $($value => Some($bound),)*
                _ => None,
            }
        }
    };
}

requests! {
    /// The first request of the runtime (§3).
    Initialize, INITIALIZE = "initialize": wire::InitializeParams => wire::Initialized, seconds(10), [];
    /// The runtime ends a program (§2).
    Shutdown, SHUTDOWN = "shutdown": wire::Empty => wire::Empty, seconds(10), [];

    /// The entries that configure the harness (§4.1).
    SetupPlan, SETUP_PLAN = "setup/plan": harness::Setup => harness::Planned, seconds(30), [];
    /// What a person gave for one need of a plan (§4.1).
    NeedsGive, NEEDS_GIVE = "needs/give": harness::Give => harness::Given, seconds(30),
        [NOT_FOUND, REFUSED] since 5;
    /// The conversations updated since a time (§4.1).
    SessionsList, SESSIONS_LIST = "sessions/list": harness::List => harness::Listed, seconds(30), [];
    /// One conversation by its id (§4.1).
    SessionsLocate, SESSIONS_LOCATE = "sessions/locate": harness::Locate => harness::Located, seconds(10),
        [NOT_FOUND];
    /// The events of a conversation after a cursor (§4.1).
    SessionsRead, SESSIONS_READ = "sessions/read": harness::Read => harness::Events, seconds(10),
        [NOT_FOUND, REFUSED];
    /// The events of records that a plugin door sent (§4.1).
    SessionsFold, SESSIONS_FOLD = "sessions/fold": harness::Fold => harness::Folded, seconds(10),
        [SELECTION_UNSUPPORTED];
    /// The document of one hook, as a moment (§4.1).
    HookRead, HOOK_READ = "hook/read": harness::HookRead => harness::HookMoment, seconds(10), [];
    /// The answer of the runtime, as the text that a command hook prints (§4.1).
    HookAnswer, HOOK_ANSWER = "hook/answer": harness::HookAnswer => harness::HookAnswered, seconds(10), [];
    /// The MCP servers the harness loads, for the person or in one folder (§4.1).
    McpList, MCP_LIST = "mcp/list": harness::McpList => harness::McpServers, seconds(30),
        [NO_BINARY, REFUSED] since 4;
    /// Who holds a conversation (§4.2).
    Held, HELD = "held": harness::Held => harness::HeldBy, seconds(10), [];
    /// What the driver can prove about a conversation now (§4.2).
    Idle, IDLE = "idle": harness::Held => harness::Sampled, seconds(20), [];
    /// The models that the harness runs on this machine (§4.2).
    Models, MODELS = "models": wire::Empty => harness::Models, seconds(30), [NO_BINARY, REFUSED];
    /// The launch plan of a turn (§4.2).
    Start, START = "start": harness::Start => harness::Launch, seconds(10),
        [NOT_FOUND, NO_PLACE, MODEL_MISSING, SELECTION_UNSUPPORTED, NO_BINARY, START_FAILED];
    /// One line of the output of a turn (§4.2).
    TurnRead, TURN_READ = "turn/read": harness::TurnRead => harness::TurnLine, seconds(10), [];
    /// The graceful stop of the harness itself (§4.2).
    Stop, STOP = "stop": harness::Stop => harness::Stopped, Duration::from_millis(2_800), [NOT_STOPPED, OTHER_TURN];
    /// A point in a conversation, before the runtime sends words (§4.2).
    ReceiptMark, RECEIPT_MARK = "receipt/mark": harness::ReceiptMark => harness::Marked, seconds(10),
        [SELECTION_UNSUPPORTED];
    /// Whether words with a delivery went in after a mark (§4.2).
    ReceiptFind, RECEIPT_FIND = "receipt/find": harness::ReceiptFind => harness::Receipt, seconds(20),
        [REFUSED];
    /// The plan of a one-shot read (§4.2).
    Reader, READER = "reader": harness::Reader => harness::ReaderPlan, seconds(10), [NO_BINARY, MODEL_MISSING];
    /// The text in the events of a reader (§4.2).
    ReaderRead, READER_READ = "reader/read": harness::ReaderRead => harness::Text, seconds(10), [NO_ANSWER];
    /// The command line that opens a conversation in a terminal (§4.2).
    Terminal, TERMINAL = "terminal": harness::Terminal => harness::TerminalCommand, seconds(10), [NOT_FOUND];
    /// Words into a live conversation, through the door of the harness (§4.3).
    Say, SAY = "say": harness::Say => harness::Said, seconds(10), [NO_DOOR, NO_ANSWER, HARNESS_BUSY, REFUSED];
    /// A model or thinking level for a live conversation (§4.3).
    Pick, PICK = "pick": harness::Pick => harness::Picked, seconds(10), [MODEL_MISSING, SELECTION_UNSUPPORTED];

    /// Start a process detached (§5.1).
    ProcessStart, PROCESS_START = "process/start": process::Start => process::Started, seconds(5), [START_FAILED];
    /// The processes that run (§5.1).
    ProcessList, PROCESS_LIST = "process/list": process::List => process::Listed, seconds(5), [];
    /// Some of the environment of a process (§5.1).
    ProcessEnviron, PROCESS_ENVIRON = "process/environ": process::Environ => process::Values, seconds(5),
        [NOT_FOUND];
    /// The files that a process holds open (§5.1).
    ProcessFiles, PROCESS_FILES = "process/files": process::Files => process::Paths, seconds(5), [NOT_FOUND];
    /// The processes that listen on a TCP port (§5.1).
    ProcessListening, PROCESS_LISTENING = "process/listening": process::Listening => process::Listeners,
        seconds(5), [REFUSED];
    /// Signal a process (§5.1).
    ProcessSignal, PROCESS_SIGNAL = "process/signal": process::Signal => process::Sent, seconds(5), [];
    /// What installs the runtime as a service (§5.1).
    ServicePlan, SERVICE_PLAN = "service/plan": process::ServicePlan => process::Plan, seconds(10), [];
    /// The state of a service (§5.1).
    ServiceStatus, SERVICE_STATUS = "service/status": process::Service => process::Status, seconds(10), [REFUSED];
    /// Start a service (§5.1).
    ServiceStart, SERVICE_START = "service/start": process::Service => wire::Empty, seconds(30), [REFUSED];
    /// Stop a service and start it again (§5.1).
    ServiceRestart, SERVICE_RESTART = "service/restart": process::Service => wire::Empty, seconds(30), [REFUSED];
    /// Stop and remove a service (§5.1).
    ServiceRemove, SERVICE_REMOVE = "service/remove": process::Service => wire::Empty, seconds(30), [REFUSED];

    /// The hosts of the adapter (§5.2).
    HostsList, HOSTS_LIST = "hosts/list": hosts::List => hosts::Listed, seconds(60),
        [OFFLINE, REFUSED];
    /// The host for a key. The adapter makes one when none exists (§5.2).
    HostsEnsure, HOSTS_ENSURE = "hosts/ensure": hosts::Ensure => hosts::Named, seconds(300),
        [OFFLINE, REFUSED, START_FAILED, NO_PLACE];
    /// Start a host (§5.2).
    HostsStart, HOSTS_START = "hosts/start": hosts::Target => hosts::State, seconds(300),
        [OFFLINE, REFUSED, NOT_FOUND, START_FAILED];
    /// Stop a host (§5.2).
    HostsStop, HOSTS_STOP = "hosts/stop": hosts::Target => hosts::State, seconds(60),
        [OFFLINE, REFUSED, NOT_FOUND];
    /// Delete a host (§5.2).
    HostsDelete, HOSTS_DELETE = "hosts/delete": hosts::Target => hosts::State, seconds(60),
        [OFFLINE, REFUSED, NOT_FOUND];
    /// Archive a host, when the adapter can (§5.2).
    HostsArchive, HOSTS_ARCHIVE = "hosts/archive": hosts::Target => hosts::State, seconds(300),
        [OFFLINE, REFUSED, NOT_FOUND];
    /// Pause a host, when the adapter can (§5.2).
    HostsPause, HOSTS_PAUSE = "hosts/pause": hosts::Target => hosts::State, seconds(60),
        [OFFLINE, REFUSED, NOT_FOUND];
    /// Fork a host, when the adapter can (§5.2).
    HostsFork, HOSTS_FORK = "hosts/fork": hosts::Target => hosts::Named, seconds(60),
        [OFFLINE, REFUSED, NOT_FOUND, START_FAILED];
    /// Prevent idle state on a host, when the adapter can (§5.2).
    HostsKeepalive, HOSTS_KEEPALIVE = "hosts/keepalive": hosts::Target => wire::Empty, seconds(60),
        [OFFLINE, REFUSED, NOT_FOUND];
    /// A URL through which each other host of the adapter reaches a port of a host, when the
    /// adapter can (§5.2).
    HostsExpose, HOSTS_EXPOSE = "hosts/expose": hosts::Expose => hosts::Exposed, seconds(60),
        [OFFLINE, REFUSED, NOT_FOUND];
    /// Write a file into a host (§5.2).
    HostsPut, HOSTS_PUT = "hosts/put": hosts::Put => wire::Empty, seconds(300), [OFFLINE, NOT_FOUND, REFUSED];
    /// A way to the standard input and output of a command in a host (§5.2).
    HostsChannel, HOSTS_CHANNEL = "hosts/channel": hosts::Channel => hosts::Reach, seconds(60),
        [OFFLINE, REFUSED, NOT_FOUND];
    /// The bytes of the runtime for a `stream` channel (§5.3).
    StreamWrite, STREAM_WRITE = "stream/write": hosts::StreamWrite => wire::Empty, seconds(60), [];
    /// The runtime ends a `stream` channel (§5.3).
    StreamClose, STREAM_CLOSE = "stream/close": hosts::StreamClose => wire::Empty, seconds(10), [];

    /// The first request of the channel (§6.1).
    HostHello, HOST_HELLO = "host/hello": host::Hello => host::Welcome, seconds(10), [];
    /// Whether the channel still answers (§6.1).
    HostPing, HOST_PING = "host/ping": wire::Empty => wire::Empty, seconds(30), [];
    /// What is at each path (§6.2).
    HostFsStat, HOST_FS_STAT = "host/fs/stat": host::FsStat => host::FsStats, seconds(30), [NOT_FOUND, REFUSED];
    /// Bytes of a file (§6.2).
    HostFsRead, HOST_FS_READ = "host/fs/read": host::FsRead => host::FsBytes, seconds(30), [NOT_FOUND, REFUSED];
    /// What a folder contains (§6.2).
    HostFsList, HOST_FS_LIST = "host/fs/list": host::FsList => host::FsEntries, seconds(30), [NOT_FOUND, REFUSED];
    /// Replace a file whole and atomically (§6.2).
    HostFsWrite, HOST_FS_WRITE = "host/fs/write": host::FsWrite => wire::Empty, seconds(30), [NOT_FOUND, REFUSED];
    /// Add to the end of a file (§6.2).
    HostFsAppend, HOST_FS_APPEND = "host/fs/append": host::FsWrite => wire::Empty, seconds(30),
        [NOT_FOUND, REFUSED];
    /// Make a folder and its parents (§6.2).
    HostFsMkdir, HOST_FS_MKDIR = "host/fs/mkdir": host::FsMkdir => wire::Empty, seconds(30), [NOT_FOUND, REFUSED];
    /// Remove a file or a folder (§6.2).
    HostFsRemove, HOST_FS_REMOVE = "host/fs/remove": host::FsRemove => wire::Empty, seconds(30),
        [NOT_FOUND, REFUSED];
    /// The programs of the host, as each answered `initialize` (§6.3).
    HostPrograms, HOST_PROGRAMS = "host/programs": wire::Empty => host::Programs, seconds(30), [OFFLINE];
    /// A method of a program, on the host. The answer is the answer of the program (§6.3).
    /// The refusal is `offline` when the program is missing or restarts. A refusal of the
    /// carried method is the refusal of that method.
    HostCall, HOST_CALL = "host/call": host::Call => Value, seconds(5), [NO_ANSWER, OFFLINE];
    /// A connection on the host (§6.4).
    HostDial, HOST_DIAL = "host/dial": host::Dial => wire::Empty, seconds(30), [NO_DOOR, REFUSED];
}

/// How long control waits for a `host/call` that carries the request named `name`: the
/// bound of that request, plus 5 s. `None` for a name that is not a request.
pub fn carried(name: &str) -> Option<Duration> {
    bound(name).map(|bound| bound + HostCall::BOUND)
}

/// A notification that a driver with its own door sends without a request (§4.3).
pub const SESSION_CHANGED: &str = "session/changed";
/// A notification that a driver whose facts say `touches` sends without a request: the
/// conversations that changed (§4.1).
pub const SESSIONS_TOUCHED: &str = "sessions/touched";
/// A notification: the bytes of the adapter from a `stream` channel (§5.3).
pub const STREAM_DATA: &str = "stream/data";
/// A notification: a `stream` channel ended (§5.3).
pub const STREAM_CLOSED: &str = "stream/closed";
/// A notification: a notification of a host program (§6.3).
pub const HOST_NOTIFIED: &str = "host/notified";
/// A notification: a host program exited (§6.3).
pub const HOST_EXITED: &str = "host/exited";
/// A notification: the listener of the host accepted a connection (§6.4).
pub const HOST_OPENED: &str = "host/opened";
/// A notification, both ways: the bytes of a connection (§6.4).
pub const HOST_DATA: &str = "host/data";
/// A notification, both ways: the bytes of a connection that the receiver forwarded (§6.4).
pub const HOST_CREDIT: &str = "host/credit";
/// A notification, both ways: one side of a connection ends (§6.4).
pub const HOST_CLOSE: &str = "host/close";
