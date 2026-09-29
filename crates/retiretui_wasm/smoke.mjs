// Checks the JavaScript edge of a `wasm-pack build --target nodejs` in pkg/.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import wasm from "./pkg/retiretui_wasm.js";

const {
  Document,
  NewPlan,
  bandPercentiles,
  baseOf,
  claimWords,
  claims,
  compactMoney,
  compareHeaders,
  compareWords,
  examples,
  historical,
  ladders,
  marketWords,
  monteCarlo,
  percentileLabel,
  rebased,
  setupSteps,
  statementPage,
  sortPressed,
  validate,
  viewWords,
  yearAmong,
} = wasm;

const [starter] = examples();
const files = new Map([["/plans/starter.toml", starter.text]]);
const read = (path) => {
  if (!files.has(path)) throw new Error(`no file at ${path}`);
  return files.get(path);
};

const document = Document.open("/plans/starter.toml", read);
assert.deepEqual(document.issues(), []);
assert.equal(document.isReadOnly, false);
const [first] = document.projection().years;
assert.equal(typeof first.year, "number");
assert.equal(document.actions(first.year).year, first.year);
assert.equal(document.yearAt(null, first.year - 1), first.year);
assert.equal(document.yearAt(first.year + 2, first.year), first.year + 2);

const ledger = document.ledger(false);
assert.equal(ledger.columns[0].header, "Year");
assert.equal(ledger.rows[0].cells.length, ledger.columns.length);
const detail = document.yearDetail(first.year, true);
assert.ok(detail.flows.length > 0);
assert.ok(detail.paid.some((line) => line.label === "Spending"));
assert.throws(() => document.yearDetail(first.year - 99, true));
const chart = document.chart(true);
assert.equal(chart.years[0].classes.length, chart.classes.length);
assert.deepEqual(bandPercentiles(), [10, 25, 50, 75, 90]);
assert.equal(compactMoney(1234567), "$1.23M");
assert.equal(percentileLabel(90), "90th percentile");
assert.deepEqual(document.said(first.year, true).ages, [["Sam", 30]]);

const accounts = document.table("accounts", sortPressed(sortPressed(null, 0), 0));
assert.ok(accounts.rows.length > 0);
assert.equal(accounts.columns.length, accounts.rows[0].cells.length);
assert.ok(document.readOut("settings", 0).length > 0);
assert.throws(() => document.table("settings", null), /not a table/);

const editor = document.edit("accounts", 0);
const name = editor.view(document).find((field) => field.key === "name");
assert.equal(name.control, "text");
editor.set("name", undefined, "Renamed");
assert.equal(editor.isDirty, true);
assert.equal(document.apply(editor), 0);
assert.equal(document.canUndo, true);
let saved = "";
document.save((text) => {
  saved = text;
});
assert.match(saved, /Renamed/);
assert.equal(document.isDirty, false);
assert.equal(document.undo(), true);
assert.throws(() => editor.set("nothing", undefined, "1"), /no field/);
assert.throws(() => document.edit("accounts", 99), /no item/);
assert.deepEqual(sortPressed(null, 1), { column: 1, is_descending: false });

const plan = document.planText();
assert.deepEqual(validate(plan), []);
const starts = historical(plan);
assert.ok(starts.runs.length > 1);
assert.equal(starts.by_year, null);
const markets = monteCarlo(plan);
assert.deepEqual(
  markets.runs.map(({ key }) => key),
  ["planned", "p90", "p75", "p50", "p25", "p10", "worst"],
);
assert.ok(marketWords().nothing_searched.length > 0);
const worst = markets.runs.at(-1).market;
const replayed = document.ledger(false, worst);
assert.equal(replayed.rows.length, ledger.rows.length);
assert.notDeepEqual(replayed.rows, ledger.rows);
assert.ok(document.yearDetail(first.year, false, starts.runs[1].market));
assert.match(document.marketSaid(worst), /^random market \d+$/);
assert.throws(() => document.ledger(false, "p10"), /no market is called p10/);

assert.throws(() => Document.open("/plans/gone.toml", read), {
  headline: "gone.toml could not be read",
  written: null,
});

const steps = setupSteps();
assert.deepEqual(
  steps.map((step) => step.slug),
  ["household", "you", "partner"],
);
const answering = new NewPlan(null, first.year);
answering.set("filing", undefined, "single");
answering.set("name", undefined, "Jordan");
const shown = answering.view().map((field) => field.key);
assert.ok(shown.includes("name") && !shown.includes("partner_name"));
const resumed = new NewPlan(answering.answers, first.year);
assert.equal(resumed.answers, answering.answers);
const made = resumed.create();
assert.equal(made.name, "Jordan");
assert.deepEqual(validate(made.text), []);

