# RetireTui plan schema (version 1)

A plan is one TOML document. All amounts are annual today's dollars at plan
start; each item escalates per its `cola`. The engine projects in nominal
dollars and reports a per-year `deflator` (nominal ÷ deflator = today's
dollars).

## Document layout

```toml
schema = 1        # required, must be 1

[plan]            # required
[household]       # required
[medicare]        # optional; enables IRMAA surcharge modeling
[[events]]        # optional, repeated
[[accounts]]      # optional, repeated
[[income]]        # optional, repeated
[[expenses]]      # optional, repeated
[[cliffs]]        # optional, repeated
[[transfers]]     # optional, repeated
[[conversions]]   # optional, repeated
[[contributions]] # optional, repeated
[[residency]]     # optional, repeated; prices state income tax
[market]          # optional; what the market tools assume
```

Unknown keys are rejected everywhere.

## [plan]

- `name` (string, optional) - display name.
- `start_year` (integer, required) - first projected calendar year.
- `horizon_age` (integer, required) - projection runs until the oldest person
  reaches this age.
- `inflation` (float, required) - annual plan inflation rate, `-0.10..=0.50`.
- `wage_growth` (float, optional) - annual growth of the national average wage
  past the last year the tax table's index carries, over which a computed Social
  Security benefit is indexed; same bounds as `inflation`. Default the tax
  table's own assumption.
- `withdrawal_order` (array of treatment classes, optional) - classes drained in
  order to cover shortfalls; default `["taxable", "deferred", "roth", "hsa"]`.
- `surplus_to` (account id, optional) - receives unspent income; defaults to the
  first cash account.

## [household]

- `filing` (required) - `"single"` or `"married-joint"`.
- `[[household.people]]` (required) - one person for `single`, two for
  `married-joint`. Each has a unique `id` (string) and `birth` (TOML date, e.g.
  `1980-06-01`), and may carry a `name` - the person as shown, where the `id` is
  only what other items reference - and `earnings` - covered (FICA) wages by
  calendar year, nominal, as the Social Security statement records them, e.g.
  `earnings = { 1995 = 4200, 2024 = 168600 }`. The `import_earnings` tool fills
  it from the statement's XML; a `social-security` income without an `amount` is
  computed from it.

## Triggers

Anywhere a field answers "when", it takes an inline table with exactly one
basis:

```toml
{ date = 2033-01-01 }             # the year containing the date
{ age = 65, owner = "jordan" }    # the year the owner reaches the age
{ event = "house-sale" }          # a named event's year
{ income = "pension", offset = 1 }  # an income's start year, shifted
```

`offset` (whole years, may be negative) is only valid with `event` or `income`.
Reference chains must not form a cycle.

## Escalation (`cola`)

On income, expenses, conversions, and contributions:

- `true` (default) - follows plan inflation.
- `false` - frozen nominal amount.
- a rate, e.g. `0.0125` - fixed annual rate of its own, compounded from plan
  start, same bounds as `inflation`.

## [[accounts]]

- `id` (string, required, unique) - referenced by transfers, conversions, and
  `surplus_to`.
- `name` (string, optional) - display name; defaults to the id.
- `kind` (required) - `"401k"`, `"403b"`, `"457b"` (penalty-exempt), `"414k"`,
  `"ira"`, `"sep-ira"`, `"simple-ira"`, `"hsa"`, `"brokerage"`, `"cash"`.
- `roth` (bool, default false) - Roth treatment; valid on `401k`, `403b`,
  `457b`, and `ira`.
- `owner` (person id, required).
- `balance` (dollars, required) - at plan start.
- `basis` (dollars, optional) - the untaxed part of the balance at plan start:
  cost basis on a brokerage, defaulting to the balance; after-tax money on a
  tax-deferred account, defaulting to nothing, which comes back untaxed pro rata
  on every withdrawal, distribution and conversion - per person across their
  IRAs, per account for a workplace plan.
