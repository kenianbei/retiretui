// The app offline: the page from the network when there is one and from
// the cache when there is not; a build's files kept before its page is.

const CACHE = "retiretui-app";
const SCOPE = new URL(self.registration.scope);
/** The canvas demo beside the app, which keeps to the network. */
const APART = "ratzilla/";
const BUILT = "assets/";
/** The build's own list of its files, written beside the page. */
const MANIFEST = "manifest.json";
/** A built file as a page names it. */
const NAMED = /assets\/[\w.-]+/g;

self.addEventListener("install", (event) => {
  void self.skipWaiting();
  event.waitUntil(keepNetwork().catch(() => undefined));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener("fetch", (event) => {
  const { request } = event;
  const url = new URL(request.url);
  if (request.method !== "GET" || url.origin !== SCOPE.origin) return;
  const within = url.pathname.slice(SCOPE.pathname.length);
  if (within.startsWith(APART)) return;
  if (request.mode === "navigate") {
    event.respondWith(page(event));
  } else if (within.startsWith(BUILT)) {
    event.respondWith(built(event));
  } else {
    event.respondWith(fresh(event));
  }
});

/** The page, fetched and its build kept once it is sent; the cache's offline. */
async function page(event) {
  try {
    const response = await fetch(event.request);
    if (response.ok) event.waitUntil(keepChanged(response.clone()));
    return response;
  } catch {
    return (await caches.match(SCOPE.href)) ?? Response.error();
  }
}

/** The build on the network, kept before a page fetched through here asks. */
async function keepNetwork() {
  const response = await fetch(SCOPE.href);
  if (!response.ok) return;
  const text = await response.clone().text();
  await keepBuild(await caches.open(CACHE), response, text);
}

/** Keeps the build of a page fetched, where it is unlike the page kept. */
async function keepChanged(response) {
  const cache = await caches.open(CACHE);
  const kept = await cache.match(SCOPE.href);
  const text = await response.clone().text();
  if (kept && (await kept.text()) === text) return;
  await keepBuild(cache, response, text);
}

/**
 * Keeps a page only once every file of its build is kept, then drops any
 * other build's: a file that does not arrive leaves the cache as it was.
 */
async function keepBuild(cache, response, text) {
  const files = await listed();
  const names = text.match(NAMED) ?? [];
  // A list from another build than the page's would keep a page without its files.
  if (!names.every((name) => files.has(new URL(name, SCOPE).href))) return;
  const requests = await cache.keys();
  const kept = new Set(requests.map((request) => request.url));
  await cache.addAll([...files].filter((file) => !kept.has(file)));
  await cache.put(SCOPE.href, response);
  const old = requests.filter(
    (request) =>
      new URL(request.url).pathname.startsWith(SCOPE.pathname + BUILT) &&
      !files.has(request.url),
  );
  await Promise.all(old.map((request) => cache.delete(request)));
}

/** Every file of the build on the network, by its address. */
async function listed() {
  const response = await fetch(new URL(MANIFEST, SCOPE), { cache: "no-store" });
  if (!response.ok) throw new Error(`${MANIFEST}: ${response.status}`);
  const chunks = Object.values(await response.json());
  const files = chunks.flatMap((chunk) => [
    chunk.file,
    ...(chunk.css ?? []),
    ...(chunk.assets ?? []),
  ]);
  return new Set(files.map((file) => new URL(file, SCOPE).href));
}

/** A built file: the cache's, or fetched as anything else is. */
async function built(event) {
  return (await caches.match(event.request)) ?? fresh(event);
}

/** Anything else: fetched and kept once it is sent, or the cache's offline. */
async function fresh(event) {
  try {
    const response = await fetch(event.request);
    if (response.ok) {
      const kept = response.clone();
      event.waitUntil(
        caches.open(CACHE).then((cache) => cache.put(event.request, kept)),
      );
    }
    return response;
  } catch {
    return (await caches.match(event.request)) ?? Response.error();
  }
}
