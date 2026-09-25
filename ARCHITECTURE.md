# RetireTui Architecture

RetireTui is a local-first retirement planner for the terminal. The user owns a
plain TOML plan file describing a household - people and filing status, each
person's covered earnings where a Social Security statement has been imported,
accounts with tax treatments and what each is invested in, contributions into
them, income sources, expenses, named milestones, opt-in Medicare surcharge
modeling and MAGI-cliff declarations, and what it assumes of the market - and
the program answers with a deterministic year-by-year projection of that
household's finances under U.S. federal tax law and the income tax of the state
it lives in, year by year, and with how that projection fares across many
markets. All amounts are entered as annual today's dollars and escalate per
item - at plan inflation by default, frozen nominal, or at a fixed rate of their
own; the engine computes in nominal dollars and carries a per-year deflator so
results read in either basis. A projection walks through a market - each year's
return on each asset class and that year's inflation - and the ledger is the one
market the plan states: each class's mean return and the plan's inflation, every
year.

A plan variant is a scenario: a TOML file naming a `base` document and stating
only deltas. Items are addressed by identity - the `id` every listed item
carries - and stated fields replace the base item's as whole values; markers
delete or substitute an item, an unmatched fragment appends and must validate on
its own, so a typo'd id cannot apply silently. A base may itself be a scenario;
chains resolve bottom-up with cycle detection. Every surface accepts a scenario
wherever it accepts a plan.

A cargo workspace splits the system into two crates with a one-way rule: logic
never depends on UI.

- `retiretui_engine` - everything that computes. `plan` is the schema, its
  semantic validation, the scenario overlay merge, and how one plan differs from
  another - items matched by id as the merge matches them - both working on
  parsed TOML tables so the engine stays IO-free; it also states its closed
  vocabularies - the kinds, statuses, trigger bases, and places a plan may
  name - so that no surface restates them. `params` holds per-year tax
  parameters: values are data, embedded as TOML tables for known years,
  overridable from user directories, and extended past the last known year by
  inflating indexed values by the inflation of the market projected through -
  and the national average wage index, grown past its last published year at an
  assumed rate a plan may override, from which the benefit formula's wage bases
  and bend points derive by statute for any year. `tax` holds the formulas: rule
  shapes are code, the Social Security benefit's among them - each year's
  covered earnings capped at its own base and indexed to the average wage of the
  year the worker turns 60, the highest years averaged, the primary insurance
  amount through the bend points of the year they turn 62, the claim age's
  reduction or credit against full retirement age, and the share of the claim
  year paid from the month the age is attained - and a career at one salary
  filled from the wage index, as SSA fills a record from current earnings.
  `statement` reads the statement a person downloads from `ssa.gov` to what a
  plan keeps of it - the birth date and the covered earnings by year - which
  replaces a person's record, once. `project` walks the years - trigger
  resolution, a year's growth on what each account opened with - its fixed
  return, or the year's class returns blended by the mix it holds that year, a
  glide path stepping between mixes as triggers fire - credited before anything
  draws on it, escalation and the deflator following the market's inflation,
  scheduled transfers, RMDs, income and expense windows (a Social Security
  benefit left unstated is computed the year it is claimed, from the owner's
  record extended with the nominal salary the walk has paid them, in the dollars
  of their age-62 year so that the income's own escalation carries SSA's COLAs
  from it; one starting on the owner's age pays its first year from the month
  the age is attained), contributions, Roth conversions, a tax-aware withdrawal
  fixed point that also settles the year's MAGI-driven costs (IRMAA surcharges
  priced from the household MAGI two years earlier, declared cliffs crossed by
  the current year's MAGI) and the MAGI-driven deduction of a covered person's
  IRA contribution, surplus sweeping - emits one row per year, and aggregates a
  projection into headline summary figures in either dollar basis. A
  contribution is an item of its own that names the account it pays into, who
  pays, and one amount - dollars, a share of a named income, the year's legal
  maximum, or an employer's match on what the employee paid - and the law's
  limits are applied rather than refused: employee amounts are held to each
  person's pooled limit in the order the plan lists them, each plan to its
  yearly cap, and what an account holds after tax comes back untaxed pro rata
  whenever it is drawn. Every year's row says how each contribution came to be
  what it is. `optimize` searches by re-projecting candidate plans - no
  closed-form tax approximations: fill-bracket Roth conversion ladders, each in
  place of any ladder the plan already holds, against a two-sided target, the
  bracket top in taxable-income space and optional MAGI ceilings (an IRMAA tier,
  an explicit cap, active cliffs), and Social Security claim ages, every
  computed benefit - and one made up for anyone with an earnings record and
  none, save the people whose claims are held as the plan states them - tried at
  each whole age it can still reach, jointly for the household, ranked by what
  the household ends with; beside the search, a person's benefit is estimated at
  the ages that frame the choice, by projection; each search emits its answer as
  a scenario overlay through the schema's own serialization. Each projected row
  also records the actions the engine executed - transfers, RMDs, contributions,
  conversion steps, funding withdrawals, the surplus swept - with post-clamp
  nominal amounts, and what each account grew, so every surface can answer "what
  do I actually do this year" and where every account's money went without
  re-deriving execution. `market` makes the markets a plan is walked through -
  correlated draws from the plan's `[market]` assumptions on a seeded generator
  of the engine's own, so a saved seed draws the same markets, historical years
  bootstrapped in blocks, or history replayed from a start year - from an
  embedded yearly record of U.S. returns and inflation since 1871 that a user
  file may replace, and runs a plan through many of them at once across threads,
  keeping of each run only what the tools show: success, ending, shortfall and
  net worth by year in that run's own today's dollars, with percentile bands and
  the runs singled out.