- `expected_return` (float, optional) - a fixed annual return that never varies;
  absent earns nothing. Refused beside `allocation`.
- `allocation` (optional) - what the account is invested in: a mix
  `{ stocks = 0.6, bonds = 0.4 }` (shares of `stocks`, `bonds` and `cash`, an
  unstated one none, adding up to 1), rebalanced every year, or a glide path - a
  list of steps `{ from = <trigger>, stocks = .., bonds = .., cash = .. }`, each
  held from its trigger until a later step fires, the first held until any does.
  The account then earns its mix: in the ledger, each class's `mean` from
  `[market]`, blended.
- `locked_until` (trigger, optional) - until it fires, the account cannot be
  drained or transferred from.
- `drain_priority` (integer, optional) - drains before every treatment class;
  lower values first.

Tax treatment by kind: `brokerage`/`cash` are taxable, `hsa` is HSA, everything
else is deferred unless `roth = true`.

## [[income]]

- `id` (string, required, unique) - what scenarios and triggers name it by.
- `name` (string, optional) - display name; defaults to the id.
- `kind` (required) - `"salary"`, `"pension"`, `"annuity"`, `"rental"`,
  `"social-security"`, `"windfall"` (untaxed one-time), `"other"`. The kind
  decides tax treatment; social security uses the provisional-income rules.
- `owner` (person id, required).
- `amount` (dollars) - annual today's dollars; required for every kind but
  `social-security`, where it is the annual benefit at the claim age and may be
  left out: the benefit is then computed at the claim from the owner's
  `earnings` record, extended with the owner's `salary` incomes through the year
  before, so the claim must be at 62 or later.
- `start` / `end` (triggers, optional) - the receiving window; absent means plan
  start / horizon. A `social-security` income whose `start` is its owner's age
  is paid, in that year, for the months from the one the age is attained in, as
  SSA pays it; a `date` start pays the whole year.
- `on` (trigger, optional) - one-time receipt; excludes `start`/`end`.
- `cola` (see escalation). On a computed `social-security` benefit it is also
  the COLA SSA adds each year from the owner's age-62 year, before and after the
  claim.

## [[expenses]]

- `id` (string, required, unique) - what a scenario addresses the item by.
- `name` (string, optional) - what the expense is shown as; defaults to the id.
- `amount` (dollars, required), `start`/`end` or `on` (triggers), `cola` - same
  window semantics as income.

## [medicare]

Opt-in IRMAA modeling: from the year each person turns 65 the engine spends that
year's Medicare surcharge, determined by the household's MAGI from two years
earlier (the statutory lookback). Base Part B/D premiums are **not** modeled -
keep them in your own `[[expenses]]`; only the surcharge above the standard
premium is added.

- `prior_magi` (array of dollars, optional) - MAGI for up to the two calendar
  years before plan start, oldest first, seeding the lookback for the first
  projected years; missing years count below every tier.
- `part_d` (bool, default true) - include Part D surcharges beside Part B.

## [market]

What the market tools assume and how they run. Every field is optional; an
unstated one takes the built-in default shown.

- `leave_at_least` (dollars, optional) - besides never falling short, what a run
  must end with, in today's dollars, to count as a success.
