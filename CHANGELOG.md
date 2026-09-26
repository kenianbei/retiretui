# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

- The minimum versions of the engine's and the binary's dependencies are now the
  versions they are built and checked against.
- Validation names the entry at fault in a list: a negative prior-year MAGI is
  reported at `medicare.prior_magi[i]` and a repeated withdrawal class at
  `plan.withdrawal_order[i]`, rather than at the whole list.
- An item open in a form while the plan changes underneath is followed to
  wherever it now sits: applying no longer refuses because an undo or a reload
  moved it. It is still refused when the item itself was changed or renamed.

### Fixed

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
