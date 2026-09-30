# RetireTui Architecture

RetireTui is a local-first retirement planner for the terminal and the browser.
The user owns a plain TOML plan file describing a household - people and filing
status, each person's covered earnings where a Social Security statement has
been imported, accounts with tax treatments and what each is invested in,
contributions into them, income sources, expenses, named milestones, opt-in
Medicare surcharge modeling and MAGI-cliff declarations, and what it assumes of
the market - and the program answers with a deterministic year-by-year
projection of that household's finances under U.S. federal tax law and the
income tax of the state it lives in, year by year, and with how that projection
fares across many markets. All amounts are entered as annual today's dollars and
escalate per item - at plan inflation by default, frozen nominal, or at a fixed
rate of their own; the engine computes in nominal dollars and carries a per-year
deflator so results read in either basis. A projection walks through a market -
each year's return on each asset class and that year's inflation - and the
ledger is the one market the plan states: each class's mean return and the
plan's inflation, every year.

A plan variant is a scenario: a TOML file naming a `base` document and stating
only deltas. Items are addressed by identity - the `id` every listed item
carries - and stated fields replace the base item's as whole values; markers
delete or substitute an item, an unmatched fragment appends and must validate on
its own, so a typo'd id cannot apply silently. A base may itself be a scenario;
chains resolve bottom-up with cycle detection. Every surface accepts a scenario
wherever it accepts a plan.

A cargo workspace splits the system into crates with a one-way rule: logic never
depends on UI.

