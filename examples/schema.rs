//! Writes one JSON Schema document holding every wire type and the release manifest under
//! `$defs` to standard output: `cargo run --features schema --example schema`. Its output is
//! `schema/protocol-v3.json`.

use schemars::SchemaGenerator;
use schemars::generate::SchemaSettings;
use serde_json::{Value, json};
use steery_sdk::release;
use steery_sdk::wire::{self, door, harness, host, hosts, process};

fn main() {
    let mut generator = SchemaGenerator::new(SchemaSettings::draft2020_12());
    macro_rules! add {
        ($($ty:ty),* $(,)?) => { $( generator.subschema_for::<$ty>(); )* };
    }
    add!(
        wire::InitializeParams,
        wire::Runtime,
        wire::Initialized,
        wire::ProgramInfo,
        wire::Kind,
        wire::Level,
        wire::HarnessInfo,
        wire::Minimum,
        wire::Identity,
        wire::SetupFacts,
        wire::ReaderFacts,
        wire::Sealed,
        wire::HarnessFacts,
        wire::Keeps,
        wire::Machine,
        wire::ProcessFacts,
        wire::Os,
        wire::ServiceManager,
        wire::HostFacts,
        wire::Survives,
        wire::Idle,
        wire::Optional,
        wire::Channel,
        wire::Setting,
        wire::Empty,
        harness::Hook,
        harness::HookKind,
        harness::Via,
        harness::Says,
        harness::Setup,
        harness::Planned,
        harness::Install,
        harness::Found,
        harness::Entry,
        harness::Format,
        harness::Part,
        harness::List,
        harness::Listed,
        harness::Session,
        harness::SubAgent,
        harness::Locate,
        harness::Located,
        harness::Read,
        harness::Events,
        harness::Fold,
        harness::Record,
        harness::Folded,
        harness::Event,
        harness::TurnState,
        harness::Execution,
        harness::Count,
        harness::Compactor,
        harness::Voice,
        harness::Clip,
        harness::Call,
        harness::Group,
        harness::Asks,
        harness::Question,
        harness::Choice,
        harness::Failure,
        harness::FailureKind,
        harness::HookRead,
        harness::HookMoment,
        harness::Moment,
        harness::Suggestion,
        harness::SessionRef,
        harness::HookAnswer,
        harness::Answer,
        harness::Decision,
        harness::Always,
        harness::HookAnswered,
        harness::Append,
        harness::McpList,
        harness::McpServers,
        harness::McpServer,
        harness::Held,
        harness::HeldBy,
        harness::Looked,
        harness::Look,
        harness::Beyond,
        harness::Holder,
        harness::HolderStop,
        harness::Door,
        harness::LiveDoor,
        harness::Act,
        harness::Sampled,
        harness::Sample,
        harness::Models,
        harness::Unlisted,
        harness::Model,
        harness::Start,
        harness::Launch,
        harness::Stdin,
        harness::TurnRead,
        harness::Stream,
        harness::TurnLine,
        harness::Ended,
        harness::Stop,
        harness::Stopped,
        harness::ReceiptMark,
        harness::Marked,
        harness::ReceiptFind,
        harness::Receipt,
        harness::Reader,
        harness::ReaderTools,
        harness::ReaderPlan,
        harness::ReaderFile,
        harness::ReadAs,
        harness::ReaderRead,
        harness::Text,
        harness::Terminal,
        harness::TerminalCommand,
        harness::Say,
        harness::SayMode,
        harness::Said,
        harness::Pick,
        harness::Picked,
        harness::Recap,
        harness::SessionChanged,
        harness::LiveState,
        harness::LiveTurn,
        harness::LiveCall,
        harness::LiveChild,
        harness::LiveEnded,
        harness::Dialog,
        harness::DialogKind,
        harness::LiveSettled,
        door::PluginFrame,
        door::DoorMode,
        door::RuntimeFrame,
        door::DoorEntries,
        door::Spoken,
        door::Ask,
        door::DoorSayMode,
        door::DoorStopped,
        door::DoorCompacted,
        process::Start,
        process::Ended,
        process::Started,
        process::List,
        process::Listed,
        process::Entry,
        process::Environ,
        process::Values,
        process::Files,
        process::Paths,
        process::Listening,
        process::Listeners,
        process::Listener,
        process::Signal,
        process::SignalName,
        process::Sent,
        process::ServicePlan,
        process::Plan,
        process::PlanCommand,
        process::PlannedFile,
        process::Service,
        process::Status,
        hosts::List,
        hosts::Listed,
        hosts::Host,
        hosts::Ensure,
        hosts::Source,
        hosts::Named,
        hosts::State,
        hosts::Expose,
        hosts::Exposed,
        hosts::Put,
        hosts::Channel,
        hosts::Reach,
        hosts::StreamWrite,
        hosts::StreamClose,
        hosts::StreamData,
        hosts::StreamClosed,
        host::Hello,
        host::Welcome,
        host::FsStat,
        host::FsStats,
        host::Stat,
        host::FileKind,
        host::FsRead,
        host::FsBytes,
        host::FsList,
        host::FsEntries,
        host::FsEntry,
        host::FsWrite,
        host::FsMkdir,
        host::FsRemove,
        host::Programs,
        host::Call,
        host::Notified,
        host::Missing,
        host::Exited,
        host::Opened,
        host::Dial,
        host::Data,
        host::Credit,
        host::Close,
        host::CloseHow,
        release::Manifest,
        release::Program,
        release::File,
    );
    let mut defs: serde_json::Map<String, Value> = generator.take_definitions(true);
    for schema in defs.values_mut() {
        tidy(schema);
    }
    // The root names every definition, so a generator that emits only what it reaches
    // emits them all.
    let refs: Vec<Value> = defs
        .keys()
        .map(|name| json!({ "$ref": format!("#/$defs/{name}") }))
        .collect();
    let document = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://raw.githubusercontent.com/steery-dev/sdk/main/schema/protocol-v3.json",
        "title": "Protocol",
        "description": "Every shape of the Steery program protocol, version 3 (PROTOCOL.md), and the release manifest programs are published by.",
        "anyOf": refs,
        "$defs": defs,
    });
    let text = serde_json::to_string_pretty(&document).expect("a schema is JSON");
    println!("{text}");
}

/// Says what the shapes mean where schemars says more: an optional field takes no `null`,
/// since it is absent, never `null` (PROTOCOL.md §2). A required field that may be `null`
/// keeps it.
fn tidy(schema: &mut Value) {
    match schema {
        Value::Object(map) => {
            let required: Vec<Value> = map
                .get("required")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if let Some(Value::Object(properties)) = map.get_mut("properties") {
                for (name, property) in properties.iter_mut() {
                    if !required.contains(&Value::String(name.clone())) {
                        absent_not_null(property);
                    }
                }
            }
            for value in map.values_mut() {
                tidy(value);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(tidy),
        _ => {}
    }
}

/// Takes `null` out of what an optional field accepts.
fn absent_not_null(property: &mut Value) {
    let Value::Object(map) = property else {
        return;
    };
    if let Some(Value::Array(types)) = map.get_mut("type") {
        types.retain(|kind| kind != "null");
        if let [only] = types.as_slice() {
            let only = only.clone();
            map.insert("type".to_owned(), only);
        }
    }
    if let Some(Value::Array(choices)) = map.get_mut("anyOf") {
        choices.retain(|choice| choice.get("type") != Some(&json!("null")));
        if choices.len() == 1 {
            let only = choices.remove(0);
            map.remove("anyOf");
            map.insert("allOf".to_owned(), json!([only]));
        }
    }
}
