//! Every method of the protocol (PROTOCOL.md §3–§6): its name, and for a request its
//! parameters, its result and how long the runtime waits for it (§9), once, as a
//! [`Method`]. A program dispatches on the names; the runtime calls through the types.

use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::refusal;
use crate::wire::{self, harness, host, hosts, process};

/// A request of the protocol.
pub trait Method {
    /// Its name on the wire.
    const NAME: &'static str;
    /// How long the runtime waits for its answer (PROTOCOL.md §9). `host/call`'s is what it
    /// adds to the bound of the method it carries ([`carried`]).
    const BOUND: Duration;
    /// The refusal words it may answer with (PROTOCOL.md §4–§6, §8): a refusal with one of
    /// these is an answer the protocol gives it, any other a program's fault.
    const REFUSES: &'static [&'static str];
    /// The first protocol version that has it (PROTOCOL.md §10): a runtime asks it only of
    /// a program that picked this version or a newer one.
    const SINCE: u32 = 3;
    /// Its parameters.
    type Params: Serialize + DeserializeOwned;
    /// Its result.
    type Result: Serialize + DeserializeOwned;
}

const fn seconds(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

/// Every request: its name's const, its marker, its parameters, result, bound, refusal
/// words and, where it is newer than version 3, the version it came in; and [`bound`],
/// which reads the same rows by name.
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

        /// How long the runtime waits for the request named `name` (PROTOCOL.md §9);
        /// `None` for a name that is no request of the protocol.
        pub fn bound(name: &str) -> Option<Duration> {
            match name {
                $($value => Some($bound),)*
                _ => None,
            }
        }
    };
}

