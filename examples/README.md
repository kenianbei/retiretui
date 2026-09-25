# Example plans

Invented households to try RetireTui on before writing a plan of your own. Each
file opens with a comment saying who the household is, what the plan shows, and
commands to try, run from the repository root. Every plan starts in 2026 and
states amounts in today's dollars.

Open the whole folder in the interactive planner and pick a plan from it:

```sh
retiretui tui examples/
```

Or run a command on one plan:

```sh
retiretui project examples/starter.toml
```

## Plans

- `starter.toml` - Sam, 30, single, Texas. The smallest plan that projects: a
  401(k) with an employer match, a Roth IRA at the limit, Social Security
  computed from salary.
- `mid-career-couple.toml` - Priya and Marcus, mid-40s, Oregon. Two earners:
  deferral rates that step up, an HSA, a brokerage with a cost basis, childcare
  and college, a retirement event each.
- `early-retiree.toml` - Morgan, 45, single, Nevada. Retiring at 50 on a
  brokerage account, with a Roth conversion ladder and Social Security at 62.
- `public-pension.toml` - Dana and Chris, 50s, Nevada. A pension with its own
  cost-of-living rate, a DC account locked until the pension starts and then
  rolled over, a 457(b).
- `retired-couple.toml` - Ruth and Walter, 68, Florida. Drawing down in
  retirement: required minimum distributions, Medicare surcharges (IRMAA),
  spending that changes with age.
- `landlord.toml` - Gloria, 55, single, Tennessee. Rental income, an annuity,
  and a house sale as a one-time windfall.
- `moving-states.toml` - Kenji and Maria, late 50s, Oregon. A move to Washington
  at retirement, and state income tax following the household.
- `aca-bridge.toml` - Lee and Pat, late 50s, Wyoming. Retiring before Medicare
  under an ACA premium-credit cliff; the conversion optimizer with a MAGI
  ceiling.
- `market-mix.toml` - Taylor, 40, single, South Dakota. What the accounts hold,
  a glide path from stocks to bonds, and the Monte Carlo and historical market
  runs.
- `with-earnings.toml` - Robin, 60, single, New Hampshire. Social Security
  computed from an earnings record, and the claim-age search.

## Scenarios

A scenario names a base plan and states only what differs from it. Compare it
with its base to see what the change does.

- `mid-career-couple-retire-early.toml`, over `mid-career-couple.toml`: both
  retire at 57 instead of 62.
- `early-retiree-no-ladder.toml`, over `early-retiree.toml`: the Roth conversion
  ladder is removed.
- `retired-couple-downsize.toml`, over `retired-couple.toml`: the house is sold
  in 2027, so a windfall arrives, the housing expense ends and a condo begins.

```sh
retiretui compare examples/early-retiree.toml examples/early-retiree-no-ladder.toml
```

## Editing an example

Copy a plan before changing it. Saving from the interactive planner writes the
plan in its own canonical layout, which drops these comments. The ids and names
are the plan's own, so rename people and items freely, as long as every
reference to an id changes with it.
