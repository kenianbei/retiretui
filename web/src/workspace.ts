/** Every key the app keeps, apart from the canvas demo's `retiretui:` keys. */
const PREFIX = "retiretui-app:";
const FILE_KEY = `${PREFIX}file:`;
const LAST_KEY = `${PREFIX}last`;

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

  /** The document last open, where it is still there. */
  lastOpen(): string | null {
    const path = this.storage.getItem(LAST_KEY);
    return path !== null && this.has(path) ? path : null;
  }

  remember(path: string): void {
    this.storage.setItem(LAST_KEY, path);
  }
}