- `retiretui` - the single user-facing binary; surfaces are clap subcommands.
  One shared resolver follows scenario base chains - reading files and resolving
  paths is surface policy: relative to the referring file on the CLI, contained
  in the served directory under MCP. `validate` runs the engine's validation
  contract; `project` renders the ledger as a text table or as JSON; `actions`
  prints one year's recorded to-dos with warnings, defaulting to the current
  calendar year; `compare` projects two or more plans side by side - a summary
  row per plan, one metric year by year, or JSON; `optimize` sweeps or targets a
  tax bracket and writes the searched conversion ladder as a scenario overlay,
  or ranks every claim age for the household's computed Social Security benefits
  and writes the best; `monte-carlo` and `historical` run the plan through
  random markets or every historical start year and report the share it
  survives, their settings the plan's and overridable by flag; `import-earnings`
  records a statement's earnings on a person and writes the plan back
  canonically, the one CLI command that rewrites a plan file; `tui` opens an
  interactive planner - a plurimus (Bevy-in-the-terminal) app shaped like a
  desktop one, driven by keyboard or pointer: a row of bordered tabs naming the
  five places to be, the page one of them shows, and a row naming the keys in
  reach. A tab is selected by its own digit, by the pointer, or by stepping the
  row; three open a page of their own and the last two each hold a group of
  pages - the tools that act on the plan as a whole, and the plan's editing
  domains - a sidebar naming the group's pages beside whichever is on show: the
  sidebar's cursor and the page are one fact, whichever of them moves, and a
  grouped tab comes back to the page last shown through it. It opens on a plan,
  a scenario, or a directory - the working directory by default - and holds one
  document at a time: a command opens another or saves the draft under another
  name, each through one file picker - a path field over the directory it names,
  opened on the workspace, the directory beside the document or the one launched
  on while there is none, and listing that directory's files of the kind wanted
  and every directory beneath or above it, so a plan anywhere is reached by
  typing or completing its path - which takes a name no file has as a new one
  wherever a file is to be written. Without a document there is nothing to view,
  so the four tabs with a page behind them are drawn dead and the form a new
  plan starts from stands over the empty shell: a household's filing status,
  where it is in life, and each person's name, birth year, retirement age,
  salary and Social Security - which a working person may leave for the engine
  to compute, from a career at that salary. Creating it builds the plan those
  answers describe, names it through the picker that saves under another name,
  and opens it, so nothing reaches disk until it is named and everything after
  the first answers is edited in the plan's own domains. The same form stands
  over an open document when a new plan is asked for, the document staying open
  beneath it and taking no key while it does; alone, it lets the shell's own
  keys through. Each command's scope states whether it runs where no page is
  shown - what finds a document, ends the session, dresses the shell, or moves
  the keyboard between panes does, and the rest refuse. A page is a view of the
  projection (an overview answering what the plan's owner asks of it - whether
  the money lasts and how surely, when the big things happen, what needs
  attention, what to do in the year, how the money is split between tax
  treatments, and what the optimizers find better, searched in the background
  while it is shown - each answer leading to the page its detail lives on; a
  year ledger - the plan's own projection, or a market run opened from a market
  tool until `esc` or an edit returns it - over the cursor year's flows, each
  account from its open to its close with every flow in and out named by where
  it came from or went, beside its income and tax - the overview and the ledger
  sharing one year cursor, today until moved and always within the plan's years,
  which each follows when the other moves it and the charts also set under a
  click and read out under the pointer), one of the plan's editing domains, or
  one of what runs over it: the document compared with other workspace files,
  which follow the disk as the document does - each plan's figures, its success
  through random markets, and what it changes of the one chosen as the baseline,
  beside the plans charted or tabled year by year, whole or as their difference
  from the baseline, ⏎ on one taking it into the document's place with the
  others kept - and the tools, each panes of its own over a line of help and a
  search on a thread of its own that runs by itself whenever what it would
  search changes, its options ranked best first in one shared table under a row
  for the plan as it stands - the conversion search's beside what it runs under,
  read out and edited as a domain's one item is, and over the highlighted ladder
  year by year, the claim search's beside a table of each person's record,
  income and estimated benefit, ⏎ on a person offering what can be done for
  them, and the claims held out of the search among what it watches, and the
  market tools' runs - the plan through random markets, or from every historical
  start year worst first - beside what they run under and how the plan fared,
  over a chart of the runs' spread that `v` turns to other views, a newer search
  stopping one under way and ⏎ on a run opening it in the ledger - the searches'
  highlighted option written as a scenario over the document into the workspace
  and compared at once, or taken into the draft, after asking, as one applied
  item - a Roth conversion ladder as conversions of its own, in place of the
  ladder taken before, a set of Social Security claims as each searched income's
  start, adding the incomes the search made up. Viewing and editing are
  distinct: a domain with many items is a table, shown in the plan's order or
  ordered by a column for the view alone, with the row under the cursor read out
  beside it, every field the item has a use for in the form's words, wherever
  the columns do not say everything - a person's ending with their earnings
  record - and a domain there is exactly one of is that read-out alone. Nothing
  on a page edits: one item at a time is the editing session, a form standing
  over the page, opened by ⏎ on a row or on the read-out and left by esc or by
  applying, the keyboard going back to what opened it; its fields work on a
  snapshot that reaches the working draft only when the whole item is applied.
  Edits no one applied are never dropped silently - the form keeps every key
  while it stands, and a press outside it asks what is to become of them - and
  never applied to an item the plan has changed underneath; an unsaved draft is
  asked about the same way before another document takes its place. Every
  applied item is one step of a whole-plan history the draft walks back and
  forward through, dropped with the document and kept across a save; a statement
  picked on the People page or beside the claim search, through the same file
  picker, lands its earnings on the highlighted person as one such step. Each
  field is entered by its kind - ticked, or picked from a closed set wherever
  the schema states one, from a menu or, where the set is too long for one,
  through the fuzzy picker, read from the engine rather than restated, and of
  the plan's own ids wherever it names one, so an invalid value or a misspelt
  reference cannot be expressed, and a pick the schema requires cannot be
  emptied. A value with parts of its own is rows of the same form rather than
  text: a trigger is picked apart into its kind and that kind's operands, a
  table the item holds is fields that reach into it - made with its first value
  and gone with its last, or, where its being there is itself the setting,
  ticked, its rows shown only while it is - and a list is rows that hold it
  between them, an order offering each place only what no other holds. A field
  is shown only while the item has a use for it, asked of the engine's own
  rules - a basis on the kinds of account that keep one, a window on what recurs
  and a single date on what happens once - and what is applied leaves out
  whatever is not. A value the file states one of several ways is chosen between
  by a pick no file holds, read from the item on open and written back as the
  keys the file does hold. What such rows do not yet make a value of refuses the
  apply and says why. What remains is typed: money, a rate and how an amount
  grows read back exactly what they show - separators, a percent, a word - and a
  name, an id or a date is the plan file's own value syntax, parsed through the
  schema's types, so what a field cannot read the schema refuses in its own
  words. The file's spelling stays in the file: each field states a label and a
  description beside its key, the schema's closed sets and the plan's items are
  offered under words and display names over the values kept, one module turns a
  value into the phrase shown for it wherever it is shown, and an issue's path
  is read back into the page, item and field it names - so a form, a table, a
  menu and an issue say the same thing in the same words. Every action the shell
  can take is a row in one static command table, which the keys, the tabs' own
  digits, the key row, and the fuzzy pickers that find a command or a page all
  read from; a key particular to a page runs only while that page is on show,
  ahead of any meaning the shell gives the same key, so two pages may bind one
  key each and a page may take one of the shell's. The keyboard walks a page's
  panes in the order they are drawn, the sidebar first beside a grouped page,
  and a page is entered on its first pane. Whatever stands over the page - a
  menu, a picker, a dialog, an open item - takes the keyboard on a stack and
  gives it back to what held it, and a command chosen from one runs once it has,
  since what holds the keyboard is what a command acts on. Whether a key is a
  command at all is asked of the widget it was typed at and everything that
  widget sits in: what stands over the page keeps every key, and a form's fields
  and buttons keep the plain ones. Everything the shell says is a `tracing`
  event with two readers: a journal the shell toasts from and lists in a drawer,
  and a log file. Colour is named by role, never by value: a theme is a table of
  roles, the terminal's own colours by default, and a cell no widget coloured is
  drawn in the theme's own ground. What the user sets - theme, motion - lives in
  one user config file the shell reads at launch and writes back a key at a
  time, leaving the rest of the file as the user wrote it. Each applied item
  re-validates the draft: a valid draft is re-projected at once so the views
  follow it, and an invalid one holds the last good view, reports its first
  issue, counts them beside the file name, and lists every one in a panel whose
  rows turn to the item. Saving writes the draft as canonical TOML through the
  same validation gate as every other write; scenario sessions are read-only,
  since a resolved plan cannot be written back into an overlay, and saving one
  under a new name writes the resolved plan as a plan of its own. The resolved
  chain's files are watched so on-disk edits re-project in place, except under
  an unsaved draft or an item being edited, which is reported rather than
  overwritten, and so are each compared file's, which have no draft to protect;
  `mcp` serves the same contract to AI agents over stdio - list, read, validate,
  write, project, actions, compare, earnings-import, optimizer and market tools
  over plan files sandboxed to a served directory, plus tax-parameter lookup and
  an embedded schema reference. Writes are gated on full validation - scenarios
  validated fully resolved - and stored in canonical TOML; the schema
  reference's worked example is kept valid by the test suite.

Plans express timing through a closed trigger vocabulary - a fixed date, a
person's age, or a reference to a named event or income source with a whole year
offset - resolved once per projection. There is no predicate language in plan
files, for the same reason tax formulas stay in code. The engine trusts a
validated plan: validation, including the horizon-wide check that reads the tax
tables - a table for every state lived in, and the benefit formula's amounts
behind a computed Social Security benefit, claimed at 62 or later - is the
boundary, and every surface runs it before projecting; what the tables limit
year by year, such as what may be paid into an account, the projection applies
and reports rather than refuses.
