//! `sessions/list` answers that give only what changed since an earlier listing
//! (PROTOCOL.md §4.1, version [`crate::LIST_CHANGES`]). A driver makes them with
//! [`Changes`], and the runtime reads each one back into the whole listing with
//! [`Listed::then`].

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::wire::harness::{Listed, Session};

/// How many of its own listings a driver keeps, so that a call with `after` can be answered
/// with only what changed. The runtime's sweeps follow one cursor; other callers ask whole,
/// and each of their answers is a listing kept too.
const KEEPS: usize = 4;

/// The listings that a driver answered, by their cursors: what it needs to give the next
/// call only what changed. A driver keeps one for the life of its process, says the
/// protocol that it picked ([`Changes::picked`]), and gives it each whole listing
/// ([`Changes::answer`]).
pub struct Changes {
    /// This process's own part of each cursor, so that a driver that started again never
    /// reads a cursor of the process before it.
    run: String,
    /// The protocol that the driver picked; 0 until it says.
    protocol: AtomicU32,
    kept: Mutex<Kept>,
}

/// The number of the next listing, and the newest listings, oldest first.
#[derive(Default)]
struct Kept {
    next: u64,
    listings: VecDeque<(String, Seen)>,
}

/// What a listing had: each conversation's fingerprint by its id, and the excluded ids.
struct Seen {
    sessions: HashMap<String, u64>,
    excluded: HashSet<String>,
}

impl Default for Changes {
    fn default() -> Self {
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        Self {
            run: format!("{:x}-{started:x}", std::process::id()),
            protocol: AtomicU32::new(0),
            kept: Mutex::default(),
        }
    }
}

impl Changes {
    /// The protocol that the driver picked in `initialize`.
    pub fn picked(&self, protocol: u32) {
        self.protocol.store(protocol, Ordering::SeqCst);
    }

    /// The answer to a `sessions/list` with `after`, from `whole`, everything that the driver
    /// lists now for that call. From version [`crate::LIST_CHANGES`] on, the answer names
    /// itself by a cursor. Where `after` names a listing that this keeps, the answer gives
    /// only what changed since it; otherwise it gives everything. Before that version, or
    /// before the driver said what it picked, it is `whole`, as it is.
    pub fn answer(&self, after: Option<&str>, whole: Listed) -> Listed {
        if self.protocol.load(Ordering::SeqCst) < crate::LIST_CHANGES {
            return whole;
        }
        let now = Seen {
            sessions: whole
                .sessions
                .iter()
                .map(|session| (session.external_id.clone(), fingerprint(session)))
                .collect(),
            excluded: whole.excluded.iter().cloned().collect(),
        };
        let mut kept = self.kept.lock().unwrap_or_else(|e| e.into_inner());
        let cursor = format!("{}-{}", self.run, kept.next);
        kept.next += 1;
        let was = after.and_then(|after| kept.listings.iter().find(|(cursor, _)| cursor == after));
        let answer = match was {
            Some((after, was)) => {
                let mut gone: Vec<String> = was
                    .sessions
                    .keys()
                    .chain(&was.excluded)
                    .filter(|id| !now.sessions.contains_key(*id) && !now.excluded.contains(*id))
                    .cloned()
                    .collect();
                gone.sort_unstable();
                gone.dedup();
                Listed {
                    sessions: whole
                        .sessions
                        .into_iter()
                        .filter(|session| {
                            was.sessions.get(&session.external_id)
                                != now.sessions.get(&session.external_id)
                        })
                        .collect(),
                    excluded: whole
                        .excluded
                        .into_iter()
                        .filter(|id| !was.excluded.contains(id))
                        .collect(),
                    complete: whole.complete,
                    cursor: Some(cursor.clone()),
                    after: Some(after.clone()),
                    gone,
                }
            }
            None => Listed {
                cursor: Some(cursor.clone()),
                after: None,
                gone: Vec::new(),
                ..whole
            },
        };
        kept.listings.push_back((cursor, now));
        while kept.listings.len() > KEEPS {
            kept.listings.pop_front();
        }
        answer
    }
}

/// What tells two listings of one conversation apart.
fn fingerprint(session: &Session) -> u64 {
    let mut hasher = DefaultHasher::new();
    session.hash(&mut hasher);
    hasher.finish()
}

impl Listed {
    /// The whole listing that `next` says, where `self` is the whole listing whose cursor
    /// the call gave as `after`: `next` as it is where it gives everything, else `self` with
    /// what changed. `None` where `next` gives what changed since another listing.
    pub fn then(self, next: Listed) -> Option<Listed> {
        let Some(after) = &next.after else {
            return Some(next);
        };
        if self.cursor.as_ref() != Some(after) {
            return None;
        }
        let changed: HashSet<&str> = next
            .sessions
            .iter()
            .map(|session| session.external_id.as_str())
            .chain(next.excluded.iter().map(String::as_str))
            .chain(next.gone.iter().map(String::as_str))
            .collect();
        let mut sessions = self.sessions;
        sessions.retain(|session| !changed.contains(session.external_id.as_str()));
        let mut excluded = self.excluded;
        excluded.retain(|id| !changed.contains(id.as_str()));
        sessions.extend(next.sessions);
        excluded.extend(next.excluded);
        Some(Listed {
            sessions,
            excluded,
            complete: next.complete,
            cursor: next.cursor,
            after: None,
            gone: Vec::new(),
        })
    }
}