requests! {
    /// The runtime's first request (§3).
    Initialize, INITIALIZE = "initialize": wire::InitializeParams => wire::Initialized, seconds(10), [];
    /// The runtime ends a program (§2).
    Shutdown, SHUTDOWN = "shutdown": wire::Empty => wire::Empty, seconds(10), [];

    /// The entries that set the harness up (§4.1).
    SetupPlan, SETUP_PLAN = "setup/plan": harness::Setup => harness::Planned, seconds(30), [];
    /// The conversations updated since a time (§4.1).
    SessionsList, SESSIONS_LIST = "sessions/list": harness::List => harness::Listed, seconds(30), [];
    /// One conversation by its id (§4.1).
    SessionsLocate, SESSIONS_LOCATE = "sessions/locate": harness::Locate => harness::Located, seconds(10),
        [NOT_FOUND];
    /// A conversation's events after a cursor (§4.1).
    SessionsRead, SESSIONS_READ = "sessions/read": harness::Read => harness::Events, seconds(10),
        [NOT_FOUND, REFUSED];
    /// The events of records a plugin door carried (§4.1).
    SessionsFold, SESSIONS_FOLD = "sessions/fold": harness::Fold => harness::Folded, seconds(10),
        [SELECTION_UNSUPPORTED];
    /// One hook's document as a moment (§4.1).
    HookRead, HOOK_READ = "hook/read": harness::HookRead => harness::HookMoment, seconds(10), [];
    /// The runtime's answer as what a command hook prints (§4.1).
    HookAnswer, HOOK_ANSWER = "hook/answer": harness::HookAnswer => harness::HookAnswered, seconds(10), [];
    /// The MCP servers the harness loads, for the person or in one folder (§4.1).
    McpList, MCP_LIST = "mcp/list": harness::McpList => harness::McpServers, seconds(30), [] since 4;
    /// Who holds a conversation (§4.2).
    Held, HELD = "held": harness::Held => harness::HeldBy, seconds(10), [];
    /// What the driver can prove of a conversation now (§4.2).
    Idle, IDLE = "idle": harness::Held => harness::Sampled, seconds(20), [];
    /// The models the harness runs here (§4.2).
    Models, MODELS = "models": wire::Empty => harness::Models, seconds(30), [NO_BINARY, REFUSED];
    /// A turn's launch plan (§4.2).
    Start, START = "start": harness::Start => harness::Launch, seconds(10),
        [NOT_FOUND, NO_PLACE, MODEL_MISSING, SELECTION_UNSUPPORTED, NO_BINARY, START_FAILED];
    /// One line of a turn's output (§4.2).
    TurnRead, TURN_READ = "turn/read": harness::TurnRead => harness::TurnLine, seconds(10), [];
    /// The harness's own graceful stop (§4.2).
    Stop, STOP = "stop": harness::Stop => harness::Stopped, Duration::from_millis(2_800), [NOT_STOPPED];
    /// A point in a conversation before words are sent (§4.2).
    ReceiptMark, RECEIPT_MARK = "receipt/mark": harness::ReceiptMark => harness::Marked, seconds(10),
        [SELECTION_UNSUPPORTED];
    /// Whether words carrying a delivery went in after a mark (§4.2).
    ReceiptFind, RECEIPT_FIND = "receipt/find": harness::ReceiptFind => harness::Receipt, seconds(20),
        [REFUSED];
    /// A one-shot read's plan (§4.2).
    Reader, READER = "reader": harness::Reader => harness::ReaderPlan, seconds(10), [NO_BINARY, MODEL_MISSING];
    /// The text in a reader's events (§4.2).
    ReaderRead, READER_READ = "reader/read": harness::ReaderRead => harness::Text, seconds(10), [NO_ANSWER];
    /// The command line that opens a conversation in a terminal (§4.2).
    Terminal, TERMINAL = "terminal": harness::Terminal => harness::TerminalCommand, seconds(10), [NOT_FOUND];
    /// Words into a live conversation through the harness's own door (§4.3).
    Say, SAY = "say": harness::Say => harness::Said, seconds(10), [NO_DOOR, NO_ANSWER, HARNESS_BUSY, REFUSED];
    /// A model or thinking level for a live conversation (§4.3).
    Pick, PICK = "pick": harness::Pick => harness::Picked, seconds(10), [MODEL_MISSING, SELECTION_UNSUPPORTED];
    /// A recap written by the conversation's own model (§4.3).
    Recap, RECAP = "recap": harness::Recap => harness::Text, seconds(40), [NO_ANSWER, MODEL_MISSING];

    /// Start a process detached (§5.1).
    ProcessStart, PROCESS_START = "process/start": process::Start => process::Started, seconds(5), [START_FAILED];
    /// The processes running (§5.1).
    ProcessList, PROCESS_LIST = "process/list": process::List => process::Listed, seconds(5), [];
    /// Some of a process's environment (§5.1).
    ProcessEnviron, PROCESS_ENVIRON = "process/environ": process::Environ => process::Values, seconds(5),
        [NOT_FOUND];
    /// The files a process holds open (§5.1).
    ProcessFiles, PROCESS_FILES = "process/files": process::Files => process::Paths, seconds(5), [NOT_FOUND];
    /// The processes listening on a TCP port (§5.1).
    ProcessListening, PROCESS_LISTENING = "process/listening": process::Listening => process::Listeners,
        seconds(5), [REFUSED];
    /// Signal a process (§5.1).
    ProcessSignal, PROCESS_SIGNAL = "process/signal": process::Signal => process::Sent, seconds(5), [];
    /// What installs the runtime as a service (§5.1).
    ServicePlan, SERVICE_PLAN = "service/plan": process::ServicePlan => process::Plan, seconds(10), [];
    /// Where a service stands (§5.1).
    ServiceStatus, SERVICE_STATUS = "service/status": process::Service => process::Status, seconds(10), [REFUSED];
    /// Start a service (§5.1).
    ServiceStart, SERVICE_START = "service/start": process::Service => wire::Empty, seconds(30), [REFUSED];
    /// Stop a service and start it again (§5.1).
    ServiceRestart, SERVICE_RESTART = "service/restart": process::Service => wire::Empty, seconds(30), [REFUSED];
    /// Stop and remove a service (§5.1).
    ServiceRemove, SERVICE_REMOVE = "service/remove": process::Service => wire::Empty, seconds(30), [REFUSED];

    /// The adapter's hosts (§5.2).
    HostsList, HOSTS_LIST = "hosts/list": hosts::List => hosts::Listed, seconds(60),
        [OFFLINE, REFUSED];
    /// The host for a key, made where none is (§5.2).
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
    /// Archive a host, where the adapter can (§5.2).
    HostsArchive, HOSTS_ARCHIVE = "hosts/archive": hosts::Target => hosts::State, seconds(300),
        [OFFLINE, REFUSED, NOT_FOUND];
    /// Pause a host, where the adapter can (§5.2).
    HostsPause, HOSTS_PAUSE = "hosts/pause": hosts::Target => hosts::State, seconds(60),
        [OFFLINE, REFUSED, NOT_FOUND];
    /// Fork a host, where the adapter can (§5.2).
    HostsFork, HOSTS_FORK = "hosts/fork": hosts::Target => hosts::Named, seconds(60),
        [OFFLINE, REFUSED, NOT_FOUND, START_FAILED];
    /// Keep a host from going idle, where the adapter can (§5.2).
    HostsKeepalive, HOSTS_KEEPALIVE = "hosts/keepalive": hosts::Target => wire::Empty, seconds(60),
        [OFFLINE, REFUSED, NOT_FOUND];
    /// A URL every other host of the adapter reaches a host's port by, where the adapter can
    /// (§5.2).
    HostsExpose, HOSTS_EXPOSE = "hosts/expose": hosts::Expose => hosts::Exposed, seconds(60),
        [OFFLINE, REFUSED, NOT_FOUND];
    /// Write a file into a host (§5.2).
    HostsPut, HOSTS_PUT = "hosts/put": hosts::Put => wire::Empty, seconds(300), [OFFLINE, NOT_FOUND, REFUSED];
    /// A way to a command's standard input and output in a host (§5.2).
    HostsChannel, HOSTS_CHANNEL = "hosts/channel": hosts::Channel => hosts::Reach, seconds(60),
        [OFFLINE, REFUSED, NOT_FOUND];
    /// The runtime's bytes for a `stream` channel (§5.3).
    StreamWrite, STREAM_WRITE = "stream/write": hosts::StreamWrite => wire::Empty, seconds(60), [];
    /// The runtime ends a `stream` channel (§5.3).
    StreamClose, STREAM_CLOSE = "stream/close": hosts::StreamClose => wire::Empty, seconds(10), [];

    /// The channel's first request (§6.1).
    HostHello, HOST_HELLO = "host/hello": host::Hello => host::Welcome, seconds(10), [];
    /// Whether the channel still answers (§6.1).
    HostPing, HOST_PING = "host/ping": wire::Empty => wire::Empty, seconds(30), [];
    /// What is at each path (§6.2).
    HostFsStat, HOST_FS_STAT = "host/fs/stat": host::FsStat => host::FsStats, seconds(30), [NOT_FOUND, REFUSED];
    /// Bytes of a file (§6.2).
    HostFsRead, HOST_FS_READ = "host/fs/read": host::FsRead => host::FsBytes, seconds(30), [NOT_FOUND, REFUSED];
    /// What a folder holds (§6.2).
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
    /// The host's programs, as each answered `initialize` (§6.3).
    HostPrograms, HOST_PROGRAMS = "host/programs": wire::Empty => host::Programs, seconds(30), [OFFLINE];
    /// A program's method, on the host: answered with what the program answered (§6.3), or
    /// refused `offline` where the program is missing or restarting. A refusal of the method
    /// it carries is that method's own.
    HostCall, HOST_CALL = "host/call": host::Call => Value, seconds(5), [NO_ANSWER, OFFLINE];
    /// A connection on the host (§6.4).
    HostDial, HOST_DIAL = "host/dial": host::Dial => wire::Empty, seconds(30), [NO_DOOR, REFUSED];
}

