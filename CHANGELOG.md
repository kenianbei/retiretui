# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- An income may say that its amount is what it pays in its first year, with
  `cola_from = "start"`, and its `cola` then runs from that year rather than
  from the plan's start. A pension estimate or an annuity contract states a
  benefit that way - a sum payable from a date, with a cost-of-living rate after
  it - and until now a fixed-rate `cola` also compounded over the years before
  the income began, so a pension of 20,000 at 2% starting five years into the
  plan paid 22,082 in its first year and stayed that far ahead for life. With
  `cola_from = "start"` it pays 20,000, then 20,400. The first year follows the
  income's `start` or `on` wherever a scenario moves it, and an income already
  being paid when the plan starts escalates from the plan's start as before.
  `cola_from` is `"plan"` unless stated, is left out of a saved file while it
  is, and no plan that does not state it projects differently. A Social Security
  income is refused it: a benefit is stated in today's dollars and takes its
  cost-of-living adjustments whether or not it is claimed. Expenses, conversions
  and contributions escalate from the plan's start as they did.
  - The Income form, in the terminal and in the web app, asks it as "Grows from"
    under Growth - "Plan start" or "Its first year" - and does not ask it of a
    Social Security income.
  - The MCP server's schema reference says it under Escalation.
  - The `public-pension` example states Dana's pension this way, so it pays
    42,000 in its first year rather than 45,462.
- In the terminal planner, and the canvas page that shares its shell, what
  stands over the page no longer vanishes on the frame it closes. A dialog, a
  picker, an item's form and the file picker break up into the page beneath over
  150 ms, and the Messages drawer and the Issues panel slide out through the
  bottom of the body over 180 ms. The page is live again at once: the keyboard
  and the pointer come back on the frame the overlay closes, the dim lifts with
  them, and only what the overlay last drew lingers. Nothing leaves this way
  when `motion` is `reduced` or `off`, when an overlay is replaced by another of
  its kind, or when one closes beneath another that still stands. Menus and
  toasts are as they were.
- The terminal planner's keys can be rebound. A `[tui.keys]` table in
  `config.toml`, beside `theme` and `motion` in the user's config directory,
  names a command as the palette lists it and states every key it answers to:

  ```toml
  [tui.keys]
  save = "ctrl-w"
  quit = ["q", "alt-q"]
  sort = []
  ledger = "L"
  ```

  - An entry replaces the command's keys whole, and an empty list leaves it with
    none; the command is still run from the palette. The first key stated is the
    one shown.
  - A key is spelled as the key row shows it: `ctrl-`, `alt-` and `shift-` ahead
    of a character or a name - `esc`, `space`, `backspace`, `delete`, `insert`,
    `home`, `end`, `pageup`, `pagedown`, `f1` to `f12` - with `tab`, `enter`,
    `up`, `down`, `left` and `right` typed for the keys shown as `⇥`, `⏎` and
    the arrows. A shifted character is written as itself, `G` or `?`, and
    `shift-g` is refused saying so.
  - A key another command holds by default is taken from it, which keeps its
    others, and the shell says so once at launch: "q now runs save, not quit". A
    key two entries state goes to the command the palette lists first, and the
    other entry is left out. A command particular to a page may share a key with
    one of the shell's, as the defaults do, and runs in its place while that
    page is shown.
  - The palette always keeps a key: an entry that empties it, or states the key
    it holds, is left out.
  - An entry that names no command, or states anything that does not read as a
    key, is left out whole, the command keeps its keys, and the shell says which
    and why.
  - Everything that names a key names the one in force: the key row, the help
    picker, the tab bar, a tool's help line, the Ledger's title while it shows a
    run. A command left with no key is not hinted. A tab names its key only
    where one cell holds it, so a tab bound to `alt-1` shows none.
  - The keys a form, a list or a dialog answers to by itself - ⏎, esc and ⇥
    inside them, the arrows in a list - are not commands and are not rebound.
    The canvas page reads the same table from the page's storage, which nothing
    on the page writes.

### Changed

- **Breaking.** No plan key is removed or renamed, and a plan written for 0.3.0
  projects as it did. What breaks is the Rust API, and the workspace is 0.4.0:
  - The engine's `Income` gains `cola_from`, a `ColaAnchor`, so a struct literal
    of one written outside the engine must add it.
  - The client's `Vocabulary` gains `ColaAnchor`, so a `match` over it written
    outside the client must cover it.
- The help picker names Tab and Enter by the glyphs the key row uses, `⇥` and
  `⏎`, where it said `tab`, `shift-tab` and `enter`.

### Fixed

- One setting in `config.toml` that does not read no longer resets every other.
  A misspelt `motion`, say, lost the theme and the document to reopen with it,
  under one complaint about the whole file. Each of `theme`, `motion`,
  `document` and `keys` is now read by itself: one that does not read keeps its
  default, the rest stand, and the shell names the one at fault. A file that is
  not TOML at all is still the defaults and one complaint.
