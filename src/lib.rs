//! Steery's program protocol ([PROTOCOL.md](https://github.com/steery-dev/sdk/blob/main/PROTOCOL.md)):
//! the one contract between Steery's runtime and every driver and adapter.
//!
//! A program is a child process speaking JSON-RPC 2.0 on its standard input and output,
//! one JSON object per line. [`serve`] is that loop for a program written in Rust: it
//! answers `initialize` and `shutdown`, keeps the answers to the last 256 ids, and hands
//! every other method to a [`Program`]. [`client::Client`] is the runtime's side. [`wire`]
//! holds every shape either side reads, [`refusal`] the only words a program refuses
//! with, and [`release`] the manifest a release of programs is published by.

pub mod client;
mod frame;
pub mod release;
pub mod wire;

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// Every protocol version this crate speaks, oldest first: 4 is 3 with `mcp/list`
/// (PROTOCOL.md §10). A program answers the newest of these the runtime offers.
pub const PROTOCOLS: &[u32] = &[3, 4];

/// The version a program answers `initialize` with: the newest of [`PROTOCOLS`] in
/// `offered`, or none where the runtime offers none of them.
pub fn pick(offered: &[u32]) -> Option<u32> {
    PROTOCOLS
        .iter()
        .rev()
        .copied()
        .find(|version| offered.contains(version))
}

/// The words a program refuses with (PROTOCOL.md §8). The runtime and the server own what
/// each one means; an unknown word reaches the person.
pub mod refusal {
    /// A live writer is in the conversation.
    pub const HARNESS_BUSY: &str = "harness-busy";
    /// The harness's store has no such conversation.
    pub const NOT_FOUND: &str = "not-found";
    /// The folder the conversation ran in is gone.
    pub const NO_PLACE: &str = "no-place";
    /// A door took no connection, or not the whole line, in time.
    pub const NO_DOOR: &str = "no-door";
    /// The door took the words and said nothing back.
    pub const NO_ANSWER: &str = "no-answer";
    /// The harness cannot apply a model or thinking pick.
    pub const SELECTION_UNSUPPORTED: &str = "selection-unsupported";
    /// A picked model is not on the harness's list here.
    pub const MODEL_MISSING: &str = "model-missing";
    /// A writer the runtime did not start is in the conversation, and it has no door.
    pub const HELD: &str = "held";
    /// The machine is out of reach.
    pub const OFFLINE: &str = "offline";
    /// The harness refuses a folder its person has not trusted.
    pub const UNTRUSTED: &str = "untrusted";
    /// The harness is not on the machine.
    pub const NO_BINARY: &str = "no-binary";
    /// The process would not start, or the harness said something no other word says.
    pub const START_FAILED: &str = "start-failed";
    /// The turn ended without its harness naming a conversation.
    pub const NO_SESSION: &str = "no-session";
    /// A stop that could not end what ran: something still runs.
    pub const NOT_STOPPED: &str = "not-stopped";
    /// Anything else.
    pub const REFUSED: &str = "refused";

    /// Every word above.
    pub const ALL: &[&str] = &[
        HARNESS_BUSY,
        NOT_FOUND,
        NO_PLACE,
        NO_DOOR,
        NO_ANSWER,
        SELECTION_UNSUPPORTED,
        MODEL_MISSING,
        HELD,
        OFFLINE,
        UNTRUSTED,
        NO_BINARY,
        START_FAILED,
        NO_SESSION,
        NOT_STOPPED,
        REFUSED,
    ];
}

pub mod method;

/// The JSON-RPC error code of a refusal.
pub const REFUSED_CODE: i64 = -32001;
/// The JSON-RPC error code of a method the program does not have.
pub const UNKNOWN_CODE: i64 = -32601;
/// The JSON-RPC error code of parameters the program cannot read.
pub const INVALID_CODE: i64 = -32602;
/// The JSON-RPC error code of a message that is no request: not JSON, too long, or sent
/// before `initialize`.
pub const BAD_REQUEST_CODE: i64 = -32600;

