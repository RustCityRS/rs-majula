// What makes the site installable as an app in browsers that still ask for a
// service worker. It caches nothing and handles no requests: everything goes
// to the network as usual, so the game's files are never served stale.
self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (event) => event.waitUntil(self.clients.claim()));