- The web app stays usable offline across a new release. Its service worker kept
  a newly released page before any of the files that page needs and dropped the
  old release's, so a visitor who opened the app as a release landed and lost
  the network before those files arrived was told "RetireTui did not start" on
  every visit until they were back online. A release's files are now kept whole,
  from the list of them the build writes beside the page, before its page is,
  and the old release's are dropped only then; a release that does not arrive
  whole leaves the old one to open offline, and is asked for again on the next
  visit online. The whole app is kept on the first visit rather than the second,
  and the page is asked of the network on every visit, so a new release shows at
  once rather than up to ten minutes late. The load that replaces 0.3.0's worker
  is still 0.3.0's to handle, and can fail this way once more.

### Security

- The web app's page carries a Content-Security-Policy, written into it when it
  is built. The page runs only the app's own script files and the one script
  written into it, named by its hash, and loads scripts, workers, images and
  data from the site alone, so a script that found its way into the page is
  refused rather than run. Styles a script writes are still let through, since
  the menus and dialogs write one to hold the page still beneath them. A browser
  too old to know the policy's word for compiling WebAssembly - Chrome before
  97, Safari before 16 - is told that the app did not start, with the browser's
  reason. The canvas page at `/ratzilla/` carries no policy.

## [0.3.0] - 2026-10-02

### Added

