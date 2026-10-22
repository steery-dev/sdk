# Steery SDK

Steery connects your agent harness and your machines to your phone: your agents ask, you
answer, and Steery carries the words. Steery's runtime (`steery`) reaches each harness
through a **driver** and each kind of machine through an **adapter**: small programs that
speak one protocol on their standard input and output. This repository is that protocol
and the means to write one.

| | |
|---|---|
| [PROTOCOL.md](PROTOCOL.md) | The protocol: every message, shape, refusal word and bound. |
| [`steery-sdk`](src/lib.rs) | The Rust crate: the wire types, `serve` (a program's loop), the runtime's client, and the release manifest. |
| [schema/protocol-v4.json](schema/protocol-v4.json) | Every shape as JSON Schema. |
| [examples/driver](examples/driver) | The smallest driver. |
| [examples/adapter](examples/adapter) | The smallest adapter that makes hosts. |

Steery's own programs are written the same way, and are the fullest examples:
[steery-dev/drivers](https://github.com/steery-dev/drivers) (omp, pi, Claude Code,
Codex) and [steery-dev/adapters](https://github.com/steery-dev/adapters) (macOS, Linux,
and the cloud vendors' adapters, Daytona's among them). Steery treats them exactly as it
treats yours.

## The rules

- A program is one Rust executable per target, named `steery-driver-<id>` or
  `steery-adapter-<id>`. The runtime never builds code; it runs what you publish.
- A program states facts and does what it is asked. It never talks to Steery's server,
  never holds a key, and changes nothing in a harness but what its `setup/plan` lists.
- Everything a person sees of your program comes from its `initialize`: `label`, `short`,
  `tint` and `mark`. Nothing in Steery names it.

## Writing a driver

1. Copy [examples/driver](examples/driver) into a repository of your own, and name the
   SDK by git instead of by path:

   ```toml
   steery-sdk = { git = "https://github.com/steery-dev/sdk", tag = "v4.0.0" }
   ```

2. Change the id, label and facts in `initialize` to your harness's (PROTOCOL.md §3.1),
   and answer the methods of the levels you declare (§4): `observe` first — `setup/plan`,
   `sessions/list`, `sessions/locate`, `sessions/read`, `mcp/list`, and your hooks — then
   `drive` to start and stop turns, then `live` to reach a running conversation.
3. Check it as you go. Install Steery (`curl -fsSL https://steery.dev/install.sh | sh -s
   -- --install-only`), then:

   ```sh
   cargo build --release
   steery programs test target/release/steery-driver-<id>
   ```

4. Use it on your own computer without publishing anything:

   ```sh
   steery programs add target/release/steery-driver-<id>
   ```

## Writing an adapter

The same, from [examples/adapter](examples/adapter): an adapter that makes hosts answers
PROTOCOL.md §5.2 (`hosts/list`, `hosts/ensure`, `start`, `stop`, `delete`, `channel`) over
your vendor's own API; an operating system answers §5.1. A computer whose work runs in
your adapter's hosts is then:

```sh
steery programs add <your release>
steery setup --adapter <id> --setting YOUR_API_KEY
```

## Publishing

A release is one manifest, `steery-programs.json`, beside one `.tar.gz` per program and
target (PROTOCOL.md §7.1). Build each target, then let `steery` write the archives and the
manifest:

```sh
cargo zigbuild --release --target x86_64-unknown-linux-gnu.2.30   # runs in any made host
cargo build --release --target aarch64-apple-darwin
steery programs pack --release v0.1.0 --out dist \
  x86_64-unknown-linux-gnu=target/x86_64-unknown-linux-gnu/release \
  aarch64-apple-darwin=target/aarch64-apple-darwin/release
gh release create v0.1.0 dist/*
```

Anyone then adds every program of your latest release, and follows its new ones, with:

```sh
steery programs add https://github.com/<you>/<repo>
```

A Linux build for `x86_64-unknown-linux-gnu` is what the hosts an adapter makes run:
publish one, and your driver works in Daytona's hosts and every other cloud vendor's too.

## Licence

MIT.
