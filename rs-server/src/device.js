(() => {
    'use strict';
    const SEAL = '__DEVICE_KEY__';
    // Fingerprint, engine, platform, graphics card, user agent, seal.
    const NAMES = '__DEVICE_NAMES__'.split(',');
    const KEY = 'rs-uid';
    // 0 means storage was blocked and 1337 is the unpatched client's: neither
    // is an id.
    const usable = (value) => {
        const id = Number(value) | 0;
        return id !== 0 && id !== 1337 ? id : 0;
    };

    function keepId() {
        let stored = 0;
        try {
            stored = usable(localStorage.getItem(KEY));
        } catch {
            return;
        }
        let open;
        try {
            open = indexedDB.open('rs', 1);
        } catch {
            return;
        }
        open.onupgradeneeded = () => open.result.createObjectStore('ids');
        open.onsuccess = () => {
            const db = open.result;
            const tx = db.transaction('ids', 'readwrite');
            const ids = tx.objectStore('ids');
            const kept = ids.get(KEY);
            kept.onsuccess = () => {
                let id = stored || usable(kept.result);
                if (!id) {
                    id = usable(crypto.getRandomValues(new Int32Array(1))[0]) || 1;
                }
                if (id !== stored) {
                    try {
                        localStorage.setItem(KEY, String(id));
                    } catch {
                        // Blocked: the login says so with 0.
                    }
                }
                if (id !== usable(kept.result)) ids.put(String(id), KEY);
            };
            tx.oncomplete = () => db.close();
        };
    }

    // 64 bits of a string, as 16 hex digits (two 32-bit lanes).
    function hash(text) {
        let h1 = 0xdeadbeef;
        let h2 = 0x41c6ce57;
        for (let i = 0; i < text.length; i++) {
            const c = text.charCodeAt(i);
            h1 = Math.imul(h1 ^ c, 2654435761);
            h2 = Math.imul(h2 ^ c, 1597334677);
        }
        h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507) ^ Math.imul(h2 ^ (h2 >>> 13), 3266489909);
        h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507) ^ Math.imul(h1 ^ (h1 >>> 13), 3266489909);
        const hex = (n) => (n >>> 0).toString(16).padStart(8, '0');
        return hex(h2) + hex(h1);
    }

    // Text, shapes and blending: drawn a little differently by every GPU,
    // driver, font renderer and browser engine.
    function drawing() {
        const c = document.createElement('canvas');
        c.width = 280;
        c.height = 60;
        const g = c.getContext('2d');
        if (!g) return '';
        g.textBaseline = 'alphabetic';
        g.fillStyle = '#f60';
        g.fillRect(100, 1, 62, 20);
        g.fillStyle = '#069';
        g.font = '15px Arial';
        g.fillText('RuneScape 2004 \u{1F409} Cwm fjordbank', 2, 15);
        g.fillStyle = 'rgba(102, 204, 0, 0.7)';
        g.font = '18px "Times New Roman"';
        g.fillText('glyphs vext quiz', 4, 45);
        g.globalCompositeOperation = 'multiply';
        for (const [x, color] of [[60, '#f0f'], [90, '#0ff'], [75, '#ff0']]) {
            g.fillStyle = color;
            g.beginPath();
            g.arc(x, 30, 20, 0, Math.PI * 2);
            g.fill();
        }
        return c.toDataURL();
    }

    // The graphics card's name, as graphics() finds it.
    let card = '';

    // The graphics card and what it can do.
    function graphics() {
        const c = document.createElement('canvas');
        const gl = c.getContext('webgl') || c.getContext('experimental-webgl');
        if (!gl) return '';
        const read = (key) => {
            try {
                return String(gl.getParameter(key));
            } catch {
                return '';
            }
        };
        // Firefox names the real card in RENDERER, and warns about the
        // extension other browsers need for it.
        const info = /Firefox\//.test(navigator.userAgent) ? null : gl.getExtension('WEBGL_debug_renderer_info');
        const found = [
            read(gl.VENDOR),
            read(gl.RENDERER),
            info ? read(info.UNMASKED_VENDOR_WEBGL) : '',
            info ? read(info.UNMASKED_RENDERER_WEBGL) : '',
            read(gl.MAX_TEXTURE_SIZE),
            read(gl.MAX_RENDERBUFFER_SIZE),
            read(gl.MAX_VERTEX_UNIFORM_VECTORS),
            read(gl.MAX_FRAGMENT_UNIFORM_VECTORS),
            (gl.getSupportedExtensions() || []).join(','),
        ];
        card = (found[3] || found[1]).replace(/[^\x20-\x7e]/g, '').slice(0, 96);
        const lose = gl.getExtension('WEBGL_lose_context');
        if (lose) lose.loseContext();
        return found.join('|');
    }

    // A tone through a compressor, rendered offline: the audio stack's
    // floating point differs by browser, OS and hardware.
    function sound() {
        const Offline = window.OfflineAudioContext || window.webkitOfflineAudioContext;
        if (!Offline) return Promise.resolve('');
        const ctx = new Offline(1, 5000, 44100);
        const tone = ctx.createOscillator();
        tone.type = 'triangle';
        tone.frequency.value = 10000;
        const squash = ctx.createDynamicsCompressor();
        squash.threshold.value = -50;
        squash.knee.value = 40;
        squash.ratio.value = 12;
        squash.attack.value = 0;
        squash.release.value = 0.25;
        tone.connect(squash);
        squash.connect(ctx.destination);
        tone.start(0);
        const rendered = ctx.startRendering().then((buffer) => {
            const data = buffer.getChannelData(0);
            let sum = 0;
            for (let i = 4500; i < 5000; i++) sum += Math.abs(data[i]);
            return sum.toString();
        });
        const late = new Promise((resolve) => setTimeout(() => resolve(''), 1000));
        return Promise.race([rendered, late]).catch(() => '');
    }

    const FONTS = [
        'Arial Black', 'Bahnschrift', 'Calibri', 'Cambria', 'Candara', 'Comic Sans MS', 'Consolas',
        'Constantia', 'Corbel', 'Courier New', 'Ebrima', 'Franklin Gothic Medium', 'Gabriola', 'Gadugi',
        'Georgia', 'Helvetica Neue', 'Impact', 'Lucida Console', 'Lucida Grande', 'Menlo', 'Monaco',
        'MS Gothic', 'Palatino', 'Segoe Print', 'Segoe UI', 'SimSun', 'Sylfaen', 'Tahoma',
        'Trebuchet MS', 'Ubuntu', 'Verdana', 'DejaVu Sans', 'Liberation Sans', 'Noto Sans', 'Roboto',
        'Cantarell', 'Avenir', 'Futura', 'Optima', 'Gill Sans',
    ];

    // Which of a list of fonts are installed: text set in one that isn't
    // measures the same as its fallback.
    function fonts() {
        const g = document.createElement('canvas').getContext('2d');
        if (!g) return '';
        const width = (family) => {
            g.font = `72px ${family}`;
            return g.measureText('mmmmmmmmmmlli10OQ').width;
        };
        const fallbacks = ['monospace', 'serif', 'sans-serif'];
        const base = fallbacks.map(width);
        return FONTS.filter((font) => fallbacks.some((f, i) => width(`'${font}', ${f}`) !== base[i])).join(',');
    }

    function about() {
        let zone = '';
        try {
            zone = Intl.DateTimeFormat().resolvedOptions().timeZone || '';
        } catch {
            // Left out.
        }
        return [
            navigator.hardwareConcurrency || 0,
            navigator.deviceMemory || 0,
            navigator.maxTouchPoints || 0,
            navigator.platform || '',
            screen.colorDepth || 0,
            zone,
        ].join('|');
    }

    // Which engine draws the page, by what only that engine has: Firefox's
    // -moz- styles, Safari's (and every iOS browser's) gesture events,
    // Chromium's user agent data.
    function engine() {
        try {
            if (CSS.supports('-moz-appearance', 'none')) return 'gecko';
        } catch {
            // Not told.
        }
        if ('GestureEvent' in window) return 'webkit';
        if (navigator.userAgentData || window.chrome) return 'blink';
        return 'other';
    }

    function attempt(measure) {
        try {
            return measure();
        } catch {
            return '';
        }
    }

    // The values sent, by name, once measured.
    let check = null;

    async function measure() {
        const parts = [attempt(drawing), attempt(graphics), await sound(), attempt(fonts), attempt(about)];
        const values = [
            hash(parts.join('~')),
            attempt(engine) || 'other',
            String(navigator.platform || '').slice(0, 32),
            card,
            hash(navigator.userAgent || '').slice(0, 8),
        ];
        values.push(hash([SEAL, ...values].join('|')));
        check = values;
    }

    // The game connection goes to this world: it carries the fingerprint.
    const Native = window.WebSocket;
    window.WebSocket = new Proxy(Native, {
        construct(target, args) {
            if (check && typeof args[0] === 'string') {
                try {
                    const url = new URL(args[0], location.href);
                    if (url.host === location.host) {
                        check.forEach((value, i) => url.searchParams.set(NAMES[i], value));
                        args[0] = url.toString();
                    }
                } catch {
                    // Left as it was.
                }
            }
            return Reflect.construct(target, args);
        },
    });

    keepId();
    // Measured right away, not when the page is idle: the client keeps it
    // busy while it loads, and a login script can log in the moment the
    // title screen shows. This takes milliseconds; loading takes seconds.
    measure().catch(() => {
    });
})();