/// Why a program does not do what it is asked.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A refusal: `word` is one of [`refusal`], `line` one line for a person.
    #[error("refused: {word}")]
    Refused { word: String, line: Option<String> },
    /// The program has no such method.
    #[error("unknown method")]
    Unknown,
    /// The parameters do not read as the method's.
    #[error("invalid: {0}")]
    Invalid(String),
}

impl Error {
    /// A refusal with one line for a person; an empty line is none.
    pub fn refused(word: &'static str, line: impl Into<String>) -> Self {
        let line = line.into();
        Self::Refused {
            word: word.to_owned(),
            line: (!line.is_empty()).then_some(line),
        }
    }

    /// Parameters the program cannot read, and why.
    pub fn invalid(why: impl Into<String>) -> Self {
        Self::Invalid(why.into())
    }

    fn to_json(&self) -> Value {
        match self {
            Self::Refused { word, line } => {
                let mut data = json!({ "refused": word });
                if let Some(line) = line {
                    data["line"] = json!(line);
                }
                json!({ "code": REFUSED_CODE, "message": self.to_string(), "data": data })
            }
            Self::Unknown => json!({ "code": UNKNOWN_CODE, "message": self.to_string() }),
            Self::Invalid(why) => json!({ "code": INVALID_CODE, "message": why }),
        }
    }
}

/// A method's parameters, read as `T`.
pub fn params<T: DeserializeOwned>(value: Value) -> Result<T, Error> {
    serde_json::from_value(value).map_err(|e| Error::Invalid(e.to_string()))
}

/// A method's result, written as JSON.
pub fn result<T: Serialize>(value: T) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|e| Error::Invalid(e.to_string()))
}

/// Sends a program's notifications to the runtime, in order with its answers.
#[derive(Clone)]
pub struct Notifier {
    out: mpsc::UnboundedSender<Option<Vec<u8>>>,
}

impl Notifier {
    /// Send `method` with `params`. Dropped once the runtime has closed the program.
    pub fn notify(&self, method: &str, params: Value) {
        let message = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        let _ = self.out.send(Some(frame::encode(&message)));
    }
}

/// A driver or an adapter: what it is, and what it does when asked.
pub trait Program: Send + Sync + 'static {
    /// What the program is, for the runtime's `initialize`. It names one of
    /// `params.protocol`.
    fn initialize(&self, params: &wire::InitializeParams) -> Result<wire::Initialized, Error>;

    /// Any other method. One the program does not have is [`Error::Unknown`].
    fn call(
        &self,
        method: &str,
        params: Value,
    ) -> impl Future<Output = Result<Value, Error>> + Send;
}

/// How many answers a program keeps for an id sent again.
const MEMO: usize = 256;

/// One id's call: the method and parameters it was sent with, and its answer once there
/// is one, or how many more times it is owed.
struct Call {
    method: String,
    params: Value,
    answer: Answer,
}

enum Answer {
    Owed { again: usize },
    Given(Vec<u8>),
}

#[derive(Default)]
struct Memo {
    calls: HashMap<String, Call>,
    order: VecDeque<String>,
}

impl Memo {
    fn remember(&mut self, key: String, call: Call) {
        self.order.push_back(key.clone());
        self.calls.insert(key, call);
        while self.order.len() > MEMO {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            // An answer still owed is kept past the bound: it is forgotten once given.
            if matches!(
                self.calls.get(&oldest).map(|call| &call.answer),
                Some(Answer::Given(_))
            ) {
                self.calls.remove(&oldest);
            } else {
                self.order.push_back(oldest);
                break;
            }
        }
    }
}

