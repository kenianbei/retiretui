# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
