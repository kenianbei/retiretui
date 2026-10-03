import { beforeEach, expect, test, vi } from "vitest";

import "../public/sw.js";

interface WorkerEvent {
  request?: { method: string; mode: string; url: string };
  respondWith: (response: Promise<Response>) => void;
  waitUntil: (work: Promise<unknown>) => void;
}

const { handlers, SCOPE } = vi.hoisted(() => {
  const SCOPE = "https://example.test/retiretui/";
  const handlers = new Map<string, (event: WorkerEvent) => void>();
  vi.stubGlobal("self", {
    registration: { scope: SCOPE },
    skipWaiting: () => Promise.resolve(),
    clients: { claim: () => Promise.resolve() },
    addEventListener: (type: string, handler: (event: WorkerEvent) => void) =>
      handlers.set(type, handler),
  });
  return { handlers, SCOPE };
});

const MANIFEST = `${SCOPE}manifest.json`;

/** What the network answers with, by address; anything else is not found. */
let network = new Map<string, string>();
let isOffline = false;
let fetched: string[] = [];
let kept = new Map<string, Response>();

const urlOf = (request: string | URL | { url: string }) =>
  typeof request === "string" || request instanceof URL
    ? String(request)
    : request.url;

function answer(request: string | URL | { url: string }) {
  if (isOffline) return Promise.reject(new TypeError("offline"));
  const url = urlOf(request);
  fetched.push(url);
  const body = network.get(url);
  return Promise.resolve(
    body === undefined
      ? new Response(null, { status: 404 })
      : new Response(body),
  );
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

/** A build's files: `shared` is in every build. */
const builtOf = (build: string) =>
  [
    `assets/index-${build}.css`,
    `assets/index-${build}.js`,
    "assets/shared.wasm",
  ] as const;

/** Puts a build on the network, as its page and Vite's list say it. */
function deploy(build: string) {
  const [css, file, shared] = builtOf(build);
  network = new Map([
    [SCOPE, `<link href="./${css}" /><script src="./${file}"></script>`],
    [
      MANIFEST,
      JSON.stringify({
        "index.html": { file, css: [css], assets: [shared] },
      }),
    ],
    ...builtOf(build).map((name) => [SCOPE + name, name] as const),
  ]);
}

async function run(type: string, request?: WorkerEvent["request"]) {
  const waited: Promise<unknown>[] = [];
  let response: Promise<Response> | undefined;
  handlers.get(type)?.({
    request,
    respondWith: (answered) => {
      response = answered;
    },
    waitUntil: (work) => waited.push(work),
  });
  const answered = await response;
  await Promise.allSettled(waited);
  return answered;
}

const install = () => run("install");

/** A page asked for through the worker; its text. */
async function visit() {
  const page = await run("fetch", {
    method: "GET",
    mode: "navigate",
    url: SCOPE,
  });
  return page?.text();
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

test("installing keeps the page and every file the build lists", async () => {
  expect(keptBuilt()).toEqual(builtOf("one"));
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
  expect(keptBuilt()).toEqual(builtOf("two"));
  expect(fetched).not.toContain(`${SCOPE}assets/shared.wasm`);
  isOffline = true;
  expect(await visit()).toContain("index-two.js");
});

test("a new build whose list does not arrive leaves the old one whole", async () => {
  deploy("two");
  network.delete(MANIFEST);
  expect(await visit()).toContain("index-two.js");
  expect(keptBuilt()).toEqual(builtOf("one"));
  isOffline = true;
  expect(await visit()).toContain("index-one.js");
});

test("a new build missing a file leaves the old one whole", async () => {
  deploy("two");
  network.delete(`${SCOPE}assets/index-two.css`);
  await visit();
  expect(keptBuilt()).toEqual(builtOf("one"));
  isOffline = true;
  expect(await visit()).toContain("index-one.js");
});

test("a list from another build than the page's is not kept under it", async () => {
  const stale = network.get(MANIFEST) ?? "";
  deploy("two");
  network.set(MANIFEST, stale);
  await visit();
  expect(keptBuilt()).toEqual(builtOf("one"));
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
  expect(keptBuilt()).toEqual(builtOf("one"));
});