/// Serve `make`'s program on standard input and output until `shutdown` or until the
/// runtime closes standard input. Every call already sent is answered first, and
/// `shutdown` last.
pub async fn serve<P: Program>(make: impl FnOnce(Notifier) -> P) -> std::io::Result<()> {
    let (out, mut lines) = mpsc::unbounded_channel::<Option<Vec<u8>>>();
    let writer = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(Some(line)) = lines.recv().await {
            stdout.write_all(&line).await?;
            stdout.flush().await?;
        }
        stdout.flush().await
    });
    let program = Arc::new(make(Notifier { out: out.clone() }));
    let memo = Arc::new(Mutex::new(Memo::default()));
    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut line = Vec::new();
    let mut initialized = false;
    // Every call still running: `shutdown` is answered once each has been.
    let mut calls = tokio::task::JoinSet::new();
    let mut shutdown = None;
    let send = |message: Value| {
        let _ = out.send(Some(frame::encode(&message)));
    };
    while let Some(read) = frame::next(&mut stdin, &mut line).await? {
        while calls.try_join_next().is_some() {}
        let message = match read {
            frame::Read::TooLong => {
                send(bad_request(Value::Null, "the line is over 16 MiB"));
                continue;
            }
            frame::Read::Line => {
                if line.iter().all(u8::is_ascii_whitespace) {
                    continue;
                }
                match serde_json::from_slice::<Value>(&line) {
                    Ok(message) => message,
                    Err(e) => {
                        send(bad_request(Value::Null, &format!("not JSON: {e}")));
                        continue;
                    }
                }
            }
        };
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            // Answers to requests a program never sends, in this version.
            continue;
        };
        let method = method.to_owned();
        let params = match message.get("params") {
            Some(params) if !params.is_null() => params.clone(),
            _ => json!({}),
        };
        let Some(id) = message.get("id").filter(|id| !id.is_null()).cloned() else {
            // A notification from the runtime: done, and answered by nobody.
            if initialized {
                let program = program.clone();
                calls.spawn(async move {
                    let _ = program.call(&method, params).await;
                });
            }
            continue;
        };
        if method == method::SHUTDOWN {
            shutdown = Some(id);
            break;
        }
        if !initialized && method != method::INITIALIZE {
            send(bad_request(id, "`initialize` comes first"));
            continue;
        }
        let key = id.to_string();
        {
            let mut memo = memo.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(call) = memo.calls.get_mut(&key) {
                if call.method != method || call.params != params {
                    send(bad_request(id, "this id was sent with another call"));
                } else {
                    match &mut call.answer {
                        Answer::Given(line) => {
                            let _ = out.send(Some(line.clone()));
                        }
                        Answer::Owed { again } => *again += 1,
                    }
                }
                continue;
            }
            memo.remember(
                key.clone(),
                Call {
                    method: method.clone(),
                    params: params.clone(),
                    answer: Answer::Owed { again: 0 },
                },
            );
        }
        if method == method::INITIALIZE {
            let answered = crate::params::<wire::InitializeParams>(params)
                .and_then(|params| program.initialize(&params))
                .and_then(result);
            initialized |= answered.is_ok();
            give(&memo, &out, key, &id, answered);
            continue;
        }
        let program = program.clone();
        let memo = memo.clone();
        let out = out.clone();
        calls.spawn(async move {
            let answered = program.call(&method, params).await;
            give(&memo, &out, key, &id, answered);
        });
    }
    while calls.join_next().await.is_some() {}
    if let Some(id) = shutdown {
        send(json!({ "jsonrpc": "2.0", "id": id, "result": {} }));
    }
    let _ = out.send(None);
    match writer.await {
        Ok(written) => written,
        Err(e) => Err(std::io::Error::other(e)),
    }
}

/// Answer `id`, as many times as it was sent, and keep the answer for its next sending.
fn give(
    memo: &Mutex<Memo>,
    out: &mpsc::UnboundedSender<Option<Vec<u8>>>,
    key: String,
    id: &Value,
    answered: Result<Value, Error>,
) {
    let message = match answered {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error.to_json() }),
    };
    let line = frame::encode(&message);
    let times = {
        let mut memo = memo.lock().unwrap_or_else(|e| e.into_inner());
        match memo.calls.get_mut(&key) {
            Some(call) => {
                let times = match call.answer {
                    Answer::Owed { again } => again + 1,
                    Answer::Given(_) => 1,
                };
                call.answer = Answer::Given(line.clone());
                times
            }
            None => 1,
        }
    };
    for _ in 0..times {
        let _ = out.send(Some(line.clone()));
    }
}

fn bad_request(id: Value, why: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": BAD_REQUEST_CODE, "message": why } })
}
