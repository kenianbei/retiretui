import { describe, expect, it } from "vitest";

import { fileAt, nameOf, pathOf, renameAt, Workspace } from "@/workspace";

/** `Storage` over a map, as a browser's `localStorage` behaves. */
class MemoryStorage implements Storage {
  private readonly items = new Map<string, string>();
  get length() {
    return this.items.size;
  }
  clear() {
    this.items.clear();
  }
  getItem(key: string) {
    return this.items.get(key) ?? null;
  }
  key(index: number) {
    return [...this.items.keys()][index] ?? null;
  }
  removeItem(key: string) {
    this.items.delete(key);
  }
  setItem(key: string, value: string) {
    this.items.set(key, value);
  }
}

describe("a workspace", () => {
  it("reads back what it wrote, and refuses a file it has not", () => {
    const workspace = new Workspace(new MemoryStorage());
    workspace.write("/plan.toml", "schema = 1");
    expect(workspace.read("/plan.toml")).toBe("schema = 1");
    expect(() => workspace.read("/gone.toml")).toThrow("no such file");
  });

  it("lists its own files in order, never the canvas demo's", () => {
    const storage = new MemoryStorage();
    storage.setItem("retiretui:file:/workspace/demo.toml", "");
    const workspace = new Workspace(storage);
    workspace.write("/b.toml", "");
    workspace.write("/a.toml", "");
    expect(workspace.list()).toEqual(["/a.toml", "/b.toml"]);
  });

  it("remembers the document last open only while it is there", () => {
    const storage = new MemoryStorage();
    const workspace = new Workspace(storage);
    workspace.write("/plan.toml", "");
    workspace.remember("/plan.toml");
    expect(workspace.lastOpen()).toBe("/plan.toml");
    storage.removeItem("retiretui-app:file:/plan.toml");
    expect(workspace.lastOpen()).toBeNull();
  });
});

it("reads a changed key as the file it holds, if any", () => {
  expect(fileAt("retiretui-app:file:/plan.toml")).toBe("/plan.toml");
  expect(fileAt("retiretui-app:last")).toBeUndefined();
  expect(fileAt("retiretui:file:/workspace/demo.toml")).toBeUndefined();
  expect(fileAt(null)).toBeNull();
});

it("keeps an uploaded file flat under the root", () => {
  expect(pathOf("plan.toml")).toBe("/plan.toml");
  expect(pathOf("C:\\plans\\plan.toml")).toBe("/plan.toml");
  expect(nameOf("/plan.toml")).toBe("plan.toml");
});

it("moves a renamed file, and tells other tabs where it went", () => {
  const storage = new MemoryStorage();
  const announced: [string, string][] = [];
  const setItem = storage.setItem.bind(storage);
  storage.setItem = (key, value) => {
    announced.push([key, value]);
    setItem(key, value);
  };
  const workspace = new Workspace(storage);
  workspace.write("/old.toml", "schema = 1");
  workspace.remember("/old.toml");
  workspace.rename("/old.toml", "/new.toml");
  expect(workspace.list()).toEqual(["/new.toml"]);
  expect(workspace.read("/new.toml")).toBe("schema = 1");
  expect(workspace.lastOpen()).toBe("/new.toml");
  const key = "retiretui-app:renamed";
  const [, value] = announced.find(([each]) => each === key) ?? [];
  expect(renameAt(key, value ?? null)).toEqual({
    from: "/old.toml",
    to: "/new.toml",
  });
  expect(storage.getItem(key)).toBeNull();
  expect(renameAt("retiretui-app:last", "/new.toml")).toBeNull();
  expect(renameAt(key, "not json")).toBeNull();
  workspace.remove("/new.toml");
  expect(workspace.list()).toEqual([]);
  expect(workspace.lastOpen()).toBeNull();
});
