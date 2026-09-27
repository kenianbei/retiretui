# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- A web app for phones and desktop browsers, published to GitHub Pages at the
  site's root with each release. It keeps its plans in the browser's own
  storage, apart from the canvas page's, and never sends them anywhere. It
  offers the example plans or an upload to start from, reopens the plan last
  open, and downloads the open plan. Its Overview shows whether the money
  lasts - its headline figures in today's or nominal dollars, and the share of a
  thousand random markets it survives, run off the page's thread - and what to
  do this year, said as the terminal says it. A plan with issues lists each by
  the page, item and field it is about. The other pages are named but not yet
  built. Light or dark follows the system, and navigation is a bottom bar on a
  phone and a sidebar on a wider screen.

### Changed

- The canvas page, the terminal planner drawn in a browser, moves from the Pages
  site's root to `/ratzilla/`.

- What every interface shares over the engine is a crate of its own,
  `retiretui_client`: the words a plan is said in, the form model with its
  editing domains named by ids of their own, issues read back as the domain,
  item and field they are about, the load-and-validate gate and the store every
  plan file goes through, the open document and the draft with its history, the
  new-plan answers, the statement import, the searches the tools share, and the
  shapes the CLI's JSON and the MCP tools reply in. `retiretui_tui` builds on
  it, and its `actions`, `files`, `ladder`, `metric`, `resolve`, `store` and
  `table` modules are gone from its API. The engine resolves scenario base
  chains itself, as `plan::resolve`.
- The command line and the MCP server are library crates of their own,
  `retiretui_cli` and `retiretui_mcp`, and the terminal launcher is
  `retiretui_tui`'s `terminal` feature, off by default; `retiretui` composes the
  three into the one command it always was, with the same commands, output and
  help. The client loads the tax tables, the market history and the user's
  directories behind a `native` feature, and the MCP server reaches its
  sandboxed plan files through the client's store.

## [0.2.0] - 2026-09-27

### Added

- A form too tall for the terminal scrolls: its fields scroll above a help line
  and buttons that stay put, the field holding the keyboard is kept in view, the
  mouse wheel scrolls it, and a scrollbar on its right edge shows where it is.
- An account's glide path is edited as up to six steps, each step offered once
  the one before it is filled; a file's steps past the sixth are kept as they
  are.
- A mix's cash, and each glide step's, is shown beside its stocks and bonds as
  what they leave, following them as they are edited and marked when they leave
  less than none.
- The Market form edits the stocks-cash and bonds-cash correlations.
- A new plan can start from one of the ten example plans, picked at the top of
  the new-plan form and saved under the example's name.
- The new-plan form asks when each person started working, and takes a retired
  person's salary as what they last earned, so a retiree's Social Security is
  computed from their career as a worker's is. A retiree who has not reached
  their claim age claims at it.
- On the Monte Carlo and Historical pages the cursor rests on "As planned", and
  ⏎ there shows the plan's own projection in the Ledger.
- The planner runs in a browser page, published to GitHub Pages with each
  release. Its workspace is kept in the browser's own storage and never leaves
  it; the page reopens the plan last open, follows the browser's light or dark
  preference, and Quit starts it over. `upload` and `download`, in the page's
  command palette, bring plans, scenarios and Social Security statements in -
  asking before one replaces a file of the same name - and take the open
  document's file back out as saved. Searches run on the page between frames, so
  a large Monte Carlo run holds the page until it answers.

### Changed

- A form stands only as tall as the fields the item has a use for, and the
  smallest terminal the planner takes no longer grows with its tallest form.
- The Roth Conversions tool converts into the plan's one Roth account without
  asking, so its ladders are ranked on a first look.
- The Ledger's detail is as tall as the cursor year needs, up to half the page,
  and Income & Tax leaves out what the year paid nothing on.
- The key row's words are shorter, so more of each page's keys fit at 128
  columns, and a tool's write and take keys are shown only once there is
  something to write or take.
- A table too narrow for its columns cuts a long header before any cell, and one
  with room to spare uses all of it. A tool table's message wraps rather than
  being cut off.
- The status bar cuts a long file name in the middle, keeping its end.
- Every built-in theme colours the good and caution zones from its own palette,
  and draws a text caret in its accent. The market runs table's title carries
  the zone colour.
- Roth Conversions, SSA Benefits and Historical, opened on a plan the Overview
  has already searched, show the Overview's answer at once rather than searching
  the same plan again.

- The minimum versions of the engine's and the binary's dependencies are now the
  versions they are built and checked against.