- A web app for phones and desktop browsers, at
  <https://kenianbei.github.io/retiretui/>, published to GitHub Pages at the
  site's root with each release: a page over the engine's JavaScript bindings,
  the unpublished `retiretui_wasm` crate, and over `retiretui_client`, the crate
  of what every interface shares, new with this release. It keeps its plans in
  the browser's own storage and never sends them anywhere. The canvas page - the
  terminal planner drawn in a browser, which 0.2.0 published at the root - is
  beside it at `/ratzilla/` and keeps its plans apart from the app's: a plan
  made in the 0.2.0 page is still there, and reaches the app by a download from
  that page and an upload to the app.
  - **Start.** The Start page opens with a sentence on what RetireTui does. A
    first plan is made from a few questions asked a step at a time - the
    household, then each person - which read every answer back to be changed
    before the plan is named, made and opened on the Overview; a blank age,
    start of work or Social Security benefit reads as what the plan will
    assume - `Blank is 65` while answered, `65, the default` read back - and the
    answers survive a reload until the plan is made. The example plans and an
    upload are offered beside them. The app reopens the plan last open, and
    downloads the open plan; a plan that does not open is said on the Start page
    by the file that failed, the line and why, with that file to download or
    replace by an upload, and its text as written beneath, the failing line
    marked.
  - **Overview.** It reads as the ledger's first page. It leads with how long
    the money lasts, the share of a thousand random markets it survives - run
    off the page's thread, and leading to the Monte Carlo tool - what it ends
    with and what it pays in tax, in today's or future dollars and in the
    terminal's words, as rows of a label and a figure on a phone; a plan that
    runs short is noted above them with the year it first does and what it
    leaves uncovered, leading to that year in the Ledger and to Expenses, and
    how long the money lasts then reads the last year it covers. Beneath is what
    to do in the year shown, in the dollars shown and said as the terminal says
    it: today's by default, stepped by button or the arrow keys, or chosen with
    a click on a chart, and leading to that year in the Ledger. Then what needs
    attention - the worst historical start the plan does not survive and how
    many fail, as the terminal says it, leading to the Historical tool on that
    start; the years the plan pays Medicare's surcharges, contributions held
    back, a benefit estimated without its earnings record, an amount too large
    to be likely - and what could do better, each row leading to its year in the
    Ledger, its item or its tool. Could do better gives each Roth owner's best
    ladder, the household's best claims and the best withdrawal order against
    the plan as it stands, each leading to its tool, a ladder's aimed at that
    owner's account; what it finds is what the tools then show without searching
    again. What to do, the milestones, what needs attention and what could do
    better sit four across on a wide page, two on a narrower one. Beneath them,
    all shown at once and two across on a wide page, the plan's balances by tax
    treatment under its net worth, its net worth, its income against its taxes,
    and its net worth through the random markets as percentile bands, each
    salary's end marked and listed under its chart, each chart read as one image
    with the Ledger as its table.
  - **Ledger.** It shows the plan year by year, the year column staying in view
    as the table scrolls sideways, and the chosen year's flows through each
    account - where each came from or went, with the year's warnings - and its
    income and what it paid, beneath the table on a wide screen and above it on
    a phone, where no page scrolls sideways but its tables. The year and the
    dollars are in the page's address, kept between the Overview, the Ledger,
    Compare and the tools and across a reload.
  - **Plan pages.** They edit the plan: each domain's items are a table sortable
    by its columns, or rows with a Sort by on a phone, beside the highlighted
    item read out in the form's words - a field left blank read as what it
    stands for, muted, such as `Earns nothing` or `6%, the default`, and the
    Market's fields under headings - and a domain there is one of is that
    read-out alone. Edit, Add and Delete work one item at a time; the form is a
    sheet, full screen on a phone, whose fields are entered by their kind -
    picks from the schema's sets and the plan's own ids, a searched list for a
    country or U.S. state, a slider beside a rate, a trigger as its kind and
    sentence - and applying stores the whole item or says why not. Every applied
    item or deletion is a step Undo and Redo walk, by button or Ctrl/Cmd+Z, and
    Save or Ctrl/Cmd+S writes the plan back through the same validation as every
    other surface; Save as… writes it under another name. A scenario opens
    read-only. Unsaved edits are asked about before a form closes, another plan
    opens, or the page is left, and a file changed in another tab under them is
    reported rather than overwritten. A plan with issues counts them in the
    header and lists each by the page, item and field it is about, each a link
    to that field in its form, while the figures keep the last ones it had
    without issues. A Social Security statement downloaded from ssa.gov is
    imported onto a person from the People page as one step of history.
  - **Roth Conversions.** The tool searches every fillable bracket's conversion
    ladder, ranks them under the plan as it stands, each saying what it ends
    with against the plan, and shows the highlighted one's conversions year by
    year beside them on a wide page, in either dollar basis, over the
    constraints it searched under, read out and edited in a sheet; it searches
    again whenever the plan or the constraints change, and takes the plan's one
    Roth account as the destination where there is only one. The highlighted
    ladder is taken into the plan after asking, as one step of history in place
    of any ladder taken before, or written as a scenario beside the saved plan,
    compared with it at once and offered to open.
  - **SSA Benefits.** The tool ranks every claim age for the household's
    computed Social Security benefits under the plan as it stands, each against
    the plan as a ladder is, beside each person's earnings record, how their
    benefit is set and the benefit estimated at 62, full retirement age and 70,
    with what can be done for the highlighted person: import a statement,
    estimate a record from their salary, compute a stated benefit from their
    record, clear the record or remove the benefit - the last two asked first -
    each one step of history, or hold their claim as the plan states it while
    the others are searched. The highlighted claims are taken into the plan
    after asking or written as a scenario, as a ladder is; the claims
    highlighted, the person and who is held are kept in the address.
  - **Withdrawal Order.** The tool ranks the orders the plan can withdraw in, by
    the search said further down, under the plan as it stands, the highlighted
    one taken into the plan after asking, as one step of history, or written as
    a scenario and compared at once. The highlighted order is kept in the
    address.
  - **Monte Carlo and Historical.** The tools run the plan through a thousand
    random markets, or from every historical start year worst first, and say how
    it fared in one sentence - `Money lasts in 87% of 1,000 markets` - in the
    colour of its zone - as the Overview's figure is coloured - beside what the
    runs were made under, each a link to the Market field it is edited at;
    beneath the runs singled out, all shown at once, are the spread of net
    worth - percentile bands in a neutral - under the highlighted run's line,
    the share still funded, what the runs end with, and net worth by year at
    each percentile (Monte Carlo only). The highlighted run opens in the Ledger,
    replayed through its market and named by it - a random market's number, or
    the year retired into - kept in the address through the year, the dollars
    and edits, with a link back to the plan's own.
  - **Compare.** It sets the plan beside the plans chosen for it from the
    workspace, the plan files the app keeps. It shows each plan's figures and
    its success through random markets - each plan searched in a worker of its
    own - or its differences from the one chosen as the baseline, what the
    highlighted plan changes of the baseline, and one of eight metrics year by
    year, charted beside its table. A plan alone is offered another from the
    workspace, or, where there is none, an example written beside it and
    compared at once. The plans compared, the baseline and the metric are kept
    in the address, the plans compared carried by every link; opening a compared
    plan puts it in the open plan's place with that plan joining the compared,
    and opening any other plan leaves nothing compared.
  - **Tax Tables.** The tool reads out the tables the plan's projection applies
    in the year shown - brackets, deductions, long-term gains, Social Security's
    thresholds and benefit formula, the contribution limits, Medicare
    surcharges, the RMD divisors and the state's income tax - in columns as many
    as the page holds - for the plan's filing status and the state it lives in
    that year, grown past the last published table at the plan's inflation, or
    for any status or modeled state picked.
  - **Plans and files.** The header names the open plan, marked with a dot while
    it has unsaved edits, and the name opens the File menu. Manage plans, in
    that menu, renames, downloads and deletes the workspace's plans, each action
    named on hover and keyboard focus: a rename rewrites the scenarios built on
    the plan and keeps an open plan's unsaved edits, a delete names the
    scenarios it breaks, and both are followed by the plans compared and by
    other tabs. The page says that clearing the browser's site data deletes the
    plans, and asks the browser to keep them on a button.
  - **Palette and keys.** A palette opened with Ctrl/Cmd+K, or from the header's
    search button on any screen, finds any page, plan or action by part of its
    name; the digits 1-5 go to the tabs and ? lists every key.
  - **Offline.** The app installs, and works offline once it has been visited:
    each page loads the first time it is shown and the rest once the app is
    idle.
  - **Accessibility.** A skip link leads to the page, focus moves to a new
    page's heading, and every page is checked against WCAG 2.1 A and AA in
    Chromium, Firefox and WebKit.
  - **Layout.** Light or dark follows the system, and navigation is a bottom bar
    on a phone - a grouped tab's pages a row of chips above the page, faded at
    an edge with more beyond it - and a sidebar on a wider screen that stays in
    view with the header as the page scrolls. Pages fill the width beside it,
    their tables and read-outs filling their columns and their sections two or
    four across where there is room; on a phone every button, tab and chip is
    touched across 44px whatever size it is drawn.
  - **Money.** It is written in full wherever a table has room for it - a
    ladder's years, a monthly benefit, the tools' options and runs, and the
    years compared - and compact only where it has not, as in the Overview's
    figures and Compare's plans; a phone's row of options shows what each ends
    with against the plan.
  - **Footer and links.** Under every page a footer says that RetireTui is a
    model and not financial advice, links to the source and to where an issue is
    reported - the report opening with the running version and the browser's
    user agent written in, and nothing of a plan - and shows that version, which
    the bindings export as `version`; from a tablet's width up the header ends
    in a GitHub mark.
  - **When it cannot start.** A page that cannot start says why in place of a
    blank screen: that it needs JavaScript, that the browser offers no
    WebAssembly, that the page's storage is blocked, or what the browser said of
    a file that did not load or run, each but the first with a link to report
    it; until it has started it says that it is loading. A page that throws
    while it is drawn is replaced by what went wrong, a button that reloads and
    the link to report it.
