# RetireTui

[![Crate Badge]][Crate] [![Docs Badge]][Docs] [![CI Badge]][CI]
[![Deps Badge]][Deps] [![License Badge]][License]

A local-first retirement planner for the terminal and the browser.

You describe a household in one plain TOML file - people, accounts and what they
hold, contributions, income, expenses, and the milestones that start and stop
them - and RetireTui projects it year by year under U.S. federal and state
income tax. The same plan runs through many random or historical markets to show
how surely the money lasts, and searches for a better Roth conversion ladder,
Social Security claim ages or withdrawal order. A scenario is a small file
stating only what differs from a base plan, so alternatives compare side by
side.

Everything runs locally; no plan leaves your machine. RetireTui is a model under
the assumptions you give it, not financial advice.

![The interactive planner opening an example plan: the overview, the year-by-year
ledger, a Monte Carlo run, and an account opened for editing][Demo]

## Requirements

- **Rust 1.95** or newer, to build.
- **A terminal.** Truecolor and the kitty keyboard protocol are used where the
  terminal offers them.

## Install

```sh
cargo install retiretui
```

Or download a prebuilt binary for x86_64 or aarch64 Linux from the [releases
page][Releases]:

```sh
curl -LO https://github.com/kenianbei/retiretui/releases/latest/download/retiretui-x86_64-unknown-linux-gnu.tar.gz
tar xzf retiretui-x86_64-unknown-linux-gnu.tar.gz
```

The binary is inside the `retiretui-x86_64-unknown-linux-gnu/` directory it
unpacks to.

## Usage

### Interactive planner

```sh
retiretui tui examples/
```

Opens a plan, a scenario, or a directory to pick one from. With no plan, a short
form builds a first one from the household's basics. The [example
plans][Examples] are invented households, from a first job to a retired couple,
to try it on from a clone of the repository.

### In a browser

The web app at <https://kenianbei.github.io/retiretui/>, published with each
release, is the planner for a phone or a desktop browser: the overview, the
year-by-year ledger, plans compared side by side, the searches, the market runs
and the tax tables, and every part of a plan edited in a form. It is tested in
Chromium, Firefox and WebKit, at desktop and phone sizes. After one visit every
page works offline, and the browser can install it as an app. Plans are kept in
the browser's own storage and never leave it unless downloaded; clearing the
browser's site data deletes them. To build and serve it yourself, with
[wasm-pack](https://wasm-bindgen.github.io/wasm-pack/) and Node 22 installed:

```sh
cd web
npm ci
npm run dev
```

The whole terminal planner also runs in a browser canvas, as an experiment, at
<https://kenianbei.github.io/retiretui/ratzilla/>: `upload` and `download`, from
the `:` command palette, bring a plan or a Social Security statement in and take
a plan back out. To build and serve it:

```sh
crates/retiretui_web/build.sh
python3 -m http.server -d crates/retiretui_web/dist
```

### Command line

| command           | what it does                                        |
| ----------------- | --------------------------------------------------- |
| `validate`        | Check a plan for schema and consistency errors      |
| `project`         | Print the year-by-year ledger, as a table or JSON   |
| `actions`         | List one year's to-dos: conversions, RMDs, and more |
| `compare`         | Compare plans or scenarios side by side             |
| `optimize`        | Search conversions, claim ages, or withdrawal order |
| `monte-carlo`     | Run the plan through many random markets            |
| `historical`      | Run the plan from every historical start year       |
| `import-earnings` | Record an ssa.gov earnings statement on a person    |
| `tui`             | Open the interactive planner                        |
| `mcp`             | Serve plans to AI agents over stdio                 |

`retiretui <command> --help` lists each command's options. From a clone of the
repository:

```sh
retiretui project examples/starter.toml
retiretui compare examples/early-retiree.toml examples/early-retiree-no-ladder.toml
retiretui monte-carlo examples/market-mix.toml
```

### AI agents

`retiretui mcp` serves the same tools over the Model Context Protocol, sandboxed
to one directory of plans, with a schema reference for writing them. Register it
with an MCP client:

```json
{
  "mcpServers": {
    "retiretui": {
      "command": "retiretui",
      "args": ["mcp", "--dir", "/path/to/plans"]
    }
  }
}
```

## Status

Pre-1.0: the plan schema and the commands still change between minor versions,
and [CHANGELOG.md][Changelog] records what did. Tax law is embedded for 2026;
later years are extended by inflation, and `--tax-dir` adds parameter files for
others. [ARCHITECTURE.md][Architecture] describes how the system fits together.

## Contributing

Changes arrive as pull requests. The checks CI runs are in [`ci.yml`][Checks];
run them locally before opening one.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE][Apache])
- MIT license ([LICENSE-MIT][MIT])

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[Crate]: https://crates.io/crates/retiretui
[Crate Badge]:
  https://img.shields.io/crates/v/retiretui?logo=rust&style=flat-square&color=E05D44
[Docs]: https://docs.rs/retiretui_engine
[Docs Badge]:
  https://img.shields.io/docsrs/retiretui_engine?logo=rust&style=flat-square&label=engine%20docs
[CI]: https://github.com/kenianbei/retiretui/actions/workflows/ci.yml
[CI Badge]:
  https://img.shields.io/github/actions/workflow/status/kenianbei/retiretui/ci.yml?style=flat-square&logo=github
[Deps]: https://deps.rs/repo/github/kenianbei/retiretui
[Deps Badge]:
  https://deps.rs/repo/github/kenianbei/retiretui/status.svg?style=flat-square
[License]: #license
[License Badge]:
  https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue?style=flat-square
[Demo]:
  https://raw.githubusercontent.com/kenianbei/retiretui/HEAD/assets/demo.gif
[Releases]: https://github.com/kenianbei/retiretui/releases
[Examples]: https://github.com/kenianbei/retiretui/blob/HEAD/examples/README.md
[Changelog]: https://github.com/kenianbei/retiretui/blob/HEAD/CHANGELOG.md
[Architecture]: https://github.com/kenianbei/retiretui/blob/HEAD/ARCHITECTURE.md
[Checks]:
  https://github.com/kenianbei/retiretui/blob/HEAD/.github/workflows/ci.yml
[Apache]: https://github.com/kenianbei/retiretui/blob/HEAD/LICENSE-APACHE
[MIT]: https://github.com/kenianbei/retiretui/blob/HEAD/LICENSE-MIT
