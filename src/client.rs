//! The runtime's side: start a program, speak to it, and end it (PROTOCOL.md §2).

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::{mpsc, oneshot};

use crate::{INVALID_CODE, REFUSED_CODE, UNKNOWN_CODE, frame, method, wire};

/// How long a program has after `shutdown` and its closed standard input before SIGTERM,
/// and after SIGTERM before it is killed.
const GRACE: Duration = Duration::from_secs(5);

/// A notification a program sent.
#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub method: String,
    pub params: Value,
}

/// Why a call has no result.
#[derive(Debug, thiserror::Error)]
pub enum CallError {
    /// The program refused: `word` as it gave it, `line` its one line for a person.
    #[error("refused: {word}")]
    Refused { word: String, line: Option<String> },
    /// The program has no such method.
    #[error("unknown method")]
    Unknown,
    /// No answer within the method's bound.
    #[error("no answer in time")]
    Timeout,
    /// The program's standard input or output closed.
    #[error("the program closed")]
    Closed,
    /// The program read the parameters as invalid, or answered something that does not
    /// read as the method's result.
    #[error("invalid: {0}")]
    Invalid(String),
    /// Any other JSON-RPC error.
    #[error("error {code}: {message}")]
    Remote { code: i64, message: String },
}

/// One caller waiting for an id's answer.
struct Waiter {
    serial: u64,
    answer: oneshot::Sender<Result<Value, CallError>>,
}

/// Every id still owed, by id; `None` once the program's output has ended.
type Pending = Arc<Mutex<Option<HashMap<String, Vec<Waiter>>>>>;

/// A running program.
pub struct Client {
    /// Locked only to kill it, or to wait for it once it is shut down.
    child: Mutex<Child>,
    stdin: tokio::sync::Mutex<Option<ChildStdin>>,
    pending: Pending,
    notifications: Mutex<Option<mpsc::UnboundedReceiver<Notification>>>,
    next: AtomicU64,
    /// The protocol version the program picked in `initialize`.
    protocol: OnceLock<u32>,
    reader: tokio::task::JoinHandle<()>,
}

