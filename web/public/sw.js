// The app offline: the page from the network when there is one and from
// the cache when there is not, and each built file, whose name changes
// with its content, from the cache once it has been fetched.

const CACHE = "retiretui-app";
const SCOPE = new URL(self.registration.scope);
/** The canvas demo beside the app, which keeps to the network. */
const APART = "ratzilla/";
const BUILT = "assets/";

self.addEventListener("install", () => {
  void self.skipWaiting();
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
    event.respondWith(page(request));
  } else if (within.startsWith(BUILT)) {
    event.respondWith(built(request));
  } else {
    event.respondWith(fresh(request));
  }
});

/** The page, kept under the scope; a new one drops the files of the old. */
async function page(request) {
  const cache = await caches.open(CACHE);
  try {
    const response = await fetch(request);
    if (!response.ok) return response;
    const kept = await cache.match(SCOPE.href);
    const text = await response.clone().text();
    if (kept && (await kept.text()) !== text) await dropBuilt(cache);
    await cache.put(SCOPE.href, response.clone());
    return response;
  } catch {
    return (await cache.match(SCOPE.href)) ?? Response.error();
  }
}

/** A built file: from the cache, or fetched and kept. */
async function built(request) {
  const cache = await caches.open(CACHE);
  const kept = await cache.match(request);
  if (kept) return kept;
  const response = await fetch(request);
  if (response.ok) await cache.put(request, response.clone());
  return response;
}

/** Anything else: from the network and kept, or from the cache offline. */
async function fresh(request) {
  const cache = await caches.open(CACHE);
  try {
    const response = await fetch(request);
    if (response.ok) await cache.put(request, response.clone());
    return response;
  } catch {
    return (await cache.match(request)) ?? Response.error();
  }
}

async function dropBuilt(cache) {
  const requests = await cache.keys();
  const old = requests.filter((kept) =>
    new URL(kept.url).pathname.startsWith(SCOPE.pathname + BUILT),
  );
  await Promise.all(old.map((kept) => cache.delete(kept)));
}