- The engine exposes `Scenario::set_base`, which makes an overlay name another
  base, and `project::benefit_params` and `project::state_lived_in`, the benefit
  formula's parameters and the state taxing a year as a plan's projection takes
  them.
- The terminal's Tools gain a Tax Tables page, reading out the tables the plan's
  projection applies in the year shown, for its filing status and the state it
  lives in: its sections listed beside the highlighted one's table. `[` and `]`
  step the year the Overview and the Ledger share; `f` and `t` pick another
  filing status or modeled state, the plan's own first and said as such -
  `The plan's (Single)`, `Where the plan lives (Oregon)` - each tried on as the
  cursor reaches it and put back by Esc. The web app's pickers say the same
  words, both from `retiretui_client`'s `YearTables`, which also names the
  status and state shown.
- The terminal edits a scenario. It opens editable, where it opened read-only,
  and saving writes its edits back into its own file, still naming its base and
  stating only what differs from the base as the save reads it, beside whatever
  it stated before that still holds, so a value it pinned stays pinned while the
  base agrees. The file is rewritten canonically, as a plan is. An edit no
  scenario can state - clearing what the base states outside an item, such as
  the plan's name - is refused in the form's words, naming the file to clear it
  in. Save As still writes the resolved plan as a plan of its own.
- The engine's `Scenario::over` states one plan as an overlay over another, the
  inverse of `Scenario::apply`, restating what an earlier overlay stated that
  still holds; `retiretui_client`'s `Draft::over` and `save_draft` save a
  scenario's draft through it.
- A salary may say that its job's workplace plan covers its owner - a pension
  plan, say - with `covered = true`, which the Income form asks as "Workplace
  plan" on a salary alone. While that salary pays, its owner's traditional IRA
  contribution phases out over the covered band, as it already did for anyone
  paying into a 401(k), 403(b), SIMPLE IRA, 414(k) or SEP IRA.
- On a joint return, a traditional IRA contribution by a person no workplace
  plan covers, whose spouse one does, phases out over the spouse's own band -
  242,000 to 252,000 of MAGI in 2026 - where it was deducted in full. The tax
  tables state it as `ira-deduction-phase-out-spouse`, grown past the last table
  like the other bands; a user's table that leaves it out deducts the spouse in
  full. The terminal's Tax Tables list it on a joint return.
- A Roth conversion ladder can be held to a long-term gains rate, beside the
  IRMAA tier and the MAGI cap: `optimize conversions --gains-rate 0`, the MCP
  conversion tools' `gains_rate`, and "Gains rate" in the Roth Conversions
  constraints of the terminal and the web. At 0% a year converts only while its
  realized gains stay untaxed, and at 15% only while they stay out of the 20%
  rate - a year is held where its ordinary taxable income and its realized gains
  together reach the rate's top. A year that realizes no gain is not held, so a
  ladder with no brokerage drawn on fills as it did, and a year whose gains
  already pass the top converts nothing. Left blank, nothing is held, as before.
