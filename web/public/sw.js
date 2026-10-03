// The app offline: the page from the network when there is one and from
// the cache when there is not; a build's files kept before its page is.

const CACHE = "retiretui-app";
const SCOPE = new URL(self.registration.scope);
/** The canvas demo beside the app, which keeps to the network. */
const APART = "ratzilla/";
const BUILT = "assets/";
/** The build's own list of its files, written beside the page. */
const MANIFEST = "manifest.json";
/** The page in that list, by the source it is built from. */
const PAGE = "index.html";

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

/**
 * The build on the network, kept before a page fetched through here asks, and
 * whatever page is kept: a worker before this one kept a page ahead of its files.
 */
async function keepNetwork() {
  const response = await fetch(SCOPE.href, { cache: "no-store" });
  if (response.ok) await keepBuild(response);
}

/** Keeps the build of a page fetched, where it is unlike the page kept. */
async function keepChanged(response) {
  const kept = await caches.match(SCOPE.href);
  const text = await response.clone().text();
  if (kept && (await kept.text()) === text) return;
  await keepBuild(response);
}

/**
 * Keeps a page only once every file of its build is kept, then drops any
 * other build's: a file that does not arrive leaves the cache as it was.
 */
async function keepBuild(response) {
  const chunks = await listed();
  const text = await response.clone().text();
  const { file, css = [] } = chunks[PAGE];
  // A list from another build than the page's would keep a page without its files.
  if (![file, ...css].every((name) => text.includes(name))) return;
  const files = new Set(
    Object.values(chunks)
      .flatMap((chunk) => [
        chunk.file,
        ...(chunk.css ?? []),
        ...(chunk.assets ?? []),
      ])
      .map((name) => new URL(name, SCOPE).href),
  );
  const cache = await caches.open(CACHE);
  const requests = await cache.keys();
  const kept = new Set(requests.map((request) => request.url));
  await cache.addAll([...files].filter((url) => !kept.has(url)));
  await cache.put(SCOPE.href, response);
  const old = requests.filter(
    ({ url }) => url.startsWith(SCOPE.href + BUILT) && !files.has(url),
  );
  await Promise.all(old.map((request) => cache.delete(request)));
}

/** The build on the network as it lists itself: each chunk's files, by its source. */
async function listed() {
  const response = await fetch(new URL(MANIFEST, SCOPE), { cache: "no-store" });
  if (!response.ok) throw new Error(`${MANIFEST}: ${response.status}`);
  return response.json();
}

/**
 * A built file: the cache's, or fetched as anything else is. Its name is its
 * content, so one kept ahead of the page answers whatever headers ask for it.
 */
async function built(event) {
  const kept = await caches.match(event.request, { ignoreVary: true });
  return kept ?? fresh(event);
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
