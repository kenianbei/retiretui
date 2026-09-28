# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- A web app for phones and desktop browsers, published to GitHub Pages at the
  site's root with each release. It keeps its plans in the browser's own
  storage, apart from the canvas page's, and never sends them anywhere. A first
  plan is made from a few questions asked a step at a time - the household, then
  each person - which read every answer back to be changed before the plan is
  named, made and opened on the Overview; a blank age, start of work or Social
  Security benefit reads as what the plan will assume, and the answers survive a
  reload until the plan is made. The example plans and an upload are offered
  beside them. It reopens the plan last open, and downloads the open plan. Its
  Overview shows whether the money lasts - its headline figures in today's or
  future dollars, and the share of a thousand random markets it survives, run
  off the page's thread - and charts the plan: its balances by tax treatment
  under its net worth, its net worth, its income against its taxes, and its net
  worth through the random markets as percentile bands, each salary's end
  marked. Beneath them is what to do in the year shown, said as the terminal
  says it: today's by default, stepped by button or the arrow keys, or chosen
  with a click on a chart. The Ledger shows the plan year by year, the year
  column staying in view as a phone scrolls sideways, over the chosen year's
  flows through each account - where each came from or went, with the year's
  warnings - and its income and what it paid. The year and the dollars are in
  the page's address, kept between the Overview, the Ledger, Compare and the
  tools and across a reload. The Plan pages edit the plan: each domain's items
  are a table sortable by its columns, or rows with a Sort by on a phone, beside
  the highlighted item read out in the form's words, and a domain there is one
  of is that read-out alone. Edit, Add and Delete work one item at a time; the
  form is a sheet, full screen on a phone, whose fields are entered by their
  kind - picks from the schema's sets and the plan's own ids, a searched list
  for a country or U.S. state, a slider beside a rate, a trigger as its kind and
  sentence - and applying stores the whole item or says why not. Every applied
  item or deletion is a step Undo and Redo walk, by button or Ctrl/Cmd+Z, and
  Save or Ctrl/Cmd+S writes the plan back through the same validation as every
  other surface; Save as… writes it under another name, the one way to keep a
  scenario's edits. Unsaved edits are asked about before a form closes, another
  plan opens, or the page is left, and a file changed in another tab under them
  is reported rather than overwritten. A plan with issues counts them in the
  header and lists each by the page, item and field it is about, each a link to
  that field in its form, while the figures keep the last ones it had without
  issues. A Social Security statement downloaded from ssa.gov is imported onto a
  person from the People page as one step of history. The Roth Conversions tool
  searches every fillable bracket's conversion ladder under constraints read out
  on the page and edited in a sheet, ranks them under the plan as it stands, and
  shows the highlighted one's conversions year by year, in either dollar basis;
  it searches again whenever the plan or the constraints change, and takes the
  plan's one Roth account as the destination where there is only one. The
  highlighted ladder is taken into the plan after asking, as one step of history
  in place of any ladder taken before, or written as a scenario beside the saved
  plan, compared with it at once and offered to open. The SSA Benefits tool
  ranks every claim age for the household's computed Social Security benefits
  under the plan as it stands, beside each person's record, income and benefit
  estimated at 62, full retirement age and 70, with what can be done for the
  highlighted person: import a statement, estimate a record from their salary,
  compute a typed benefit from their record, clear the record or remove the
  benefit - the last two asked first - each one step of history, or hold their
  claim as the plan states it while the others are searched. The highlighted
  claims are taken into the plan after asking or written as a scenario, as a
  ladder is; the claims highlighted, the person and who is held are kept in the
  address. The Overview's Could do better card gives each Roth owner's best
  ladder and the household's best claims against the plan as it stands, each
  leading to its tool, a ladder's aimed at that owner's account; what it finds
  is what the tools then show without searching again. The Monte Carlo and
  Historical tools run the plan through a thousand random markets, or from every
  historical start year worst first, and say how it fared in the colour of its
  zone - as the Overview's figure is coloured - beside what the runs were made
  under, each a link to the Market field it is edited at; the runs singled out
  are charted as the spread of net worth under the highlighted run's line, net
  worth by year at each percentile (Monte Carlo only), the share still funded,
  and what the runs end with. The highlighted run opens in the Ledger, replayed
  through its market and named by it - a random market's number, or the year
  retired into - kept in the address through the year, the dollars and edits,
  with a link back to the plan's own. Compare sets the plan beside the workspace
  plans chosen for it: each plan's figures and its success through random
  markets - each plan searched in a worker of its own - or its differences from
  the one chosen as the baseline, what the highlighted plan changes of the
  baseline, and one of eight metrics year by year as a chart or a table. The
  plans compared, the baseline, the metric and the view are kept in the address,
  the plans compared carried by every link; opening a compared plan puts it in
  the document's place with the document joining the compared, and opening any
  other plan leaves nothing compared. The Tax Tables tool reads out the tables
  the plan's projection applies in the year shown - brackets, deductions,
  long-term gains, Social Security's thresholds and benefit formula, the
  contribution limits, Medicare surcharges, the RMD divisors and the state's
  income tax - for the plan's filing status and the state it lives in that year,
  grown past the last published table at the plan's inflation, or for any status
  or modeled state picked. Manage plans, in the File menu, renames, downloads
  and deletes the workspace's plans: a rename rewrites the scenarios built on
  the plan and keeps an open plan's unsaved edits, a delete names the scenarios
  it breaks, and both are followed by the plans compared and by other tabs. The
  page says that clearing the browser's site data deletes the plans, and asks
  the browser to keep them on a button. A palette opened with Ctrl/Cmd+K finds
  any page, plan or action by part of its name; the digits 1-5 go to the tabs
  and ? lists every key. The app installs, and works offline once it has been
  visited: each page loads the first time it is shown and the rest once the app
  is idle. A skip link leads to the page, focus moves to a new page's heading,
  and every page is checked against WCAG 2.1 A and AA in Chromium, Firefox and
  WebKit. Light or dark follows the system, and navigation is a bottom bar on a
  phone and a sidebar on a wider screen.
- The engine exposes `Scenario::set_base`, which makes an overlay name another
  base, and `project::benefit_params` and `project::state_lived_in`, the benefit
  formula's parameters and the state taxing a year as a plan's projection takes
  them.

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
- The terminal's new-plan form asks about a partner only once the household
  files jointly.
- The terminal's SSA Benefits tool names claims by person rather than by income
  id: "Take these claims? Ann at 70, Bob at 67."
- The Roth Conversions constraints read a blank as what the search assumes -
  every bracket, the plan's start, no cap - and a blank last year is said to be
  the year before the owner's RMDs begin, as the search has always taken it.
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