impl Client {
    /// Start `command` with `args`. Its standard error is appended to `stderr_log`, or
    /// is this process's own where there is none.
    pub async fn spawn(
        command: &Path,
        args: &[String],
        stderr_log: Option<&Path>,
    ) -> std::io::Result<Self> {
        let stderr = match stderr_log {
            Some(path) => Stdio::from(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)?,
            ),
            None => Stdio::inherit(),
        };
        let mut child = tokio::process::Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .kill_on_drop(true)
            .spawn()?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(std::io::Error::other("the program's stdio is not piped"));
        };
        let pending: Pending = Arc::new(Mutex::new(Some(HashMap::new())));
        let (notify, notifications) = mpsc::unbounded_channel();
        let reader = tokio::spawn(read(stdout, pending.clone(), notify));
        Ok(Self {
            child: Mutex::new(child),
            stdin: tokio::sync::Mutex::new(Some(stdin)),
            pending,
            notifications: Mutex::new(Some(notifications)),
            next: AtomicU64::new(1),
            protocol: OnceLock::new(),
            reader,
        })
    }

    /// Send `initialize`, and read what the program is. A program that names a protocol
    /// version `params` did not offer, or breaks a rule [`wire::Initialized::check`]
    /// reads, is [`CallError::Invalid`]. The version it names is then
    /// [`Client::protocol`].
    pub async fn initialize(
        &self,
        params: &wire::InitializeParams,
    ) -> Result<wire::Initialized, CallError> {
        let initialized = self.ask::<method::Initialize>(params).await?;
        if !params.protocol.contains(&initialized.protocol) {
            return Err(CallError::Invalid(format!(
                "protocol {} was not offered",
                initialized.protocol
            )));
        }
        initialized.check().map_err(CallError::Invalid)?;
        let _ = self.protocol.set(initialized.protocol);
        Ok(initialized)
    }

    /// The protocol version the program picked, once it answered `initialize`.
    pub fn protocol(&self) -> Option<u32> {
        self.protocol.get().copied()
    }

    /// Ask the request `M` with `params`, waiting at most its bound (PROTOCOL.md §9), and
    /// read its answer as its result. A request newer than the version the program picked
    /// is [`CallError::Unknown`], and is never sent.
    pub async fn ask<M: method::Method>(&self, params: &M::Params) -> Result<M::Result, CallError> {
        if self.protocol().is_some_and(|picked| picked < M::SINCE) {
            return Err(CallError::Unknown);
        }
        let params = serde_json::to_value(params).map_err(|e| CallError::Invalid(e.to_string()))?;
        let answer = self.call(M::NAME, params, M::BOUND).await?;
        serde_json::from_value(answer).map_err(|e| CallError::Invalid(e.to_string()))
    }

    /// Call `method` under a new id, waiting at most `within`.
    pub async fn call(
        &self,
        method: &str,
        params: Value,
        within: Duration,
    ) -> Result<Value, CallError> {
        let id = self.next.fetch_add(1, Ordering::Relaxed).to_string();
        self.call_as(&id, method, params, within).await
    }

    /// Call `method` under `id`, waiting at most `within` for the request to be written and
    /// answered together. The same id sent again with the same method and parameters is
    /// answered from its first result. A request whose writing was cut short, by `within`
    /// or by its caller, kills the program: half a line is on its input, and one that stops
    /// reading would hold every later call behind it. Every call in flight then reads
    /// [`CallError::Closed`], and the next start is a fresh process.
    pub async fn call_as(
        &self,
        id: &str,
        method: &str,
        params: Value,
        within: Duration,
    ) -> Result<Value, CallError> {
        let deadline = tokio::time::Instant::now() + within;
        let serial = self.next.fetch_add(1, Ordering::Relaxed);
        let (answer, answered) = oneshot::channel();
        {
            let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            let Some(pending) = pending.as_mut() else {
                return Err(CallError::Closed);
            };
            pending
                .entry(id.to_owned())
                .or_default()
                .push(Waiter { serial, answer });
        }
        let line = frame::encode(
            &json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }),
        );
        let written = match tokio::time::timeout_at(deadline, self.write(&line)).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(CallError::Closed),
            Err(_) => Err(CallError::Timeout),
        };
        if let Err(error) = written {
            self.forget(id, serial);
            return Err(error);
        }
        match tokio::time::timeout_at(deadline, answered).await {
            Ok(Ok(answer)) => answer,
            Ok(Err(_)) => Err(CallError::Closed),
            Err(_) => {
                // A late answer is dropped.
                self.forget(id, serial);
                Err(CallError::Timeout)
            }
        }
    }

    /// The program's notifications, in the order it sent them. `Some` the first time only.
    pub fn notifications(&self) -> Option<mpsc::UnboundedReceiver<Notification>> {
        self.notifications
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    /// Send `shutdown`, wait for its answer, close standard input, and send SIGTERM to a
    /// program still running 5 s later; one still running 5 s after that is killed.
    pub async fn shutdown(mut self) -> std::io::Result<std::process::ExitStatus> {
        let _ = self.ask::<method::Shutdown>(&wire::Empty {}).await;
        drop(self.stdin.lock().await.take());
        let child = self.child.get_mut().unwrap_or_else(|e| e.into_inner());
        if let Ok(status) = tokio::time::timeout(GRACE, child.wait()).await {
            self.reader.abort();
            return status;
        }
        #[cfg(unix)]
        if let Some(pid) = child.id().and_then(|pid| i32::try_from(pid).ok()) {
            // SAFETY: a signal to the child this client started and has not reaped.
            unsafe { libc::kill(pid, libc::SIGTERM) };
            if let Ok(status) = tokio::time::timeout(GRACE, child.wait()).await {
                self.reader.abort();
                return status;
            }
        }
        child.start_kill()?;
        let status = child.wait().await;
        self.reader.abort();
        status
    }

    /// Write one whole line to the program's input. Waiting for the input is harmless to
    /// cut short; the line once begun is written whole or the program is killed ([`Torn`]).
    async fn write(&self, line: &[u8]) -> std::io::Result<()> {
        let mut stdin = self.stdin.lock().await;
        let Some(stdin) = stdin.as_mut() else {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        };
        let torn = Torn(self);
        stdin.write_all(line).await?;
        stdin.flush().await?;
        std::mem::forget(torn);
        Ok(())
    }

    /// End the program at once: its output closes, so every call in flight reads closed.
    fn kill(&self) {
        let _ = self
            .child
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .start_kill();
    }

    fn forget(&self, id: &str, serial: u64) {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(pending) = pending.as_mut()
            && let Some(waiters) = pending.get_mut(id)
        {
            waiters.retain(|waiter| waiter.serial != serial);
            if waiters.is_empty() {
                pending.remove(id);
            }
        }
    }
}

