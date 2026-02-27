var CACHE_NAME = 'kiri-remote-v1';
var STATIC_ASSETS = ['/', '/style.css', '/app.js', '/manifest.json'];

self.addEventListener('install', function (e) {
  e.waitUntil(
    caches.open(CACHE_NAME).then(function (cache) {
      return cache.addAll(STATIC_ASSETS);
    })
  );
  self.skipWaiting();
});

self.addEventListener('activate', function (e) {
  e.waitUntil(
    caches.keys().then(function (keys) {
      return Promise.all(
        keys
          .filter(function (k) {
            return k !== CACHE_NAME;
          })
          .map(function (k) {
            return caches.delete(k);
          })
      );
    })
  );
  self.clients.claim();
});

self.addEventListener('fetch', function (e) {
  // Only cache GET requests for static assets
  if (e.request.method !== 'GET') return;

  // Don't cache API or WebSocket requests
  var url = new URL(e.request.url);
  if (url.pathname.startsWith('/api/') || url.pathname.startsWith('/ws/')) return;

  e.respondWith(
    caches.match(e.request).then(function (cached) {
      // Network first, fallback to cache
      return fetch(e.request)
        .then(function (response) {
          var clone = response.clone();
          caches.open(CACHE_NAME).then(function (cache) {
            cache.put(e.request, clone);
          });
          return response;
        })
        .catch(function () {
          return cached;
        });
    })
  );
});