- A year's taxes report `gains`, the long-term gains realized that year and the
  amount its gains tax is figured on, in `project --format json`, the MCP
  server's projection and the JavaScript bindings.
- A withdrawal-order search, a third optimizer beside the conversion ladder and
  the claim ages. Every order the classes the plan's `withdrawal_order` lists
  can be drained in is projected and ranked by what the household ends with -
  the least left unfunded, then the most at the end in today's dollars. A class
  the plan leaves out stays out, accounts with a `drain_priority` still drain
  first, and orders that project alike are one row, said as the plan says it
  where the plan's own is among them. A plan with fewer than two classes to
  order is refused, and says so.
  - `optimize order` prints the plan's own order and every other, best first, as
    a table or JSON, and writes the best as a scenario with `--write`.
  - The MCP server's `optimize_order` replies the same, with the best as a
    scenario document that `write_to` stores through the validated write gate.
  - The terminal gains a Withdrawal Order page among its tools, as the web app
    has: the orders ranked under the plan as it stands, the highlighted one
    taken into the draft after asking, as one step of history, or written as a
    scenario and compared at once.
  - In the terminal and the web app, the Overview's Could do better says the
    best order and what it gains where it beats the plan's own, and that the
    order as planned is best where none does, leading to the page. The line it
    shows where there is nothing to search reads "No conversion, claim or
    withdrawal order to search", and shows only where none of the three can be.
  - The engine gains `optimize_order`, `apply_order`, `order_overlay`,
    `OrderSearch` and `OrderCandidate`. The JavaScript bindings carry the search
    as `orders`, `orderWords`, and the document's `takeOrder` and
    `orderScenario`, with the types `OrderOptions`, `OrderOption`, `OrderWords`
    and `TreatmentClass`. What the Overview found of it is `order` in
    `retiretui_client`'s `searches::overview::Found`.
- The rule of 55. A 401(k), 403(b) or 414(k), Roth or not, states `separated`,
  when its owner leaves the job the plan is with: a date, an age, or the event
  the salary ends on, and a date in the past for a job already left. Left in or
  after the year the owner turns 55, the plan pays no early-withdrawal penalty
  from that year on, and is drawn on in its place in the withdrawal order rather
  than last. `public_safety = true` beside it lowers the age to 50, for a
  public-safety employee of the plan's employer. Money rolled out of the plan
  into an IRA pays the penalty again until 59 and a half. Any other kind of
  account is refused the field, and the flag is refused alone. The Accounts form
  of the terminal and the web asks "Job left" of such a plan, and "Public
  safety" once it is answered. Not modeled: 25 years of service in place of the
  public-safety age, and a 72(t) series of equal payments.
- A year that pays the early-withdrawal penalty says so among its warnings,
  `Early-withdrawal penalty paid this year: $10,000`, in the dollars shown: on
  the Overview and the Ledger of the terminal and the web, and from `actions`
  and the MCP server's actions tool.
- An early Roth withdrawal is priced as the law orders it. A Roth account keeps
  a `basis`, what was paid into it, the whole balance where left blank. Until
  its owner is 59 and a half and the account has been held five years, a
  person's Roth IRAs give up, as one, what was paid in first, untaxed; then each
  conversion the plan makes, oldest first, which pays the early-withdrawal
  penalty inside five years of it; then earnings, taxed as ordinary income and
  penalized. A Roth 401(k), 403(b) or 457(b) gives up what was paid in pro rata,
  its earnings taxed and penalized the same way, though never penalized in a
  457(b) or a plan the rule of 55 frees. A Roth holding a balance at plan start
  has been held its five years, and one that opens empty counts them from its
  first contribution, conversion or transfer in. What the employee contributes
  and what is converted add to what was paid in, and a transfer out of a Roth
  that is drawn untaxed arrives as money paid in. The Accounts form shows Basis
  on a Roth account. Not modeled: earnings made before the plan where the basis
  is left blank, conversions made in the five years before the plan, a Roth
  opened in the four years before it, the five years of a conversion into a
  workplace plan, and the first-home and disability exceptions.
- An HSA withdrawal beyond the year's medical spending is taxed. An expense
  states `medical = true`, "Medical" in the Expenses form, and the household's
  HSAs pay the year's marked total untaxed. What they pay beyond it is ordinary
  income, and until the year its owner turns 65 pays a 20% penalty besides. A
  tax table states the rate as `hsa-penalty` under `[early-withdrawal]`, and one
  that leaves it out charges 20%. The Tax Tables list it. Not modeled: medical
  bills of earlier years reimbursed later.
