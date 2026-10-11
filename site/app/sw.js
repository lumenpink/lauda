const CACHE_NAME = 'lauda-pwa-v2';
const ASSETS_TO_CACHE = [
  '/app/',
  '/app/index.html',
  '/app/manifest.json',
  '/lauda-logo.svg',
  '/lauda-logo.png'
];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return cache.addAll(ASSETS_TO_CACHE);
    }).then(() => self.skipWaiting())
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) => {
      return Promise.all(
        keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key))
      );
    }).then(() => self.clients.claim())
  );
});

self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);

  // Não interceptar chamadas da API REST (/auth, /messages, etc.)
  if (!url.pathname.startsWith('/app') && !url.pathname.endsWith('.svg') && !url.pathname.endsWith('.png')) {
    return;
  }

  event.respondWith(
    caches.match(event.request).then((cached) => {
      if (cached) {
        return cached;
      }
      return fetch(event.request).then((response) => {
        // Cachear dinamicamente assets estáticos do Dioxus (.wasm, .js)
        if (response.status === 200 && (url.pathname.endsWith('.wasm') || url.pathname.endsWith('.js'))) {
          const clone = response.clone();
          caches.open(CACHE_NAME).then((cache) => cache.put(event.request, clone));
        }
        return response;
      });
    }).catch(() => {
      if (event.request.mode === 'navigate') {
        return caches.match('/app/index.html');
      }
    })
  );
});
