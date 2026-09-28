// The app offline: the page from the network when there is one and from
// the cache when there is not; each built file, named by its content, kept.

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
    event.respondWith(page(event));
  } else if (within.startsWith(BUILT)) {
    event.respondWith(built(event));
  } else {
    event.respondWith(fresh(event));
  }
});

/** The page, fetched and kept under the scope once it is sent; the cache's offline. */
async function page(event) {
  try {
    const response = await fetch(event.request);
    if (response.ok) event.waitUntil(keepPage(response.clone()));
    return response;
  } catch {
    return (await caches.match(SCOPE.href)) ?? Response.error();
  }
}

/** Keeps a page fetched; one unlike the page kept drops the old build's files. */
async function keepPage(response) {
  const cache = await caches.open(CACHE);
  const kept = await cache.match(SCOPE.href);
  const text = await response.clone().text();
  if (kept && (await kept.text()) !== text) await dropBuilt(cache);
  await cache.put(SCOPE.href, response);
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

async function dropBuilt(cache) {
  const requests = await cache.keys();
  const old = requests.filter((kept) =>
    new URL(kept.url).pathname.startsWith(SCOPE.pathname + BUILT),
  );
  await Promise.all(old.map((kept) => cache.delete(kept)));
}
