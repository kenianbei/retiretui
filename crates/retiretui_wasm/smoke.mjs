// Checks the JavaScript edge of a `wasm-pack build --target nodejs` in pkg/.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import wasm from "./pkg/retiretui_wasm.js";

const {
  Document,
  NewPlan,
  bandPercentiles,
  compactMoney,
  examples,
  historical,
  percentileLabel,
  setupSteps,
  sortPressed,
  validate,
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
assert.deepEqual(document.said(first.year).ages, [["Sam", 30]]);

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
assert.ok(historical(plan).start_years.length > 0);

assert.throws(() => Document.open("/plans/gone.toml", read), /no file at/);

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
assert.throws(() => document.importEarnings(0, statement), /was born/);
files.set("/plans/born.toml", made.text.replace(/birth = \S+/, "birth = 1975-06-14"));
const born = Document.open("/plans/born.toml", read);
assert.match(born.importEarnings(0, statement), /recorded 3 year\(s\)/);
assert.equal(born.canUndo, true);
console.log("smoke: ok");
