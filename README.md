# Steery SDK

Steery connects your agent harness and your machines to your phone. Your agents ask, you
answer, and Steery sends the words. Steery's runtime (`steery`) uses a **driver** to reach
each harness and an **adapter** to reach each kind of machine. Drivers and adapters are small
programs. Each program speaks one protocol on its standard input and output. This repository
holds that protocol and the tools to write a program.

| | |
|---|---|
| [PROTOCOL.md](PROTOCOL.md) | The protocol: every message, shape, refusal word and bound. |
| [`steery-sdk`](src/lib.rs) | The Rust crate: the wire types, `serve` (a program's loop), the runtime's client, and the release manifest. |
| [schema/protocol-v11.json](schema/protocol-v11.json) | Every shape as JSON Schema. |
| [examples/driver](examples/driver) | The smallest driver. |
| [examples/adapter](examples/adapter) | The smallest adapter that makes hosts. |

Steery's own programs use the same method. They are the most complete examples:

- [steery-dev/drivers](https://github.com/steery-dev/drivers): omp, pi, Claude Code, Codex.
- [steery-dev/adapters](https://github.com/steery-dev/adapters): macOS, Linux, and the
  adapters of the cloud vendors, for example Daytona.

Steery uses them exactly as it uses your programs.

## The rules

- A program is one Rust executable for each target, with the name `steery-driver-<id>` or
  `steery-adapter-<id>`. The runtime does not build code. It runs what you publish.
- A program states facts and does what the runtime asks. It does not speak to Steery's
  server and does not hold a key. It changes only the items in a harness that its
  `setup/plan` lists.
- All that a person sees of your program comes from its `initialize`: `label`, `short`,
  `tint` and `mark`. No other part of Steery names it.

## Writing a driver

1. Copy [examples/driver](examples/driver) into your own repository. Name the SDK by git,
   not by path:

   ```toml
   steery-sdk = { git = "https://github.com/steery-dev/sdk", tag = "v11.1.0" }
   ```

2. In `initialize`, change the id, label and facts to those of your harness (PROTOCOL.md §3.1).
3. Answer the methods of each level that you declare (§4):
   1. `observe`: `setup/plan`, `sessions/list`, `sessions/locate`, `sessions/read`,
      `mcp/list`, and your hooks.
   2. `drive`: start and stop turns.
   3. `live`: reach a conversation that runs.
4. Test the driver after each change. Install Steery
   (`curl -fsSL https://steery.dev/install.sh | sh -s -- --install-only`), then:

   ```sh
   cargo build --release
   steery programs test target/release/steery-driver-<id>
   ```

5. Use the driver on your own computer. You do not have to publish it:

   ```sh
   steery programs add target/release/steery-driver-<id>
   ```

## Writing an adapter

Use the same steps, but start from [examples/adapter](examples/adapter). An adapter that
makes hosts answers PROTOCOL.md §5.2 (`hosts/list`, `hosts/ensure`, `start`, `stop`,
`delete`, `channel`) through the API of your vendor. An adapter for an operating system
answers §5.1. To run the work of a computer in the hosts of your adapter, use:

```sh
steery programs add <your release>
steery setup --adapter <id> --setting YOUR_API_KEY
```

## Publishing

A release is one manifest, `steery-programs.json`, with one `.tar.gz` for each program and
target (PROTOCOL.md §7.1). Build each target. Then let `steery` write the archives and the
manifest:

```sh
cargo zigbuild --release --target x86_64-unknown-linux-gnu.2.30   # runs in any made host
cargo build --release --target aarch64-apple-darwin
steery programs pack --release v0.1.0 --out dist \
  x86_64-unknown-linux-gnu=target/x86_64-unknown-linux-gnu/release \
  aarch64-apple-darwin=target/aarch64-apple-darwin/release
gh release create v0.1.0 dist/*
```

Then any person can add each program of your latest release with this command. Steery also
adds the new programs of each later release:

```sh
steery programs add https://github.com/<you>/<repo>
```

The hosts that an adapter makes run a Linux build for `x86_64-unknown-linux-gnu`. Publish one.
Then your driver works in the hosts of Daytona and of every other cloud vendor.

## Releasing the SDK

Raise `version` in `Cargo.toml` in a pull request. Once it merges, run this from a clean
checkout of that commit of `main`:

```sh
./release.sh
```

It tags the commit `v<version>` and pushes the tag. It records the release on Steery's
`sdk-releases` train: `build` as it starts, then `shipped`, or `failed` with the step that
stopped it. Steery's `release-of-commit` reads that train to say which release first carries a
commit. It needs `steery` signed in to the `steery-dev` workspace.

## Licence

MIT.