- The MCP schema reference says who pays the early-withdrawal penalty, until
  when, and what frees an account from it, and how a Roth and an HSA withdrawal
  are taxed.
- A state's income tax leaves retirement income untaxed where its table says so,
  and Illinois, Pennsylvania, Mississippi and Iowa are modeled states. A year's
  taxable income is kept by person and by source - wages, pensions,
  retirement-account withdrawals the federal early-withdrawal penalty does not
  reach and those it does, Roth conversions, and other income - and a state's
  table lists `exclusions`: sources left untaxed whole for each person, at any
  age or `from-age`, read as the whole calendar year the age is reached. A table
  also says whether the state taxes what is paid into a tax-deferred account in
  the year it is paid (`taxes-deferrals`) and whether its deduction is fixed in
  nominal dollars (`deduction-unindexed`), and a bracket lists the rates the law
  has already set for later years (`later`), which a year's table takes as they
  begin; a year's resolved table, as `TaxTables::params_for` and the MCP
  `tax_parameters` tool give it, lists under `later` only the rates still to
  come. Illinois leaves pensions, withdrawals at any age and conversions
  untaxed; Pennsylvania pensions and conversions, withdrawals from the year the
  owner turns 59 and a half, and taxes deferrals; Mississippi pensions,
  conversions and withdrawals that are not early, at a rate that falls each year
  to 3% in 2030 as enacted; Iowa all of them from the year a person turns 55.
  Each state's figures are taken from its statute and its revenue department's
  publications, named in the table. The Tax Tables read out what a state leaves
  untaxed and from what age. An `annuity` income is taxed by all four, as a
  commercial annuity is; an annuity a retirement plan pays is entered as a
  `pension`. The MCP schema reference lists the modeled states and the keys a
  state's table holds. Not modeled: dollar caps and income tests on an
  exclusion, which most other states need; Pennsylvania's cost recovery on a
  withdrawal it taxes, a workplace plan's own retirement age, and Tax
  Forgiveness; Mississippi's second untaxed 10,000 where both spouses have
  income; Iowa's rules for a low income.
- A state's table says what it adjusts its tax by, and the tables of Oregon,
  Illinois, Mississippi, Iowa and Washington each say theirs. Oregon and
  Washington were modeled in 0.2.0, with no adjustment. A table's
  `federal-tax-subtraction` takes federal income tax off income up to a `cap`,
  lost in equal `steps` across a band of federal AGI; its `exemption-credit`
  comes off the tax for each person and is never refunded; `deduction-until-agi`
  and the credit's `until-agi` end each above a federal AGI; `deduction-at-65`
  and the credit's `at-65` add for each person from the year they reach 65; and
  `gains-excise` taxes the household's long-term gains beyond a deduction
  through brackets of its own, inside `taxes.state`. Oregon subtracts up to
  8,750 of federal tax, the early-withdrawal penalties counted in it, stepped
  down in fifths from 125,000 of AGI (250,000 jointly) to nothing at 145,000
  (290,000); credits 263 a person, none over 100,000 (200,000); and deducts
  1,200 more at 65 (1,000 each, jointly). Illinois gives no exemption over
  250,000 of AGI (500,000 jointly), and 1,000 more a person at 65. Mississippi
  exempts 1,500 more a person at 65. Iowa credits 40 a person and 20 more at 65.
  Washington takes 7% of long-term gains over 290,000 and 9.9% of taxed gains
  over 1,000,000, one deduction to a couple; a withdrawal from a retirement
  account is never such a gain. The 290,000 is Washington's 2025 deduction
  carried by the statute's index, since its department publishes 2026's by
  October 31, 2026. The Tax Tables read each adjustment out, Washington's excise
  as a section of its own, and the MCP schema reference documents the keys. Not
  modeled: Oregon's retirement income credit, the 100-dollar bands of its tax
  table and its kicker; Illinois's exemption after 2028, where the statute as
  written drops it to 1,000; Iowa's deduction of health insurance premiums at
  65; Washington's deductions for charitable gifts and the sale of a family
  business, and its income tax over 1,000,000 a year from 2028; what a state
  adds for the blind; and the federal deductions at 65, which these states now
  lead.
- The terminal planner copies and pastes. `ctrl+c` copies the row under the
  cursor of a table, a sidebar or a page's list, a table's cells separated by
  tabs, and what is selected in a form's text field or the file picker's path,
  which select with shift and the arrows or with `ctrl+a`, cut with `ctrl+x`,
  and paste what the planner last copied with `ctrl+v`. In a terminal a copy is
  written to the system clipboard through OSC 52, which a terminal may ignore,
  and the system clipboard's text arrives by the terminal's own paste, which now
  goes into the file picker's path and the query of every fuzzy picker too,
  where it did nothing.

### Changed

