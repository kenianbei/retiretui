// Checks the JavaScript edge of a `wasm-pack build --target nodejs` in pkg/.
import assert from "node:assert/strict";
import wasm from "./pkg/retiretui_wasm.js";

const { Document, examples, historical, sortPressed, validate } = wasm;

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
assert.equal(document.thisYear(first.year - 1), first.year);
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
console.log("smoke: ok");
