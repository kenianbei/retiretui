// Checks the JavaScript edge of a `wasm-pack build --target nodejs` in pkg/.
import assert from "node:assert/strict";
import wasm from "./pkg/retiretui_wasm.js";

const { Document, examples, historical, validate } = wasm;

const [starter] = examples();
const files = new Map([["/plans/starter.toml", starter.text]]);
const read = (path) => {
  if (!files.has(path)) throw new Error(`no file at ${path}`);
  return files.get(path);
};

const document = Document.open("/plans/starter.toml", read);
assert.deepEqual(document.issues(), []);
assert.equal(document.isReadOnly, false);
assert.equal(document.names().accounts["roth-ira-sam"], "Sam's Roth IRA");
const [first] = document.projection().years;
assert.equal(typeof first.year, "number");
assert.equal(document.actions(first.year).year, first.year);

const plan = document.planText();
assert.deepEqual(validate(plan), []);
assert.ok(historical(plan).start_years.length > 0);

assert.throws(() => Document.open("/plans/gone.toml", read), /no file at/);
console.log("smoke: ok");