- Validation names the entry at fault in a list: a negative prior-year MAGI is
  reported at `medicare.prior_magi[i]` and a repeated withdrawal class at
  `plan.withdrawal_order[i]`, rather than at the whole list.
- An item open in a form while the plan changes underneath is followed to
  wherever it now sits: applying no longer refuses because an undo or a reload
  moved it. It is still refused when the item itself was changed or renamed.
- `optimize conversions` and MCP `sweep_conversion_brackets` list the brackets
  best first - the least left unfunded, then the most left at the end - as the
  Roth Conversions tool does, rather than by rate.
- The conversion source is optional: without `--from` on the CLI, or `from` over
  MCP, a ladder converts from every deferred account of the destination's owner,
  as the Roth Conversions tool does when its source is left blank. Where that
  owner holds none, the search says there is no deferred account to convert
  from.
- The new-plan form numbers the ids it makes from a person's name, as new items
  are numbered: Alice is `alice-1`, a second Alice `alice-2`, and their
  retirement and workplace account follow as `retire-alice-1` and
  `alice-1-401k`.
- MCP `project_plan` and `tax_parameters` describe their replies with full
  output schemas - summary or full rows, and every tax table - and an issue in
  any reply is described as the engine's own. Replies are unchanged.
- A computed Social Security benefit is priced at the claim's age in months, not
  whole years: a claim on a date is priced from that date's month, and a claim
  on an event or another income from the month of the date or age it rests on. A
  claim at 62 starts in the first month 62 is held throughout, the month after
  the birthday unless it falls on the 2nd, so most claims at 62 are 59 months
  early rather than 60. A computed benefit claimed before the month 62 is
  attained is refused.
- Every Social Security income's first year, typed or computed, is paid from the
  month it starts, not only one started on its owner's age.
- Credits for delaying past full retirement age that are earned in the claim
  year are paid from the next January, as SSA pays them, except for a claim
  at 70.
- A benefit whose owner turned 62 before the plan starts is carried to the
  plan's first year by the COLAs SSA published, truncated to the dime after
  each, rather than at the plan's inflation. The tax tables carry every COLA
  from 1975 to 2025.
- A person with no earnings record is taken to have worked from 22 until the
  plan at the salary its first year pays them, so a computed benefit counts the
  years before the plan; a stated record, even a partial one, is used as it
  stands. The Overview says the benefit comes from an estimated career.
- An earnings statement row stating several years as one sum is spread evenly
  over them rather than refused, and the import says which years it spread; MCP
  `import_earnings` says so in a new `note`.
- A tax parameter file for a year that leaves out a state takes that state from
  the latest earlier year that has it, inflated, so a plan living there is no
  longer refused.
- The engine is 0.2.0: `tax::social_security_benefit` takes an age in months and
  the COLAs to carry the benefit through, `tax::claim_year_share` takes the
  first paid month, `BenefitParams` gains `cola`, `Statement` gains `grouped`,
  and `StatementError::Grouped` is gone. `optimize::optimize_claims` and
  `optimize::sweep_brackets` take a `Progress` and answer a `RunError`, which
  now live in a `search` module and are still re-exported from `market`.
- A conversion sweep searches its brackets side by side on the machine's
  threads, so `optimize conversions` without `--bracket`, MCP
  `sweep_conversion_brackets` and the Roth Conversions tool answer in about half
  the time.
- A claim or ladder search that a newer one replaces - an edit made while the
  Overview, SSA Benefits or Roth Conversions page is searching - stops rather
  than running to its end, and the SSA Benefits page works out each person's
  estimates off the main thread.
- Turning back to an editing page whose plan has not changed keeps its table as
  it was rather than rebuilding it, typing in a picker rewrites its rows rather
  than making them anew, and `tab` finds the page's panes without visiting every
  row of its tables.
- The interactive planner is its own library crate, `retiretui_tui`, which the
  `tui` subcommand and the browser page both run; the CLI and MCP take their
  plan-file reading and the words their tables and actions are said in from it.
  It reads and writes every plan file through a `Store` - the disk, or keys in a
  browser's storage - and is built on plurimus 0.7.2 and plurimus_filepicker
  0.1.1.

### Fixed

- A computed Social Security benefit claimed before the plan starts is priced at
  the age it was claimed at, not at the age its owner has reached in the plan's
  first year.
- Taking a searched Roth conversion ladder no longer removes a conversion of
  your own whose id happens to start with `opt-`; only the optimizer's own
  `opt-<account>-<year>` conversions are replaced.
