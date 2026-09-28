/** Every key the app keeps, apart from the canvas demo's `retiretui:` keys. */
const PREFIX = "retiretui-app:";
const FILE_KEY = `${PREFIX}file:`;
const LAST_KEY = `${PREFIX}last`;
/** The last rename, announced to other tabs, which see files written and removed. */
const RENAMED_KEY = `${PREFIX}renamed`;

export interface Renamed {
  from: string;
  to: string;
}

/** The rename a changed storage `key`, now holding `value`, announces; `null` where it is none. */
export function renameAt(
  key: string | null,
  value: string | null,
): Renamed | null {
  if (key !== RENAMED_KEY || value === null) return null;
  try {
    const { from, to } = JSON.parse(value) as Partial<Renamed>;
    return typeof from === "string" && typeof to === "string"
      ? { from, to }
      : null;
  } catch {
    return null;
  }
}

/**
 * The file a changed storage `key` holds: `undefined` for a key that holds
 * none, and `null` where the storage was cleared whole.
 */
export function fileAt(key: string | null): string | null | undefined {
  if (key === null) return null;
  return key.startsWith(FILE_KEY) ? key.slice(FILE_KEY.length) : undefined;
}

/** The workspace's path for a file named `name`: flat under `/`. */
export function pathOf(name: string): string {
  return `/${name.split(/[\\/]/).pop() ?? name}`;
}

const PLAN_EXTENSION = ".toml";

/** A typed name as a plan file's: `.toml` added where it is missing. */
export function planName(typed: string): string {
  const name = typed.trim();
  return name.endsWith(PLAN_EXTENSION) ? name : `${name}${PLAN_EXTENSION}`;
}

/** A plan file's name without its extension. */
export function stemOf(name: string): string {
  return name.endsWith(PLAN_EXTENSION)
    ? name.slice(0, -PLAN_EXTENSION.length)
    : name;
}

/** The file name a scenario over `path` is offered under: its stem, then `suffix`. */
export function offeredName(path: string | null, suffix: string): string {
  return `${stemOf(nameOf(path ?? "plan"))}-${suffix}.toml`;
}

/** What a path is shown as. */
export function nameOf(path: string): string {
  return path.replace(/^\//, "");
}

/** The plan files a page keeps, in its origin's storage. */
export class Workspace {
  constructor(private readonly storage: Storage) {}

  /** `path`'s text; throws where there is no such file, as a document's read must. */
  read(path: string): string {
    const text = this.storage.getItem(FILE_KEY + path);
    if (text === null) throw new Error("no such file");
    return text;
  }

  has(path: string): boolean {
    return this.storage.getItem(FILE_KEY + path) !== null;
  }

  write(path: string, text: string): void {
    this.storage.setItem(FILE_KEY + path, text);
  }

  /** Every file's path, in order. */
  list(): string[] {
    const keys = Array.from({ length: this.storage.length }, (_, at) =>
      this.storage.key(at),
    );
    return keys
      .map((key) => fileAt(key))
      .filter((path) => typeof path === "string")
      .sort();
  }

  /** Deletes `path`, and forgets it as the document last open. */
  remove(path: string): void {
    this.storage.removeItem(FILE_KEY + path);
    if (this.storage.getItem(LAST_KEY) === path) {
      this.storage.removeItem(LAST_KEY);
    }
  }

  /** Moves `from` to `to`, announcing it to other tabs. */
  rename(from: string, to: string): void {
    this.write(to, this.read(from));
    this.storage.setItem(RENAMED_KEY, JSON.stringify({ from, to }));
    if (this.storage.getItem(LAST_KEY) === from) this.remember(to);
    this.remove(from);
  }

  /** The document last open, where it is still there. */
  lastOpen(): string | null {
    const path = this.storage.getItem(LAST_KEY);
    return path !== null && this.has(path) ? path : null;
  }

  remember(path: string): void {
    this.storage.setItem(LAST_KEY, path);
  }
}