- **Breaking.** No plan key is removed or renamed, and 0.2.0's example plans
  validate as written; no command or flag is removed; and a tax parameter file
  written for 0.2.0 reads as it did. What a plan written for 0.2.0 may project
  differently is said in a bullet of its own: the spouse's IRA deduction
  phase-out, early Roth and HSA withdrawals, the walk of the withdrawal order,
  Oregon and Washington, and a transfer that pays the early-withdrawal penalty.
  What breaks:
  - The terminal planner quits on `q` or `ctrl+q`. `ctrl+c` copies and no longer
    quits.
  - The engine is 0.3.0. Structs gain fields, so a struct literal of one written
    outside the engine must add them: `Income` gains `covered`;
    `ContributionLimits` gains `ira_deduction_phase_out_spouse`;
    `OptimizeOptions` gains `gains_rate`, an `Option<GainsRate>`; `Account`
    gains `separated` and `public_safety`; `Expense` gains `medical`;
    `EarlyWithdrawal` gains `hsa_penalty`; `Bracket` gains `later`, a list of
    `RateStep`, and is no longer `Copy`; and `StateParams` gains `exclusions`,
    `taxes_deferrals`, `deduction_unindexed`, `deduction_at_65`,
    `deduction_until_agi`, `federal_tax_subtraction`, `exemption_credit` and
    `gains_excise`, the last three the new `FederalTaxSubtraction`,
    `ExemptionCredit` and `GainsExcise`. `tax::state_tax` takes the year's
    income as a `StateIncome` - each person's `PersonIncome` by `Source`, gains,
    the taxable Social Security, what was deferred, the federal tax and the
    federal AGI - in place of two totals. `AccountKind::keeps_basis` answers
    true for a Roth account, and validation accepts `basis` on one.
  - `retiretui_tui` is 0.3.0. Its `actions`, `files`, `ladder`, `metric`,
    `resolve`, `store` and `table` modules are gone from its API, `resolve` now
    the engine's `plan::resolve` and the rest `retiretui_client`'s, so
    `Launch.store` is an `Arc<dyn retiretui_client::store::Store>`. Its `clap`
    feature is gone: the terminal launcher and its arguments are behind the
    `terminal` feature, off by default. It is built on plurimus 0.8.0 and
    plurimus_filepicker 0.2.0, so an app that embeds it moves to them with it.
- The canvas page moves from the Pages site's root to `/ratzilla/`.
- The terminal's Overview says the year's to-dos in the dollars shown, following
  `n` as every other pane does, and lists each of the plan's issues in Needs
  attention at its item rather than their count and the first. Its attention and
  milestone rows are `retiretui_client`'s.
- RetireTui is six published crates where it was three. What every interface
  shares over the engine is a crate of its own, `retiretui_client`, which
  `retiretui_tui` builds on: the words a plan is said in, the form model with
  its editing domains named by ids of their own, issues read back as the domain,
  item and field they are about, the load-and-validate gate and the store every
  plan file goes through, the open document and the draft with its history, the
  new-plan answers, the statement import, the searches the tools share, and the
  shapes the CLI's JSON and the MCP tools reply in. It loads the tax tables, the
  market history and the user's directories behind a `native` feature. The
  command line and the MCP server are library crates of their own,
  `retiretui_cli` and `retiretui_mcp`, the server reaching its sandboxed plan
  files through the client's store, and `retiretui` composes them with
  `retiretui_tui`'s terminal launcher into the one command it always was. The
  engine resolves scenario base chains itself, as `plan::resolve`, whose error
  names the file in the chain that failed and why. Each published crate carries
  both licence files.
- The terminal's new-plan form asks about a partner only once the household
  files jointly.
- The terminal's SSA Benefits tool names claims by person rather than by income
  id: "Take these claims? Ann at 70, Bob at 67."
- The Roth Conversions constraints read a blank as what the search assumes -
  every bracket, the plan's start, no cap - and a blank last year is said to be
  the year before the owner's RMDs begin, as the search has always taken it.
- Compact money reads one way on every interface: always with its `$`, its
  thousands separated under $10,000 (`$2,086`), and rounded rather than cut
  short (`$19,999` is `$20k`), and in billions from a billion (`$1.23B`); what
  the market runs end with is bucketed in whole millions (`$1M–$2M`). The
  terminal's Roth Conversions and SSA Benefits options gain a Vs. the plan
  column - what each ends with against the plan, blank on the plan's own row -
  and an options table too narrow for every column drops the figures at its end
  rather than cutting every header short.
- A plan that does not open is said by the file that failed, where in its text,
  and why -
  ``not opened: plan.toml, line 4, column 7: key with no value, expected `=` `` -
  where the terminal said only the path and position. The command line and the
  MCP server say what they said before.
- Needs attention no longer repeats the year a plan runs short, which the
  verdict says, and lists an amount too large to be likely at its item - a
  yearly amount of $10M or more, or a balance or basis of $1B or more:
  `Checking: a balance of $2.00B - check the amount`. The plan projects all the
  same.
