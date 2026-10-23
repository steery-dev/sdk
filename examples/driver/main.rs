//! The smallest driver: it observes a harness called `example`, whose conversations live
//! as one file each in `~/.example/sessions/<id>.jsonl`. It says what the harness is,
//! where it is installed and what Steery writes into it, and lists its conversations;
//! every method it does not have is `Error::Unknown`, which the runtime reads as a method
//! the harness does not do.
//!
//! A driver for a real harness answers the rest of PROTOCOL.md §4 the same way:
//! `sessions/read` for its transcript, `hook/read` and `hook/answer` for its hooks, and
//! `start`, `turn/read` and the others of `drive` once it can run turns.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde_json::Value;
use steery_sdk::wire::{self, harness};
use steery_sdk::{Error, Program, method, params, refusal, result};

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    steery_sdk::serve(|_notifier| Example::default()).await
}

#[derive(Default)]
struct Example {
    /// The person's home folder, given in `initialize`.
    home: OnceLock<PathBuf>,
}

impl Example {
    fn sessions(&self) -> PathBuf {
        self.home
            .get()
            .map_or_else(PathBuf::new, |home| home.join(".example/sessions"))
    }
}

impl Program for Example {
    fn initialize(&self, params: &wire::InitializeParams) -> Result<wire::Initialized, Error> {
        let protocol = steery_sdk::pick(&params.protocol)
            .ok_or_else(|| Error::invalid("this driver speaks protocols 3 and 4"))?;
        let _ = self.home.set(params.home.clone());
        Ok(wire::Initialized {
            protocol,
            program: wire::ProgramInfo {
                id: "example".parse().map_err(Error::invalid)?,
                kind: wire::Kind::Harness,
                release: env!("CARGO_PKG_VERSION").into(),
                label: "Example".into(),
                short: "Ex".into(),
                tint: Some("#3b6ea8".into()),
                // A square with a dot cut out, filled even-odd.
                mark: Some("M4 4h16v16H4z M10 10h4v4h-4z".into()),
            },
            harness: Some(Box::new(wire::HarnessInfo {
                levels: vec![wire::Level::Observe],
                minimum: None,
                sign_in: None,
                knowing: "Its conversation's id is the name of its file in ~/.example/sessions."
                    .into(),
                transcript: Some("One JSON object per line.".into()),
                identity: wire::Identity {
                    executables: vec!["example".into()],
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

    async fn call(&self, method: &str, given: Value) -> Result<Value, Error> {
        match method {
            method::SETUP_PLAN => {
                let _: harness::Setup = params(given)?;
                result(self.plan())
            }
            method::SESSIONS_LIST => {
                let _: harness::List = params(given)?;
                result(harness::Listed {
                    sessions: self.list(),
                    excluded: Vec::new(),
                    complete: true,
                })
            }
            method::SESSIONS_LOCATE => {
                let asked: harness::Locate = params(given)?;
                let session = self
                    .list()
                    .into_iter()
                    .find(|session| session.external_id == asked.external_id)
                    .ok_or_else(|| Error::refused(refusal::NOT_FOUND, "no such conversation"))?;
                result(harness::Located { session })
            }
            _ => Err(Error::Unknown),
        }
    }
}

impl Example {
    /// Where the harness is, and what Steery writes into it: nothing, in this example. A
    /// real driver lists the MCP entry, the skill and the hooks as `Entry`s here.
    fn plan(&self) -> harness::Planned {
        let root = self
            .home
            .get()
            .map_or_else(PathBuf::new, |home| home.join(".example"));
        harness::Planned {
            found: which("example").map(|binary| harness::Found {
                binary,
                // A real driver asks its harness: `example --version`.
                release: "unknown".into(),
                root,
            }),
            refused: None,
            entries: Vec::new(),
            notes: Vec::new(),
            install: None,
        }
    }

    /// Every conversation: one file each.
    fn list(&self) -> Vec<harness::Session> {
        let Ok(entries) = std::fs::read_dir(self.sessions()) else {
            return Vec::new();
        };
        entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let id = path.file_stem()?.to_str()?.to_owned();
                let updated_at = entry
                    .metadata()
                    .ok()?
                    .modified()
                    .ok()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()?
                    .as_millis() as u64;
                Some(harness::Session {
                    external_id: id,
                    cwd: None,
                    title: None,
                    created_at: None,
                    updated_at,
                    transcripts: vec![path],
                    parent: None,
                    app: false,
                    sub_agents: Vec::new(),
                })
            })
            .collect()
    }
}

/// The first `name` on `PATH`.
fn which(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|dir| dir.join(name))
        .find(|candidate| Path::new(candidate).is_file())
}