const statement = readFileSync(
  new URL("../retiretui_engine/tests/fixtures/statement.xml", import.meta.url),
  "utf8",
);
assert.throws(() => document.importEarnings(0, "Sam", statement), /was born/);
files.set("/plans/born.toml", made.text.replace(/birth = \S+/, "birth = 1975-06-14"));
const born = Document.open("/plans/born.toml", read);
assert.match(born.importEarnings(0, "Jordan", statement), /recorded 3 year\(s\)/);
assert.equal(born.canUndo, true);
assert.equal(statementPage(), "people");

const early = examples().find((each) => each.file === "early-retiree.toml");
files.set("/plans/early.toml", early.text);
const converting = Document.open("/plans/early.toml", read);
const constraints = converting.constraints();
constraints.set("bracket", undefined, "12");
converting.applyConstraints(constraints);
assert.equal(converting.canUndo, false);
const found = ladders(
  converting.planText(),
  converting.constraintsText,
  converting.destination,
);
assert.deepEqual(
  found.brackets.map((bracket) => bracket.label),
  ["12%"],
);
const [ladder] = found.brackets;
assert.match(converting.takeLadder(found.destination, ladder.steps), /took \d+ conversion/);
assert.equal(converting.canUndo, true);
assert.throws(
  () =>
    converting.ladderScenario(
      "/plans/early-ladder.toml",
      found.destination,
      ladder.steps,
    ),
  /save first/,
);
converting.save(() => {});
assert.match(
  converting.ladderScenario(
    "/plans/early-ladder.toml",
    found.destination,
    ladder.steps,
  ),
  /base = "early.toml"/,
);
assert.ok(converting.constraintsRead().some(([label]) => label === "Fill bracket"));

files.set(
  "/plans/early-ladder.toml",
  converting.ladderScenario("/plans/early-ladder.toml", found.destination, ladder.steps),
);
const laddered = Document.open("/plans/early-ladder.toml", read);
const unladdered = Document.open("/plans/early.toml", read);
const words = compareWords();
assert.equal(words.metrics[0].key, "net-worth");
const [from, to] = unladdered.years();
const view = { nominal: false, metric: "taxes", year: yearAmong(null, from, [from, to], [from, to]) };
const headers = compareHeaders("taxes", view.year);
assert.equal(headers[4], `Taxes ${view.year}`);
const rate = { kind: "rate", rate: 0.9 };
assert.equal(laddered.planFigures(view, rate).length, headers.length - 1);
assert.equal(laddered.planFiguresAgainst(view, rate, unladdered, rate)[2], "same");
assert.ok(laddered.byYearAgainst(view, unladdered).some(({ amount }) => amount !== 0));
assert.equal(unladdered.byYear(view)[0].year, from);
assert.ok(laddered.changesFrom(unladdered).some((line) => line.startsWith("Conversions")));
assert.deepEqual(unladdered.changesFrom(unladdered), []);
assert.ok(!converting.constraintsText.includes(found.destination));
assert.equal(converting.rothOwners()[0].destination, found.destination);

files.set("/plans/claiming.toml", starter.text);
const claiming = Document.open("/plans/claiming.toml", read);
const [person] = claiming.people([]);
assert.equal(person.cells.length, claimWords().people_columns.length);
assert.ok(person.actions.some(({ action }) => action === "import"));
const options = claims(claiming.planText(), []);
const [best] = options.options;
assert.equal(best.key, best.claims.map(({ age }) => age).join("-"));
assert.match(claiming.takeClaims(best.claims, options.added), /^claimed /);
assert.throws(
  () => claiming.claimsScenario("/plans/claimed.toml", best.claims, options.added),
  /save first/,
);
claiming.save(() => {});
assert.match(
  claiming.claimsScenario("/plans/claimed.toml", best.claims, options.added),
  /base = "claiming.toml"/,
);
const removal = person.actions.find(({ action }) => action === "remove-benefit");
assert.match(removal.question, /^Remove /);
assert.match(claiming.act("remove-benefit", 0, person.name), /^removed /);
assert.throws(() => claims(claiming.planText(), [person.id]));

const scenario = files.get("/plans/claimed.toml") ?? 'schema = 1\nbase = "claiming.toml"\n';
assert.equal(baseOf("/plans/claimed.toml", scenario), "/plans/claiming.toml");
assert.equal(baseOf("/plans/starter.toml", starter.text), undefined);
assert.equal(
  baseOf("/plans/claimed.toml", rebased(scenario, "renamed.toml")),
  "/plans/renamed.toml",
);
assert.throws(() => rebased(starter.text, "renamed.toml"), /no base/);
claiming.relocate("/plans/claiming.toml", "/plans/renamed.toml");
assert.deepEqual(claiming.files(), ["/plans/renamed.toml"]);
const taxed = claiming.taxTables({ year: first.year });
assert.equal(taxed.year, first.year);
assert.ok(taxed.sections.some(({ title }) => title === "Income tax brackets"));
assert.equal(claiming.taxTables({ year: first.year, status: "single" }).status, "single");
const said = viewWords();
assert.equal(said.basis.nominal, "future dollars");
assert.deepEqual(said.flow_headers[1], ["Open", true]);
console.log("smoke: ok");