/// A line being written: dropped before it is whole — its call's bound passed, its caller
/// went, or the write failed — it kills the program, whose input now holds a torn frame.
struct Torn<'a>(&'a Client);

impl Drop for Torn<'_> {
    fn drop(&mut self) {
        self.0.kill();
    }
}

/// Read the program's output until it ends: answers to their waiters, notifications to
/// their channel. A line that is no JSON, or over 16 MiB, is skipped.
async fn read(stdout: ChildStdout, pending: Pending, notify: mpsc::UnboundedSender<Notification>) {
    let mut stdout = BufReader::new(stdout);
    let mut line = Vec::new();
    while let Ok(Some(read)) = frame::next(&mut stdout, &mut line).await {
        if !matches!(read, frame::Read::Line) {
            continue;
        }
        let Ok(message) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };
        let id = message.get("id").filter(|id| !id.is_null());
        if let Some(method) = message.get("method").and_then(Value::as_str) {
            // A program sends no requests in this version.
            if id.is_none() {
                let _ = notify.send(Notification {
                    method: method.to_owned(),
                    params: message.get("params").cloned().unwrap_or(Value::Null),
                });
            }
            continue;
        }
        let Some(id) = id else {
            continue;
        };
        let key = match id {
            Value::String(id) => id.clone(),
            other => other.to_string(),
        };
        let waiters = {
            let mut pending = pending.lock().unwrap_or_else(|e| e.into_inner());
            pending.as_mut().and_then(|pending| pending.remove(&key))
        };
        for waiter in waiters.into_iter().flatten() {
            let _ = waiter.answer.send(answer(&message));
        }
    }
    // Every waiter's sender is dropped with the map: each reads as closed.
    pending.lock().unwrap_or_else(|e| e.into_inner()).take();
}