- `stocks`, `bonds`, `cash` - `{ mean, volatility }`: the compound yearly return
  (the median year's) and its standard deviation. Defaults 6% ± 16%, 4% ± 6%,
  2.5% ± 1%.
- `inflation` - `{ volatility, persistence }`: the yearly shock around the
  plan's `inflation`, and the share of a year's deviation carried into the next.
  Defaults 1.5% and 0.6.
- `correlation` - `stocks_bonds`, `stocks_cash`, `stocks_inflation`,
  `bonds_cash`, `bonds_inflation`, `cash_inflation`, each -1 to 1 and together
  possible. Defaults 0.1, 0, -0.1, 0, -0.2, 0.5.
- `monte_carlo` - `draw` (`"assumptions"` or `"history"`, the historical years
  drawn at random), `trials` (1 to 100000, default 1000), `seed` (default 42),
  `block_years` (history drawn this many consecutive years at a time, default
  1).
- `historical` - `from`/`to` (the start years tried, default 1871 to 2025) and
  `wrap` (a history reaching past the data goes on from its start, default
  true).

## [[cliffs]]

A MAGI-triggered cost - the ACA premium-credit cliff's shape, with your own
numbers:

- `id` (string, required, unique) - what a scenario addresses the item by.
- `name` (string, optional) - what the cliff is shown as; defaults to the id.
- `magi_over` (dollars, required) - a year whose MAGI exceeds this incurs the
  cost that same year.
- `cost` (dollars, required) - annual cost when crossed.
- `start` / `end` (triggers, optional) - the active window; absent means plan
  start through the year before the youngest person turns 65.
- `cola` (see escalation) - scales both the threshold and the cost.

## [[events]]

- `id` (string, required, unique) - referenced by triggers as `event`.
- `name` (string, optional) - what the event is shown as; defaults to the id.
- `trigger` (trigger, required) - when the event fires.

## [[transfers]]

One-time account-to-account moves (e.g. a rollover when a linked pension
starts):

- `id` (string, required, unique) - what a scenario addresses the item by.
- `name` (string, optional) - what the transfer is shown as; defaults to the id.
- `from` / `to` (account ids, required), `on` (trigger, required).
- `amount` (dollars, optional) - today's dollars; absent moves the whole
  balance.

Tax treatment is resolved by the engine: a same-treatment move is not taxable;
deferred to taxable is a taxable distribution.

## [[conversions]]

Roth conversion schedules; the converted amount is ordinary income in the year
converted:

- `id` (string, required, unique) - what a scenario addresses the item by.
- `name` (string, optional) - what the conversion is shown as; defaults to the
  id.
- `from` (deferred account id) / `to` (Roth account id), required.
- `amount` (dollars per year, required), `start`/`end` or `on` (triggers),
  `cola`.

## [[contributions]]

Money paid into an account, one item per stream, so an account may take
several - a deferral and a match, or a second stream from a later year.

- `id` (string, required, unique) - what a scenario addresses the item by.
- `name` (string, optional) - what the contribution is shown as; defaults to the
  id.
- `to` (required) - account id.
- `by` - `"employee"` (default; out of cash flow, deducted where the kind defers
  tax), `"employer"` (landing in the account directly), or `"after-tax"` (the
  employee's after-tax money into a 401(k) or 403(b), kept as basis and filled
  last into the plan's yearly cap). Employer contributions are valid on employer
  plans and HSAs; SEP IRA takes employer only.
- The amount, stated one way: `amount` (annual today's dollars, escalated by
  `cola`); `rate` with `of` (a share of the named income's gross each year; `of`
  is an income id), optionally with `step = { add = 0.01, up_to = 0.15 }`
  raising the rate each year after the first until it reaches `up_to`;
  `max = true` (the year's employee limit for the kind and the owner's age,
  employee only); or `match = { rate = 0.5, up_to = 0.06 }` with `of` (an
  employer's match of half the employee's contributions to the account, up to 6%
  of the income's gross; employer only).
- `start`/`end` triggers, or `on` for a single year.

Employee contributions are held to the year's IRS limit (with age-50 and 60-63
catch-ups) per person across the accounts that share one - 401(k) and 403(b)
together, 457(b), SIMPLE, traditional and Roth IRA together, HSA (employer HSA
contributions count) - filled in the order the items are listed; each account's
employee plus employer total is held to the overall plan cap, the employer share
giving way. What was held is said in the year's actions, as is a Roth IRA
contribution in a year whose MAGI is over the Roth income band, and the part of
a traditional IRA contribution a workplace-plan-covered person cannot deduct -
which stays in the account as basis.

## [[residency]]

Where the household lives, which decides the state income tax inside each year's
taxes (`taxes.state`, part of `taxes.total`). Without a residency, and abroad,
only federal tax is projected. There is no end date - each residency lasts until
the next begins:

- `country` (required): an ISO 3166-1 two-letter code, such as `us` or `pt`. A
  U.S. territory is its own country (`pr`, `gu`, `vi`, `as`, `mp`).
- `state`: required when `country` is `us` and refused otherwise - a two-letter
  state code or `dc`.
- `from` (trigger): when the move happens. With any residencies at all, exactly
  one has no `from`: where the household lives when the plan starts.

Codes are read in any case and stored lowercase.

A move is taxed by the new state for the whole year its `from` resolves to; of
two moves in one year, the one listed later stands. State tax is the state's
brackets over ordinary income and capital gains alike, less its standard
deduction, with Social Security taxed only where the state taxes it. Not
modeled: retirement-income exclusions, credits, local taxes, Oregon's federal
tax subtraction (so Oregon reads somewhat high), and Washington's excise on
long-term gains.

Only states with a table validate: the states without an income tax (`ak`, `fl`,
`nv`, `nh`, `sd`, `tn`, `tx`, `wa`, `wy`) and `or`. Any other U.S. state is
refused at `residency[i].state` until a parameter file supplies `[states.xx]` -
and a parameter file overriding a year must carry the states lived in that year;
the `tax_parameters` tool shows each year's tables.

## Worked example

```toml
schema = 1

[plan]
start_year = 2026
horizon_age = 95
inflation = 0.03

[household]
filing = "single"

[[household.people]]
id = "alex"
birth = 1970-06-15

[[accounts]]
id = "ira"
kind = "ira"
owner = "alex"
balance = 500000
expected_return = 0.06

[[accounts]]
id = "roth"
kind = "ira"
roth = true
owner = "alex"
balance = 50000
expected_return = 0.06

[[accounts]]
id = "cash"
kind = "cash"
owner = "alex"
balance = 40000

[[income]]
id = "ssa"
kind = "social-security"
owner = "alex"
amount = 30000
start = { age = 67, owner = "alex" }
cola = 0.024

[[expenses]]
id = "living"
amount = 48000

[[conversions]]
id = "roth-ladder"
name = "Roth ladder"
from = "ira"
to = "roth"
amount = 20000
start = { date = 2027-01-01 }
end = { age = 66, owner = "alex" }

[[contributions]]
id = "roth-savings"
to = "roth"
amount = 7000
end = { age = 66, owner = "alex" }
```

## Scenarios

A scenario is a TOML document that overlays a base plan with deltas. It is
recognized by its `base` key and resolves wherever a plan path is accepted;
`base` may name another scenario (chains are followed bottom-up, cycles
refused).

```toml
schema = 1               # required, must be 1
base = "plan.toml"       # required; relative to this file's directory

[plan]
name = "retire-2030"     # [plan] and [household] fields replace the base's

[[income]]
id = "salary"            # items match by id; stated fields replace the
end = { date = 2030-06-01 }  # base item's, unstated fields survive

[[expenses]]
id = "travel"            # ids are unique within their section
remove = true            # delete the matched item; error if unmatched

[[expenses]]
id = "sabbatical"        # no match -> appended as a new item
amount = 12000
on = { date = 2029-06-01 }
```

Identity keys: every listed item - people, events, accounts, income, expenses,
cliffs, transfers, conversions and contributions - matches by `id`, which every
item must carry; `name` is only what an item is shown as; `residency` replaces
wholesale; `[market]` merges like `[plan]`, one level deep, so a stated `stocks`
or `monte_carlo` table replaces the base's whole. Within a matched item each
stated field replaces the base field entirely - triggers never merge internally.
`replace = true` substitutes the stated item wholesale, which is also how an
optional field is cleared. `write_plan` accepts scenario documents and validates
them fully resolved; `optimize_conversions` and `optimize_claims` emit one.
