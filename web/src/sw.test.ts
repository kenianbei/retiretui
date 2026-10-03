import { beforeEach, expect, test, vi } from "vitest";

import "../public/sw.js";

const SCOPE = "https://example.test/retiretui/";
const MANIFEST = `${SCOPE}manifest.json`;

interface WorkerEvent {
  request?: { method: string; mode: string; url: string };
  respondWith?: (response: Promise<Response>) => void;
  waitUntil: (work: Promise<unknown>) => void;
}

const handlers = vi.hoisted(() => {
  const handlers = new Map<string, (event: WorkerEvent) => void>();
  vi.stubGlobal("self", {
    registration: { scope: "https://example.test/retiretui/" },
    skipWaiting: () => Promise.resolve(),
    clients: { claim: () => Promise.resolve() },
    addEventListener: (type: string, handler: (event: WorkerEvent) => void) =>
      handlers.set(type, handler),
  });
  return handlers;
});

/** What the network answers with, by address; anything else is not found. */
let network = new Map<string, string>();
let isOffline = false;
let fetched: string[] = [];
let kept = new Map<string, Response>();

const urlOf = (request: string | URL | { url: string }) =>
  typeof request === "string" || request instanceof URL
    ? String(request)
    : request.url;

async function answer(request: string | URL | { url: string }) {
  await Promise.resolve();
  if (isOffline) throw new TypeError("offline");
  const url = urlOf(request);
  fetched.push(url);
  const body = network.get(url);
  return body === undefined
    ? new Response(null, { status: 404 })
    : new Response(body);
}

const cache = {
  match: (request: string | { url: string }) =>
    Promise.resolve(kept.get(urlOf(request))?.clone()),
  put: (request: string | { url: string }, response: Response) => {
    kept.set(urlOf(request), response);
    return Promise.resolve();
  },
  keys: () => Promise.resolve([...kept.keys()].map((url) => ({ url }))),
  delete: (request: { url: string }) =>
    Promise.resolve(kept.delete(request.url)),
  // All or nothing, as a browser's is.
  addAll: async (urls: string[]) => {
    const responses = await Promise.all(urls.map(answer));
    if (!responses.every((response) => response.ok)) {
      throw new TypeError("a file did not arrive");
    }
    urls.forEach((url, at) => kept.set(url, responses[at] as Response));
  },
};

/** A build as its page and Vite's list say it: `shared` is in every build. */
function deploy(build: string) {
  const files = [`assets/index-${build}.js`, `assets/index-${build}.css`];
  network = new Map([
    [SCOPE, `<script src="./assets/index-${build}.js"></script>`],
    [
      MANIFEST,
      JSON.stringify({
        "index.html": {
          file: files[0],
          css: [files[1]],
          assets: ["assets/shared.wasm"],
        },
      }),
    ],
    ...[...files, "assets/shared.wasm"].map(
      (file) => [SCOPE + file, file] as const,
    ),
  ]);
}

async function run(type: string, event: Partial<WorkerEvent>) {
  const waited: Promise<unknown>[] = [];
  let response: Promise<Response> | undefined;
  handlers.get(type)?.({
    ...event,
    respondWith: (answered) => {
      response = answered;
    },
    waitUntil: (work) => waited.push(work),
  });
  const answered = await response;
  await Promise.allSettled(waited);
  return answered;
}

const install = () => run("install", {});

/** A page asked for through the worker; its text. */
async function visit() {
  const request = { method: "GET", mode: "navigate", url: SCOPE };
  return (await run("fetch", { request }))?.text();
}

const keptBuilt = () =>
  [...kept.keys()]
    .filter((url) => url.includes("/assets/"))
    .map((url) => url.slice(SCOPE.length))
    .sort();

beforeEach(async () => {
  vi.stubGlobal("fetch", answer);
  vi.stubGlobal("caches", {
    open: () => Promise.resolve(cache),
    match: cache.match,
  });
  kept = new Map();
  isOffline = false;
  deploy("one");
  await install();
  fetched = [];
});

const ONE = [
  "assets/index-one.css",
  "assets/index-one.js",
  "assets/shared.wasm",
];
const TWO = [
  "assets/index-two.css",
  "assets/index-two.js",
  "assets/shared.wasm",
];

test("installing keeps the page and every file the build lists", async () => {
  expect(keptBuilt()).toEqual(ONE);
  isOffline = true;
  expect(await visit()).toContain("index-one.js");
});

test("a page that has not changed asks for no list", async () => {
  await visit();
  expect(fetched).toEqual([SCOPE]);
});

test("a new build takes the old one's place once its files are kept", async () => {
  deploy("two");
  expect(await visit()).toContain("index-two.js");
  expect(keptBuilt()).toEqual(TWO);
  expect(fetched).not.toContain(`${SCOPE}assets/shared.wasm`);
  isOffline = true;
  expect(await visit()).toContain("index-two.js");
});

test("a new build whose list does not arrive leaves the old one whole", async () => {
  deploy("two");
  network.delete(MANIFEST);
  expect(await visit()).toContain("index-two.js");
  expect(keptBuilt()).toEqual(ONE);
  isOffline = true;
  expect(await visit()).toContain("index-one.js");
});

test("a new build missing a file leaves the old one whole", async () => {
  deploy("two");
  network.delete(`${SCOPE}assets/index-two.css`);
  await visit();
  expect(keptBuilt()).toEqual(ONE);
  isOffline = true;
  expect(await visit()).toContain("index-one.js");
});

test("a list from another build than the page's is not kept under it", async () => {
  const stale = network.get(MANIFEST) ?? "";
  deploy("two");
  network.set(MANIFEST, stale);
  await visit();
  expect(keptBuilt()).toEqual(ONE);
  isOffline = true;
  expect(await visit()).toContain("index-one.js");
});

test("a worker that installs offline keeps the build on the next page fetched", async () => {
  kept = new Map();
  isOffline = true;
  await install();
  expect(keptBuilt()).toEqual([]);
  isOffline = false;
  await visit();
  expect(keptBuilt()).toEqual(ONE);
});