/// What one answer says.
fn answer(message: &Value) -> Result<Value, CallError> {
    let Some(error) = message.get("error") else {
        return Ok(message.get("result").cloned().unwrap_or(Value::Null));
    };
    let code = error.get("code").and_then(Value::as_i64).unwrap_or(0);
    let text = |value: Option<&Value>| value.and_then(Value::as_str).map(str::to_owned);
    let message = text(error.get("message")).unwrap_or_default();
    Err(match code {
        REFUSED_CODE => {
            let data = error.get("data");
            CallError::Refused {
                word: text(data.and_then(|data| data.get("refused")))
                    .unwrap_or_else(|| crate::refusal::REFUSED.to_owned()),
                line: text(data.and_then(|data| data.get("line"))),
            }
        }
        UNKNOWN_CODE => CallError::Unknown,
        INVALID_CODE => CallError::Invalid(message),
        code => CallError::Remote { code, message },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A program that stops reading its input: a line over the pipe's buffer cannot be
    /// written whole. Each call still answers within its own bound, a call queued behind
    /// the stuck one included, and the program is killed, so the next call reads it closed
    /// and its runtime starts a fresh one.
    #[tokio::test]
    async fn a_program_that_stops_reading_answers_each_call_in_its_bound_and_is_killed() {
        let client = Client::spawn(
            Path::new("sh"),
            &["-c".to_owned(), "exec sleep 30".to_owned()],
            None,
        )
        .await
        .unwrap();
        let big = json!({ "bytes": "x".repeat(1 << 20) });
        let began = std::time::Instant::now();
        let (stuck, queued) = tokio::join!(
            client.call("host/fs/write", big, Duration::from_millis(600)),
            async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                client
                    .call("process/list", json!({}), Duration::from_millis(300))
                    .await
            },
        );
        assert!(matches!(stuck, Err(CallError::Timeout)), "{stuck:?}");
        assert!(matches!(queued, Err(CallError::Timeout)), "{queued:?}");
        assert!(
            began.elapsed() < Duration::from_secs(2),
            "{:?}",
            began.elapsed()
        );

        let next = client
            .call("process/list", json!({}), Duration::from_secs(5))
            .await;
        assert!(matches!(next, Err(CallError::Closed)), "{next:?}");
        // The kill is a signal: the program's input closes before the system reports its
        // exit, so the exit is waited for, not read once.
        let mut exited = None;
        for _ in 0..200 {
            exited = client
                .child
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .try_wait()
                .unwrap();
            if exited.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            exited.is_some(),
            "the program that held a torn line is killed"
        );
    }

    /// The test that is the program the next test runs: this test binary, run again with
    /// [`PROGRAM`] set, serves [`Lister`] on its standard input and output.
    const PROGRAM: &str = "STEERY_SDK_TEST_PROGRAM";

    /// The URLs [`Lister`]'s harness names its servers by, as its configuration spells
    /// them; each `hunter2` is a secret.
    const URLS: &[(&str, Option<&str>)] = &[
        (
            "tracker",
            Some("https://bot:hunter2@mcp.example.com:8443/v1/mcp?token=hunter2#hunter2"),
        ),
        ("socket", Some("wss://hunter2@mcp.example.com/ws")),
        ("slash", Some("https://bot:hunter2/x@mcp.example.com/mcp")),
        (
            "question",
            Some("https://bot:hunter2?x@mcp.example.com/mcp"),
        ),
        (
            "digits",
            Some("https://bot:2222/hunter2@mcp.example.com/mcp"),
        ),
        ("loopback", Some("http://[::1]:7000/mcp")),
        ("files", None),
    ];

    /// A driver whose harness loads the servers [`URLS`] names, and in a folder one more.
    struct Lister;

    impl crate::Program for Lister {
        fn initialize(
            &self,
            params: &wire::InitializeParams,
        ) -> Result<wire::Initialized, crate::Error> {
            let protocol = crate::pick(&params.protocol)
                .ok_or_else(|| crate::Error::invalid("no protocol this driver speaks"))?;
            Ok(wire::Initialized {
                protocol,
                program: wire::ProgramInfo {
                    id: "lister".parse().map_err(crate::Error::invalid)?,
                    kind: wire::Kind::Harness,
                    release: "1.0.0".into(),
                    label: "Lister".into(),
                    short: "L".into(),
                    tint: None,
                    mark: None,
                },
                harness: Some(Box::new(wire::HarnessInfo {
                    levels: vec![wire::Level::Observe],
                    minimum: None,
                    sign_in: None,
                    knowing: "It has no conversations.".into(),
                    transcript: None,
                    identity: wire::Identity {
                        executables: vec!["lister".into()],
                        package: None,
                        session_variable: None,
                    },
                    setup: wire::SetupFacts {
                        reads_skills: false,
                        cuts_tool_text: false,
                        call_bound: None,
                    },
                    reader: None,
                    facts: wire::HarnessFacts {
                        by_file: false,
                        resumes_in_place: false,
                        forks_beside: false,
                        keeps: wire::Keeps {
                            model: false,
                            thinking: false,
                        },
                        door: None,
                        checks_idle: false,
                        worktrees: false,
                        acts: Vec::new(),
                    },
                    hooks: Vec::new(),
                })),
                machine: None,
                settings: Vec::new(),
            })
        }

        async fn call(&self, method: &str, params: Value) -> Result<Value, crate::Error> {
            if method != method::MCP_LIST {
                return Err(crate::Error::Unknown);
            }
            let asked: wire::harness::McpList = crate::params(params)?;
            let mut servers: Vec<_> = URLS
                .iter()
                .map(|(server, url)| wire::harness::McpServer::new(*server, *url))
                .collect();
            if let Some(cwd) = asked.cwd {
                servers.push(wire::harness::McpServer::new(
                    cwd.display().to_string(),
                    None,
                ));
            }
            crate::result(wire::harness::McpServers {
                servers,
                complete: true,
                watch: vec!["/home/a/.lister/mcp.json".into()],
            })
        }
    }

    /// The runtime offers 3 and 4: a program on this crate picks 4 and answers `mcp/list`
    /// with each server's name and its URL bare, or no URL where its host cannot be told
    /// from a password. Offered 3 alone it picks 3, and the client never sends `mcp/list`
    /// to it; offered neither, it is refused.
    #[tokio::test]
    async fn a_program_offered_4_lists_its_mcp_servers_and_one_offered_3_is_never_asked() {
        if std::env::var_os(PROGRAM).is_some() {
            let served = crate::serve(|_notifier| Lister).await;
            std::process::exit(i32::from(served.is_err()));
        }
        let start = || async {
            let program = std::env::current_exe().unwrap();
            let this = "client::tests::a_program_offered_4_lists_its_mcp_servers_and_one_offered_3_is_never_asked";
            Client::spawn(
                Path::new("env"),
                &[
                    format!("{PROGRAM}=1"),
                    program.display().to_string(),
                    "--exact".into(),
                    this.into(),
                    "--nocapture".into(),
                ],
                None,
            )
            .await
            .unwrap()
        };
        let offer = |protocol: Vec<u32>| wire::InitializeParams {
            protocol,
            runtime: wire::Runtime {
                release: "7.0.0".into(),
                os: "linux".into(),
                arch: "x86_64".into(),
            },
            home: "/home/a".into(),
            config: "/home/a/.config/steery".into(),
            settings: Default::default(),
        };

        let new = start().await;
        assert_eq!(new.protocol(), None);
        let initialized = new.initialize(&offer(vec![3, 4])).await.unwrap();
        assert_eq!(initialized.protocol, 4);
        assert_eq!(new.protocol(), Some(4));
        let raw = new
            .call(method::MCP_LIST, json!({}), Duration::from_secs(30))
            .await
            .unwrap();
        assert_eq!(
            raw,
            json!({
                "servers": [
                    {"server": "tracker", "url": "https://mcp.example.com:8443/v1/mcp"},
                    {"server": "socket", "url": "wss://mcp.example.com/ws"},
                    {"server": "slash"},
                    {"server": "question"},
                    {"server": "digits"},
                    {"server": "loopback", "url": "http://[::1]:7000/mcp"},
                    {"server": "files"},
                ],
                "complete": true,
                "watch": ["/home/a/.lister/mcp.json"],
            })
        );
        assert!(!raw.to_string().contains("hunter2"), "{raw}");
        let unknown = new
            .call(
                method::MCP_LIST,
                json!({"folder": "/home/a"}),
                Duration::from_secs(30),
            )
            .await;
        assert!(matches!(unknown, Err(CallError::Invalid(_))), "{unknown:?}");
        let in_folder = new
            .ask::<method::McpList>(&wire::harness::McpList {
                cwd: Some("/home/a/work".into()),
            })
            .await
            .unwrap();
        assert_eq!(
            in_folder.servers.last(),
            Some(&wire::harness::McpServer {
                server: "/home/a/work".into(),
                url: None,
            })
        );
        new.shutdown().await.unwrap();

        let old = start().await;
        let initialized = old.initialize(&offer(vec![3])).await.unwrap();
        assert_eq!(initialized.protocol, 3);
        let asked = old
            .ask::<method::McpList>(&wire::harness::McpList { cwd: None })
            .await;
        assert!(matches!(asked, Err(CallError::Unknown)), "{asked:?}");
        old.shutdown().await.unwrap();

        let none = start().await;
        let refused = none.initialize(&offer(vec![2])).await;
        assert!(matches!(refused, Err(CallError::Invalid(_))), "{refused:?}");
        none.shutdown().await.unwrap();
    }
}