- Could do better says what an option covers rather than a signed difference in
  what is left unfunded: `ends +$120k, covers $46k more spending`.
- The words every interface says are a person's rather than the schema's:
  - Counts take their noun's form - `1 person`, `3 years of earnings`,
    `2 conversions`, `1 issue` - on the command line, in the MCP server and in
    the terminal, where `(s)` had stood.
  - The claim search refuses in sentences naming people, such as that no one's
    Social Security benefit is computed from a record, and a bracket the tax
    tables lack is `the tax tables have no 99% bracket`. The terminal says a
    search's refusal as sentences, every issue rather than the first.
  - The SSA Benefits People table is headed Earnings and Benefit, its cells
    `No record`, `6 years`, `Computed`, `Stated`, `No benefit` and `Held`; a
    plan without a claim reads `No claim`.
  - The options tables are headed `Converted`, `Unfunded`, `Ends with`, `Taxes`
    and `Medicare`, Compare's words, and Historical's runs `Start years`.
  - The terminal's Monte Carlo and Historical say how the plan fared the same
    way: `Money lasts in 87% of 1,000 markets`,
    `Money lasts in 85.2% of 155 start years`, where Historical said `Survived`.
    The command line's `historical` begins `Money lasts in` too, and its
    `monte-carlo` line is as it was. The Overview's Success figure reads
    `87% of 1,000 markets`. What the runs were made under reads
    `Counts as a success` (`Never running short`,
    `Ending with at least $500,000`), `Markets drawn from`
    (`Your return assumptions`, `Historical years`) and `Fixed return` for the
    accounts no market moves.
  - A field left blank says what it stands for wherever it is read: the value it
    is made as, `6%, the default`, or what absence means, `Earns nothing`,
    `Usual order`, `The ID`, dimmed in the terminal.
  - An age reads by the person's name, `Priya at 62`, where it read
    `age 62 (priya)` by id, and a form and a read-out lead with an item's Name,
    then its ID.
  - The Market's fields are gathered under headings - Success, Stocks, Bonds,
    Cash, Inflation, Correlations, Monte Carlo, Historical - and an issue or a
    change names one with its heading, as `Stocks: Spread`.
  - Each search tool says what it is for in a line, on its page and on the
    terminal's help line; SSA Benefits says what full retirement age is, and
    Headroom and IRMAA tier say what they mean in their help. Country's help no
    longer names the terminal's Space key.
- A terminal form scrolls all the way back to a field above the screen.
- A terminal Delete whose item moved under the question says so -
  `Cash is no longer where it was in the plan` - where it did nothing without a
  word. The item is checked by its ID, so of two accounts with one name only the
  one asked about is deleted.
- The MCP schema reference says that a class left out of `withdrawal_order` is
  never drained.
- Every Roth and HSA withdrawal was untaxed at any age. A plan that draws a Roth
  account before 59 and a half beyond what was paid in, or draws an HSA and
  marks no expense `medical`, now pays tax and penalty on it. Its ledger, its
  market runs and what its searches rank best can move.
- The withdrawal order is walked for what each account gives up without a
  penalty before anything pays one. In its place in the order, a Roth IRA under
  59 and a half gives up what was paid in and the conversions past their five
  years, and an HSA under 65 the year's medical spending. The rest of each comes
  after every other account, as a penalized tax-deferred account does.
- A plan living in Oregon projects a lower state tax, by its federal tax
  subtraction and its exemption credit, and Oregon's standard deduction is the
  2,910 and 5,820 its department's withholding formulas give for 2026, where the
  table held an earlier estimate of 2,900 and 5,800. A plan living in Washington
  pays its excise in a year it realizes more than 290,000 of long-term gains.
  `taxes.state` is what the state takes, an excise with its income tax. Of the
  states 0.2.0 modeled, no other projects differently: each of the rest has no
  income tax.
- The terminal's own cursor now sits on the text field being typed in, beside
  the caret drawn there.

### Fixed

- The terminal refuses a Save As onto a scenario's own file, and says to save it
  under a name of its own. It wrote the plan the scenario resolved to over the
  overlay.
- A scheduled transfer from a tax-deferred account to a taxable one pays the
  early-withdrawal penalty on its taxable part where a withdrawal would. It was
  taxed as income and never penalized, at any age. A 457(b), an owner past 59
  and a half, and a plan freed by the rule of 55 still pay none.
- The file picker opens at the top of its listing with its cursor showing.
  Opened as the terminal planner launched, on a directory of ten entries or
  more, it was scrolled past its first rows with no row marked, and a listing
  that replaced a scrolled one could open with `../` out of view.
- `retiretui tui` refuses a plan it cannot open before it takes the terminal. It
  switched to the alternate screen and back before printing the refusal.

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