/// How long control waits for a `host/call` carrying the request named `name`: its own
/// bound, and 5 s. `None` for a name that is no request.
pub fn carried(name: &str) -> Option<Duration> {
    bound(name).map(|bound| bound + HostCall::BOUND)
}

/// A notification a driver with its own door sends unasked (§4.3).
pub const SESSION_CHANGED: &str = "session/changed";
/// A notification: the adapter's bytes from a `stream` channel (§5.3).
pub const STREAM_DATA: &str = "stream/data";
/// A notification: a `stream` channel ended (§5.3).
pub const STREAM_CLOSED: &str = "stream/closed";
/// A notification: a host program's notification (§6.3).
pub const HOST_NOTIFIED: &str = "host/notified";
/// A notification: a host program exited (§6.3).
pub const HOST_EXITED: &str = "host/exited";
/// A notification: the host's listener took a connection (§6.4).
pub const HOST_OPENED: &str = "host/opened";
/// A notification, both ways: a connection's bytes (§6.4).
pub const HOST_DATA: &str = "host/data";
/// A notification, both ways: a connection's bytes passed on (§6.4).
pub const HOST_CREDIT: &str = "host/credit";
/// A notification, both ways: a connection's side ends (§6.4).
pub const HOST_CLOSE: &str = "host/close";