- `retiretui_engine` - everything that computes. `plan` is the schema, its
  semantic validation, the scenario overlay merge and the resolution that
  follows a scenario's base chain through whatever reads the files it names, and
  how one plan differs from another - items matched by id as the merge matches
  them - both working on parsed TOML tables so the engine stays IO-free; it also
  states its closed vocabularies - the kinds, statuses, trigger bases, and
  places a plan may name - so that no surface restates them. `params` holds
  per-year tax parameters: values are data, embedded as TOML tables for known
  years, overridable from user directories - a year's table leaving out a state
  takes it from the latest earlier table that has it - and extended past the
  last known year by inflating indexed values by the inflation of the market
  projected through - and the national average wage index, grown past its last
  published year at an assumed rate a plan may override, from which the benefit
  formula's wage bases and bend points derive by statute for any year, beside
  every cost-of-living adjustment SSA has published. `tax` holds the formulas:
  rule shapes are code, the Social Security benefit's among them - each year's
  covered earnings capped at its own base and indexed to the average wage of the
  year the worker turns 60, the highest years averaged, the primary insurance
  amount through the bend points of the year they turn 62 and through each COLA
  from that year, truncated to the dime after each, the reduction or credit for
  each month the claim falls before or after full retirement age - credits
  earned in the claim year paid from the next January, save at 70 - and the
  share of the first year paid from the first month paid for, 62 being held
  throughout it - and a career at one salary filled from the wage index, as SSA
  fills a record from current earnings. `statement` reads the statement a person
  downloads from `ssa.gov` to what a plan keeps of it - the birth date and the
  covered earnings by year, a sum stated for several years spread evenly over
  them - which replaces a person's record, once. `project` walks the years -
  trigger resolution, a year's growth on what each account opened with - its
  fixed return, or the year's class returns blended by the mix it holds that
  year, a glide path stepping between mixes as triggers fire - credited before
  anything draws on it, escalation and the deflator following the market's
  inflation, scheduled transfers, RMDs, income and expense windows (a Social
  Security benefit left unstated is computed the year it is claimed, at the
  month of the date or age its start rests on, from the owner's record - or,
  where they have none, a career before the plan at the salary its first year
  pays them - extended with the nominal salary the walk has paid them, and
  carried from their age-62 year by the COLAs SSA has published so that the
  income's own escalation carries those still to come; every Social Security
  benefit pays its first year from the month it starts), contributions, Roth
  conversions, a tax-aware withdrawal fixed point that also settles the year's
  MAGI-driven costs (IRMAA surcharges priced from the household MAGI two years
  earlier, declared cliffs crossed by the current year's MAGI) and the
  MAGI-driven deduction of a covered person's IRA contribution, surplus
  sweeping - emits one row per year, and aggregates a projection into headline
  summary figures in either dollar basis. A contribution is an item of its own
  that names the account it pays into, who pays, and one amount - dollars, a
  share of a named income, the year's legal maximum, or an employer's match on
  what the employee paid - and the law's limits are applied rather than refused:
  employee amounts are held to each person's pooled limit in the order the plan
  lists them, each plan to its yearly cap, and what an account holds after tax
  comes back untaxed pro rata whenever it is drawn. Every year's row says how
  each contribution came to be what it is. `optimize` searches by re-projecting
  candidate plans - no closed-form tax approximations: fill-bracket Roth
  conversion ladders, each in place of any ladder the plan already holds,
  against a two-sided target, the bracket top in taxable-income space and
  optional MAGI ceilings (an IRMAA tier, an explicit cap, active cliffs), and
  Social Security claim ages, every computed benefit - and one made up for
  anyone with an earnings record and none, save the people whose claims are held
  as the plan states them - tried at each whole age it can still reach, jointly
  for the household - each search ranking what it finds by what the household
  ends with; beside the claim search, a person's benefit is estimated at the
  ages that frame the choice, by projection; each search emits its answer as a
  scenario overlay through the schema's own serialization. Each projected row
  also records the actions the engine executed - transfers, RMDs, contributions,
  conversion steps, funding withdrawals, the surplus swept - with post-clamp
  nominal amounts, and what each account grew, so every surface can answer "what
  do I actually do this year" and where every account's money went without
  re-deriving execution. `market` makes the markets a plan is walked through -
  correlated draws from the plan's `[market]` assumptions on a seeded generator
  of the engine's own, so a saved seed draws the same markets, historical years
  bootstrapped in blocks, or history replayed from a start year - from an
  embedded yearly record of U.S. returns and inflation since 1871 that a user
  file may replace, and runs a plan through many of them at once across the
  machine's threads where it has more than one, keeping of each run only what
  the tools show: success, ending, shortfall and net worth by year in that run's
  own today's dollars, with percentile bands and the runs singled out. `search`
  is what every search shares: a way to follow it and to stop it at its next
  step, and the machine's threads, where there are any, to run its independent
  steps across - a market's runs, a sweep's brackets.
- `retiretui_client` - what every interface shares over the engine, and draws in
  its own way; it depends on none of their own crates. The words a plan is said
  in - a value, a table, an action, and an issue read back as the domain, item
  and field it is about. The form model - each editing domain by an id of its
  own, its fields in a form's words and under the headings that gather them,
  what edits each and when the item has a use for it, what one left blank stands
  for - said one way while entered and another once read back - the choices a
  closed set or the plan's own ids offer - and the item open in a form: what its
  fields hold, applied whole into the draft or refused with why, a trigger or a
  list made from the parts it is entered in, a table's items ordered by a
  column, and an item read out in the form's words. A year's ledger in those
  words: its columns, each account's flows from its open to its close named by
  where each came from or went, its income and what it paid, and the years each
  earner's salary ends. The load-and-validate gate, a scenario's base chain
  followed over whatever reads its files and one that does not resolve said by
  the file in it that failed, where in its text and why, the file a scenario's
  base names found beside it and a scenario made to name another, and every plan
  file read, written, listed and stamped through one store - the disk, or files
  kept as keys of a browser's storage, each file's count of writes its stamp.
  The document a shell holds open, its projection, and the draft every edit
  lands in with its whole-plan history. A first plan from the new-plan answers -
  asked of a partner only where the household files jointly, in the steps a page
  asks them in - or an example; a statement's earnings recorded on a person; the
  searches the tools and the overview share, the words their options and what
  the overview finds better are said in - the market runs' among them: how the
  plan fared and in which zone, what the runs were made under and where each is
  edited, how they end, and the market a run went through as an address keeps it
  and as it is named - and what can be done for a person beside the claim
  search, each edit said and refused in its own words; what the Overview lists
  beside its verdict - the draft's issues, the years the plan pays Medicare's
  surcharges, the contributions it could not make as stated, a benefit estimated
  without its record, an amount too large to be likely, and the plan's
  milestones - each row led by its year or the item behind it, and a year's
  actions and warnings in either dollar basis; what plans compared side by side
  are said in - each one's figures, or its differences from a baseline's, a
  metric year by year, and what one changes of another; a year's tax tables as a
  plan's projection applies them - for its filing status and the state it lives
  in that year, or any other - in the words its other tables are said in; and
  the shapes a search or a year's actions are replied in as data. Where there is
  a machine beneath it, what that machine supplies: the user's own tax tables,
  market history and directories.
- `retiretui_tui` - the interactive planner described under `tui` below, as a
  library over the client and above whatever backend draws it, mapping each
  editing domain to its page. It runs each search beside the frames: on a thread
  of its own, or, where there are no threads, whole on the frame after the one
  that shows it under way. A launch names what the session opens, its store and
  settings file, whether the screen it is drawn on is light, and, for a page,
  that the document last open is reopened, the directory no picker climbs above,
  and what hands files across the page's edge. Behind a feature, it holds the
  launcher that runs it in a terminal.
- `retiretui` - the single user-facing binary, composing each interface's clap
  subcommands into one command: the command line's from `retiretui_cli`, `tui`
  from `retiretui_tui`'s terminal launcher, and `mcp` from `retiretui_mcp`; each
  is a library of its own over the client. The engine's resolver follows
  scenario base chains - reading files and resolving paths is surface policy:
  relative to the referring file on the CLI, contained in the served directory
  under MCP. `validate` runs the engine's validation contract; `project` renders
  the ledger as a text table or as JSON; `actions` prints one year's recorded
  to-dos with warnings, defaulting to the current calendar year; `compare`
  projects two or more plans side by side - a summary row per plan, one metric
  year by year, or JSON; `optimize` sweeps or targets a tax bracket and writes
  the searched conversion ladder as a scenario overlay, or ranks every claim age
  for the household's computed Social Security benefits and writes the best;
  `monte-carlo` and `historical` run the plan through random markets or every
  historical start year and report the share it survives, their settings the
  plan's and overridable by flag; `import-earnings` records a statement's
  earnings on a person and writes the plan back canonically, the one CLI command
  that rewrites a plan file; `tui` opens the interactive planner in the
  terminal - a plurimus (Bevy-in-the-terminal) app shaped like a desktop one,
  driven by keyboard or pointer: a row of bordered tabs naming the five places
  to be, the page one of them shows, and a row naming the keys in reach. A tab
  is selected by its own digit, by the pointer, or by stepping the row; three
  open a page of their own and the last two each hold a group of pages - the
  tools that act on the plan as a whole, and the plan's editing domains - a
  sidebar naming the group's pages beside whichever is on show: the sidebar's
  cursor and the page are one fact, whichever of them moves, and a grouped tab
  comes back to the page last shown through it. It opens on a plan, a scenario,
  or a directory - the working directory by default - and holds one document at
  a time: a command opens another or saves the draft under another name, each
  through one file picker - a path field over the directory it names, opened on
  the workspace, the directory beside the document or the one launched on while
  there is none, and listing that directory's files of the kind wanted and every
  directory beneath or above it, so a plan anywhere is reached by typing or
  completing its path - which takes a name no file has as a new one wherever a
  file is to be written. Without a document there is nothing to view, so the
  four tabs with a page behind them are drawn dead and the form a new plan
  starts from stands over the empty shell: one of the example plans, or a
  household's filing status, where it is in life, and each person's name, birth
  year, retirement age, when they started working, salary and Social Security -
  which anyone may leave for the engine to compute, from a career at that
  salary, a retiree's the last they earned. Creating it builds the plan the
  example or the answers describe, names it through the picker that saves under
  another name, and opens it, so nothing reaches disk until it is named and
  everything after the first answers is edited in the plan's own domains. The
  same form stands over an open document when a new plan is asked for, the
  document staying open beneath it and taking no key while it does; alone, it
  lets the shell's own keys through. Each command's scope states whether it runs
  where no page is shown - what finds a document, ends the session, dresses the
  shell, or moves the keyboard between panes does, and the rest refuse. A page
  is a view of the projection (an overview answering what the plan's owner asks
  of it - whether the money lasts and how surely, when the big things happen,
  what needs attention, what to do in the year, how the money is split between
  tax treatments, and what the optimizers find better, searched in the
  background while it is shown - each answer leading to the page its detail
  lives on; a year ledger - the plan's own projection, or a market run opened
  from a market tool until `esc` or an edit returns it - over the cursor year's
  flows, each account from its open to its close with every flow in and out
  named by where it came from or went, beside its income and tax - the overview
  and the ledger sharing one year cursor, today until moved and always within
  the plan's years, which each follows when the other moves it and the charts
  also set under a click and read out under the pointer), one of the plan's
  editing domains, or one of what runs over it: the document compared with other
  workspace files, which follow the disk as the document does - each plan's
  figures, its success through random markets, and what it changes of the one
  chosen as the baseline, beside the plans charted or tabled year by year, whole
  or as their difference from the baseline, ⏎ on one taking it into the
  document's place with the others kept - and the tools, each panes of its own
  over a line of help and a search beside the frames that runs by itself
  whenever what it would search changes, a newer search stopping one under way,
  taking instead what the overview has already found over the same plan, its
  options ranked best first, each against the plan, in one shared table under a
  row for the plan as it stands - the conversion search's beside what it runs
  under, read out and edited as a domain's one item is, and over the highlighted
  ladder year by year, the claim search's beside a table of each person's
  earnings record, how their benefit is set and its estimates, ⏎ on a person
  offering what can be done for them, and the claims held out of the search
  among what it watches, and the market tools' runs - the plan through random
  markets, or from every historical start year worst first - beside what they
  run under and how the plan fared, over a chart of the runs' spread that `v`
  turns to other views, ⏎ on a run opening it in the ledger, and on the plan's
  own row the plan's own projection - the searches' highlighted option written
  as a scenario over the document into the workspace and compared at once, or
  taken into the draft, after asking, as one applied item - a Roth conversion
  ladder as conversions of its own, in place of the ladder taken before, a set
  of Social Security claims as each searched income's start, adding the incomes
  the search made up. Viewing and editing are distinct: a domain with many items
  is a table, shown in the plan's order or ordered by a column for the view
  alone, with the row under the cursor read out beside it, every field the item
  has a use for in the form's words, wherever the columns do not say
  everything - a person's ending with their earnings record - and a domain there
  is exactly one of is that read-out alone. Nothing on a page edits: one item at
  a time is the editing session, a form standing over the page as tall as the
  fields on show and scrolling what the body cannot hold, opened by ⏎ on a row
  or on the read-out and left by esc or by applying, the keyboard going back to
  what opened it; its fields work on a snapshot that reaches the working draft
  only when the whole item is applied. Edits no one applied are never dropped
  silently - the form keeps every key while it stands, and a press outside it
  asks what is to become of them - and never applied to an item the plan has
  changed underneath; an unsaved draft is asked about the same way before
  another document takes its place. Every applied item is one step of a
  whole-plan history the draft walks back and forward through, dropped with the
  document and kept across a save; a statement picked on the People page or
  beside the claim search, through the same file picker, lands its earnings on
  the highlighted person as one such step. Each field is entered by its kind -
  ticked, or picked from a closed set wherever the schema states one, from a
  menu or, where the set is too long for one, through the fuzzy picker, read
  from the engine rather than restated, and of the plan's own ids wherever it
  names one, so an invalid value or a misspelt reference cannot be expressed,
  and a pick the schema requires cannot be emptied. A value with parts of its
  own is rows of the same form rather than text: a trigger is picked apart into
  its kind and that kind's operands, a table the item holds is fields that reach
  into it - made with its first value and gone with its last, or, where its
  being there is itself the setting, ticked, its rows shown only while it is -
  and a list is rows that hold it between them, an order offering each place
  only what no other holds. A field is shown only while the item has a use for
  it, asked of the engine's own rules - a basis on the kinds of account that
  keep one, a window on what recurs and a single date on what happens once - and
  what is applied leaves out whatever is not. A value the file states one of
  several ways is chosen between by a pick no file holds, read from the item on
  open and written back as the keys the file does hold. What such rows do not
  yet make a value of refuses the apply and says why. A share the others
  settle - a mix's cash - is not entered but shown as what they leave. What
  remains is typed: money, a rate and how an amount grows read back exactly what
  they show - separators, a percent, a word - and a name, an id or a date is the
  plan file's own value syntax, parsed through the schema's types, so what a
  field cannot read the schema refuses in its own words. The file's spelling
  stays in the file: each field states a label and a description beside its key,
  the schema's closed sets and the plan's items are offered under words and
  display names over the values kept, one module turns a value into the phrase
  shown for it wherever it is shown, and an issue's path is read back into the
  page, item and field it names - so a form, a table, a menu and an issue say
  the same thing in the same words. Every action the shell can take is a row in
  one static command table, which the keys, the tabs' own digits, the key row,
  and the fuzzy pickers that find a command or a page all read from; a key
  particular to a page runs only while that page is on show, ahead of any
  meaning the shell gives the same key, so two pages may bind one key each and a
  page may take one of the shell's. The keyboard walks a page's panes in the
  order they are drawn, the sidebar first beside a grouped page, and a page is
  entered on its first pane. Whatever stands over the page - a menu, a picker, a
  dialog, an open item - takes the keyboard on a stack and gives it back to what
  held it, and a command chosen from one runs once it has, since what holds the
  keyboard is what a command acts on. Whether a key is a command at all is asked
  of the widget it was typed at and everything that widget sits in: what stands
  over the page keeps every key, and a form's fields and buttons keep the plain
  ones. Everything the shell says is a `tracing` event with two readers: a
  journal the shell toasts from and lists in a drawer, and, in a terminal, a log
  file. Colour is named by role, never by value: a theme is a table of roles,
  the terminal's own colours by default, and a cell no widget coloured is drawn
  in the theme's own ground. What the user sets - theme, motion - lives in one
  user config file the shell reads at launch and writes back a key at a time,
  leaving the rest of the file as the user wrote it. Each applied item
  re-validates the draft: a valid draft is re-projected at once so the views
  follow it, and an invalid one holds the last good view, reports its first
  issue, counts them beside the file name, and lists every one in a panel whose
  rows turn to the item. Saving writes the draft as canonical TOML through the
  same validation gate as every other write; scenario sessions are read-only,
  since a resolved plan cannot be written back into an overlay, and saving one
  under a new name writes the resolved plan as a plan of its own. The resolved
  chain's files are watched so edits made outside the session - on disk, or from
  another tab of the page - re-project in place, except under an unsaved draft
  or an item being edited, which is reported rather than overwritten, and so are
  each compared file's, which have no draft to protect; `mcp` serves the same
  contract to AI agents over stdio - list, read, validate, write, project,
  actions, compare, earnings-import, optimizer and market tools over plan files
  sandboxed to a served directory, plus tax-parameter lookup and an embedded
  schema reference. Writes are gated on full validation - scenarios validated
  fully resolved - and stored in canonical TOML; the schema reference's worked
  example is kept valid by the test suite.
- `retiretui_web` - the planner in a browser page, built for wasm alone and
  published to GitHub Pages under `/ratzilla` with each release, apart from the
  web app it sits beside: plurimus's WebGL canvas, the workspace kept in the
  page's own storage under `/workspace`, the browser's light or dark preference,
  and `upload` and `download` - commands only the page's table holds - through
  the browser's file dialog and a download link, an upload asking before it
  replaces a file of the same name. Quit starts the page over, on the document
  last open.
- `retiretui_wasm` - the engine and client for a JavaScript page, unpublished: a
  document opened through whatever reads the page's files - its base chain
  resolved as every surface resolves it, a scenario read-only, a file of the
  chain renamed followed without reading it again, and one that does not open
  said with the failing file's text where the failure is in it - and edited as
  its draft: a domain's items tabled, in an order a column's header gives
  through the client's sort, and read out in the form's words; one item open in
  its form, each field on show said as it is entered, with what it offers and
  what is wrong with it, and a trigger's or a list's parts held while they do
  not yet make a value; the item applied or removed as a step of the draft's
  history, undone and redone, and the draft saved as canonical text through
  whatever writes the page's files, under the same gate as every write; a
  statement's earnings recorded on a person, still the one named, as one more
  such step; the Roth Conversions tool's constraints, held beside the draft and
  outside its history and aimed at any Roth account there, and a searched ladder
  or set of claims taken into the draft as one more step or made a scenario
  beside the document's saved file, never over a file it was made from; each
  person's row of the SSA Benefits page, their benefit estimated once for each
  projection, and what is done for them as one more step; and a first plan made
  from the new-plan questions, answered a field at a time and kept as text
  between visits. Its issues are in the forms' words, each with the page, item
  and field it is about, and from its last draft without any - kept beside the
  plan it was projected from, so what is said of it names what the projection
  holds - come its projection, summary, the year a view shows held within the
  plan's years, the Ledger's years, a year's flows, income and what it paid -
  these last in the plan's own market or one a run went through, replayed once
  and kept until the draft changes - the series the Overview charts, what the
  Overview says of it - how long the money lasts and where it runs short, what
  it ends with and pays in tax, what needs attention and its milestones - and a
  year's actions both as data and said as every surface says them, in either
  dollar basis, and, against another such document, what the Compare page says
  of it: its figures or their differences from the other's, a metric year by
  year, and what it changes of the other - and the tax tables its plan asks for
  in a year, under issues too, with the client's names for the editing domains,
  what the Overview, the Ledger and Compare title what they show, and its count
  of issues, and, over a file's text, the file a scenario is resolved over and
  the scenario renamed to name another, and, over a plan's text alone so that a
  worker can run them, the gate, the conversion search into a given account and
  the claim search with the people held out of it, the market runs through
  random markets or from every historical start, and the example plans - each
  search in the words the tools table it in: the conversion and claim searches'
  every option in both dollar bases and against the plan, with what the best
  does better than the plan, and the market runs' verdict in its zone, each run
  singled out with its net worth year by year and the market it went through,
  their spread, and what they were made under with where each is edited. Values
  cross as plain objects, typed by TypeScript generated from the Rust types; the
  build fails where the two have drifted.
- `web/` - outside the cargo workspace, the web app, published at the Pages
  site's root with each release: a React page over `retiretui_wasm` for a phone
  or a desktop. Its plan files are kept in the page's own storage, under keys
  apart from the canvas page's, and followed across the browser's tabs; a
  document is opened from the new-plan questions - asked a step at a time, each
  step in the address, read back to be changed and kept in the tab until the
  plan is named and made, which opens it on the Overview - from an example plan,
  an upload, or the file last open, downloaded back out, and renamed or deleted
  from a list of them: a rename rewrites the scenarios over the file and is
  followed by the document with its unsaved draft, by the plans compared and by
  other tabs, and a delete names the scenarios it breaks. A file that does not
  open is said where it fails, the failing file offered to download or to
  replace by an upload, and its text shown beneath with the failing line marked
  where the failure is in it. The page says that clearing the browser's site
  data deletes the plans, and asks, on a button, for the browser to keep them.
  The terminal's five tabs are a bar along a phone's bottom edge and a sidebar
  on a wider screen, their addresses in the page's hash. The Overview, the
  Ledger, Compare and the tools share a year, a basis and the people whose
  claims are held, all in that address and kept by the links between them - each
  route naming what of the address a tab's link carries to it, and every link
  carrying the plans compared: the Overview leads with how long the money lasts
  and how surely, noting the year a plan first runs short, then says the year's
  actions in the dollars shown, what needs attention, each leading to its year
  in the Ledger or its item, and what each Roth owner's best ladder and the
  household's best claims do better than the plan, each leading to its tool -
  beside, on a wide page, the plan's balances by treatment charted over its
  milestones - and charts at once its net worth, its income against its taxes,
  and its net worth through random markets as percentile bands in today's
  dollars, each chart one image with the Ledger as its table and a click
  choosing the year its actions are for; the Ledger is every year in a table
  whose year column stays in view, and the chosen year's flows through each
  account and its income and tax - under the table on a wide screen, beside it
  on the widest, above it on a phone - the arrow keys stepping the year on both.
  The Plan tab's pages are the plan's editing domains: a table - rows of a name
  and one figure on a phone - with the highlighted item read out, or a domain's
  one item read out alone, and one item at a time edited in a form over the
  page, a sheet or a phone's whole screen, whose address names the item and the
  field an issue's link lands on. A Social Security statement downloaded from
  ssa.gov is recorded on a person from the People page. The Tools tab's Roth
  Conversions page ranks every fillable bracket's ladder under and against the
  plan as it stands, the highlighted one - kept in the address - year by year,
  taken into the draft after asking or written as a scenario beside the saved
  plan and compared with it at once, over the constraints its ladders are
  searched under, read out and edited in the same sheet as an item; it searches
  again whenever the plan or the constraints change. Its SSA Benefits page ranks
  every claim age for the household the same way, taken or written the same way,
  over each person's earnings record, how their benefit is set and its estimated
  benefit, with what can be done for the highlighted one, a held claim left as
  the plan states it; what the Overview found is what each tool shows, a search
  answered once for both. Its Monte Carlo and Historical pages say how the plan
  fared through random markets or from every historical start, in the colour of
  its zone, beside what the runs were made under - each a link to the field it
  is edited at - over the runs singled out, the highlighted one kept in the
  address, and the views of their spread, all shown at once; the highlighted run
  opens in the Ledger, which shows the plan replayed through its market, named
  by it and kept in the address through the year, the basis and edits, until a
  link returns it to the plan's own. Its Tax Tables page reads out the tables
  the plan's projection applies in the shared year, for the plan's filing status
  and the state it lives in, or for any status or modeled state picked in the
  address. The Compare tab sets the document beside the workspace files chosen
  from a menu: each plan's figures and its success through random markets, or
  their differences from the one chosen as the baseline, what the highlighted
  plan changes of the baseline, and one metric year by year, charted and tabled
  at once - a plan alone offered another file, or an example written beside it;
  a compared plan opened takes the document's place, the document joining the
  compared, and any other plan opened leaves nothing compared. The compared
  files are opened again whenever the workspace is written, here or in another
  tab. What is applied is a step of the draft's history, undone and redone from
  the header, which saves it or saves it under another name; edits not yet
  applied or saved are asked about before they are dropped, and a file another
  tab changes under unsaved edits is reported rather than reopened. What it says
  of a plan - a year's actions, where an issue is, the domains' names, a form's
  fields - is the client's words through the bindings, never its own. Each kind
  of search runs in a Web Worker kept loaded for it between searches - a ladder
  search one for each account it fills, so that owners' ladders run side by
  side, and a compared plan's market runs one for each plan, released once it is
  no longer compared; stopping one, or a newer search in its worker, terminates
  that worker and loads another. A palette finds any page, plan or action by
  part of its name, opened from the header on any screen; keys open it, go to
  the tabs and list every key, never while a field or anything over the page
  holds them; focus moves to a page's heading when another page is shown, unless
  the page has placed it. Each page loads the first time it is shown, and all of
  them once the app is idle; a service worker serves the page from the network
  while there is one and from its cache when there is not, and each built file
  from its cache once fetched, a new build dropping the old one's, so that after
  one visit every page works offline, and the app installs. Its colours are the
  terminal theme's roles, light or dark as the system is, each held to a
  readable contrast by a test, and every page is checked against WCAG 2.1 A and
  AA in Chromium, Firefox and WebKit by a browser suite CI runs.

Plans express timing through a closed trigger vocabulary - a fixed date, a
person's age, or a reference to a named event or income source with a whole year
offset - resolved once per projection. There is no predicate language in plan
files, for the same reason tax formulas stay in code. The engine trusts a
validated plan: validation, including the horizon-wide check that reads the tax
tables - a table for every state lived in, and the benefit formula's amounts
behind a computed Social Security benefit, claimed no earlier than the month 62
is attained - is the boundary, and every surface runs it before projecting; what
the tables limit year by year, such as what may be paid into an account, the
projection applies and reports rather than refuses.