- After a reload fails, the planner and the Compare page watch the files the
  plan now reads, so fixing a newly named base plan is picked up without
  pressing `r`.
- An issue against one place of a list in a form - one year's prior income -
  marks and names that row alone, not every row of the list.
- A list a form refuses keeps its amounts written as money once the keyboard
  leaves them, rather than as bare digits.
- A value an item has no use for, cleared by hand, leaves the form once the
  keyboard leaves its row.
- A trigger's kind menu lists its blank - the plan's start, say - first.
- The new-plan form refuses to write a plan that does not validate, as every
  save does.
- An issue against one step of a glide path marks that step's row in the form.
- A mix whose stocks are written as a whole number, `stocks = 1`, no longer
  gains a full share of cash when applied.
- The Ledger's warnings state their amounts in the dollars on show, not always
  in nominal dollars.
- A click in a table whose pane has grown, after a terminal resize say, lands on
  the row drawn under the pointer.
- A person's claim held out of the claim search is no longer held for a person
  of the same id in the next document opened.

## [0.1.0] - 2026-09-25

### Added

#### Plans

- Plan files: a TOML description of a one- or two-person household - people,
  accounts, income, expenses, contributions, transfers, conversions and events.
- Accounts: 401(k), 403(b), 457(b), 414(k), traditional and Roth IRAs, HSA,
  brokerage and cash, with cost basis, locks and a withdrawal order.
- Triggers: anything starts or stops on a date, an age, or a named event or
  income, with a year offset.
- Escalation: each amount grows with inflation, stays flat in nominal dollars,
  or grows at a rate of its own.
- Contributions: fixed dollars, a share of salary, the legal maximum, or an
  employer match, held to each year's IRS limits.
- Residency: where the household lives over time; state income tax follows a
  move.
- Scenarios: a file naming a base plan and stating only what differs from it.
  Scenarios can build on other scenarios.
- Market assumptions: per-account stock, bond and cash mixes or glide paths, and
  a `[market]` table of returns, volatility and inflation.
- Social Security: a benefit stated as an amount, or computed from the person's
  earnings record with SSA's formula.
- Opt-in Medicare IRMAA surcharges, and MAGI cliffs such as ACA premium credits.

#### Engine

- Federal tax: brackets, capital gains, taxation of Social Security, RMDs,
  early-withdrawal penalties and contribution limits, with 2026 tables built in.
- State income tax for Oregon and the states without one.
- Year-by-year projection in nominal dollars, readable in today's dollars, with
  tax-aware withdrawals.
- Each projected year lists the actions taken: contributions, transfers, RMDs,
  conversions, withdrawals and surplus saved.
- Monte Carlo runs from the plan's assumptions or resampled history, and
  historical runs from every start year since 1871.
- Tax tables and market history can be replaced by files in the user config
  directory.

#### Command line

- `validate`, `project` (text or JSON), `actions` (one year's to-dos) and
  `compare` (plans side by side).
- `optimize conversions`: finds the Roth conversion ladder that fills a tax
  bracket, optionally under an IRMAA tier or MAGI cap.
- `optimize claims`: ranks Social Security claim ages for the household.
- Both optimizers can write their result as a scenario.
- `monte-carlo` and `historical`: how often the plan's money lasts, with
  percentile runs.
- `import-earnings`: records the earnings from an `ssa.gov` statement on a
  person.
- `mcp`: the same tools for AI agents over the Model Context Protocol, limited
  to one directory, with a plan schema reference.

#### Interactive planner

- `retiretui tui`: a keyboard- and mouse-driven planner for a plan, a scenario
  or a directory.
- Overview: whether the money lasts, milestones, warnings, the year's to-dos,
  balance charts and suggested improvements.
- Ledger: every year's totals, and each account's flows, income and taxes for
  the selected year.
- Compare: the open plan beside other plans, with their differences and charts
  by year.
- Tools: the conversion and claim-age searches, Monte Carlo and historical runs.
  A result can be applied to the plan or saved as a scenario.
- Editing: a page per plan section and a form per item, validated on apply, with
  undo and redo.
- A new-plan form that builds a starting plan from a few answers.
- A file picker for opening and saving; open files reload when they change on
  disk.
- A command palette, key hints, a message log, themes and reduced motion.

#### Project

- Example plans in `examples/`.
- Crates on crates.io and prebuilt binaries for x86_64 and aarch64 Linux.
