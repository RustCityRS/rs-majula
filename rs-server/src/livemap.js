// The live map for the staff console. Loaded on /livemap beside the world
// map viewer (mapview.js), it draws the players the console sends it over the
// map, as dots, and tells the console which ones a click lands on.
//
// The console frames this page and talks to it with postMessage: it sends
// {type: 'players', players: [{name, x, z, level, tone}]}, {type: 'select',
// names} (ringed, and named if just one) and {type: 'focus', x, z}; the page
// answers 'ready' once it has found its way around the viewer, and {type: 'pick', names, x, y} for a click on dots (x, y in the
// page's own pixels).
//
// The viewer is minified, so the fields this reads (where it's looking) are
// found from its own code: its view is `focusX - (sWid / zoom | 0)` across,
// and the same down from focusZ.
(() => {
    'use strict';

    // The surface map's south-west corner, in tiles: fixed in MapView's
    // source (mapOriginX, and mapOriginZ + mapHeight).
    const ORIGIN_X = 2048;
    const TOP_Z = 4032;

    // How far (in page pixels) a click may be from a dot to pick it.
    const PICK_PX = 8;
    // Dots this close together (in page pixels) are drawn as one, with a count.
    const GROUP_PX = 6;

    const TONES = {
        flagged: '#ff5a52',
        staff: '#5d9cff',
        muted: '#f2b33d',
        normal: '#fefefe',
    };

    const live = {
        view: null,
        fields: null,
        players: [],
        selected: new Set(),
        hover: null,
        mouse: null,
        drawn: '',
    };

    const canvas = () => document.getElementById('canvas');
    const overlay = document.createElement('canvas');
    overlay.style.cssText = 'position:absolute;left:0;top:0;pointer-events:none;';

    function post(message) {
        if (window.parent !== window) {
            window.parent.postMessage({ source: 'rs-livemap', ...message }, '*');
        }
    }

    // The viewer's field names, from the code that works out its view:
    // `this.focusX-(this.sWid/this.zoom|0)` and the same for Z with sHei.
    function findFields(view) {
        const width = canvas().width;
        const height = canvas().height;
        const found = {};
        const pattern = /this\.([\w$]+)-\((?:0\|)?this\.([\w$]+)\/this\.([\w$]+)(?:\|0)?\)/g;
        for (let proto = Object.getPrototypeOf(view); proto && proto !== Object.prototype; proto = Object.getPrototypeOf(proto)) {
            for (const name of Object.getOwnPropertyNames(proto)) {
                const fn = Object.getOwnPropertyDescriptor(proto, name)?.value;
                if (typeof fn !== 'function') {
                    continue;
                }
                const source = Function.prototype.toString.call(fn);
                for (const [, focus, size, zoom] of source.matchAll(pattern)) {
                    if (typeof view[focus] !== 'number' || typeof view[zoom] !== 'number') {
                        continue;
                    }
                    if (view[size] === width && !found.x) {
                        found.x = focus;
                        found.zoom = zoom;
                    } else if (view[size] === height && !found.z) {
                        found.z = focus;
                    }
                }
                // Its draw starts `if (this.redraw) { this.redraw = false; ...`.
                const redraw = /^async [\w$]+\(\)\{if\(this\.([\w$]+)\)\{this\.\1=!1/.exec(source);
                if (redraw && !found.redraw) {
                    found.redraw = redraw[1];
                }
            }
        }
        return found.x && found.z ? found : null;
    }

    // Where the viewer is looking, in map pixels (a tile each), and how many
    // canvas pixels a tile takes.
    function bounds() {
        const v = live.view;
        const f = live.fields;
        const c = canvas();
        const halfW = (c.width / v[f.zoom]) | 0;
        const halfH = (c.height / v[f.zoom]) | 0;
        const left = v[f.x] - halfW;
        const top = v[f.z] - halfH;
        return { left, top, sx: c.width / (2 * halfW), sy: c.height / (2 * halfH) };
    }

    // The players' dots in page pixels, those close together as one.
    function groups(b, scale) {
        const out = [];
        const width = overlay.clientWidth;
        const height = overlay.clientHeight;
        for (const p of live.players) {
            const px = ((p.x - ORIGIN_X - b.left + 0.5) * b.sx) / scale;
            const py = ((TOP_Z - p.z - b.top - 0.5) * b.sy) / scale;
            if (px < -GROUP_PX || py < -GROUP_PX || px > width + GROUP_PX || py > height + GROUP_PX) {
                continue;
            }
            const near = out.find((g) => Math.abs(g.x - px) <= GROUP_PX && Math.abs(g.y - py) <= GROUP_PX);
            if (near) {
                near.players.push(p);
            } else {
                out.push({ x: px, y: py, players: [p] });
            }
        }
        return out;
    }

    const tone = (g) => {
        for (const t of ['flagged', 'staff', 'muted']) {
            if (g.players.some((p) => p.tone === t)) {
                return t;
            }
        }
        return 'normal';
    };

    function label(ctx, text, x, y) {
        ctx.font = '600 11px Inter, system-ui, sans-serif';
        const w = ctx.measureText(text).width + 8;
        const left = Math.min(Math.max(2, x - w / 2), overlay.clientWidth - w - 2);
        ctx.fillStyle = 'rgba(12, 13, 15, 0.85)';
        ctx.fillRect(left, y - 16, w, 15);
        ctx.fillStyle = '#e8eaed';
        ctx.fillText(text, left + 4, y - 5);
    }

    function draw() {
        requestAnimationFrame(draw);
        if (!live.fields) {
            return;
        }
        const c = canvas();
        const rect = c.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;
        const b = bounds();
        const key = [rect.width, rect.height, dpr, b.left, b.top, b.sx, live.players.length, live.version, [...live.selected].join('|'), live.mouse?.x, live.mouse?.y].join();
        if (key === live.drawn) {
            return;
        }
        live.drawn = key;
        if (overlay.width !== Math.round(rect.width * dpr) || overlay.height !== Math.round(rect.height * dpr)) {
            overlay.width = Math.round(rect.width * dpr);
            overlay.height = Math.round(rect.height * dpr);
        }
        overlay.style.width = `${rect.width}px`;
        overlay.style.height = `${rect.height}px`;
        const ctx = overlay.getContext('2d');
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.clearRect(0, 0, rect.width, rect.height);

        const scale = c.width / rect.width;
        const all = groups(b, scale);
        live.groups = all;
        const radius = Math.max(2.5, Math.min(5, (b.sx / scale) * 0.6));
        const selected = [];
        for (const g of all) {
            ctx.beginPath();
            ctx.arc(g.x, g.y, g.players.length > 1 ? radius + 1.5 : radius, 0, Math.PI * 2);
            ctx.fillStyle = TONES[tone(g)];
            ctx.fill();
            ctx.lineWidth = 1;
            ctx.strokeStyle = '#000';
            ctx.stroke();
            if (g.players.length > 1) {
                ctx.font = '700 10px Inter, system-ui, sans-serif';
                ctx.fillStyle = '#fff';
                ctx.strokeStyle = '#000';
                ctx.lineWidth = 3;
                const n = String(g.players.length);
                ctx.strokeText(n, g.x + radius + 2, g.y - radius);
                ctx.fillText(n, g.x + radius + 2, g.y - radius);
            }
            if (g.players.some((p) => live.selected.has(p.name))) {
                selected.push(g);
            }
        }
        for (const g of selected) {
            ctx.beginPath();
            ctx.arc(g.x, g.y, radius + 5, 0, Math.PI * 2);
            ctx.lineWidth = 2;
            ctx.strokeStyle = '#ffffff';
            ctx.stroke();
        }
        if (live.selected.size === 1 && selected.length === 1) {
            label(ctx, [...live.selected][0], selected[0].x, selected[0].y - radius - 6);
        }
        const hover = live.mouse && hit(live.mouse.x, live.mouse.y);
        if (hover && !(live.selected.size === 1 && selected[0] === hover)) {
            const names = hover.players.map((p) => p.name);
            const text = names.length > 3 ? `${names.slice(0, 3).join(', ')} +${names.length - 3}` : names.join(', ');
            label(ctx, text, hover.x, hover.y - radius - 6);
        }
        if (live.mouse) {
            const x = Math.floor(live.mouse.x * scale / b.sx + b.left) + ORIGIN_X;
            const z = TOP_Z - Math.floor(live.mouse.y * scale / b.sy + b.top) - 1;
            ctx.font = '500 11px "JetBrains Mono", ui-monospace, monospace';
            const text = `${x}, ${z}`;
            const w = ctx.measureText(text).width + 10;
            ctx.fillStyle = 'rgba(12, 13, 15, 0.85)';
            ctx.fillRect(rect.width - w - 6, 6, w, 18);
            ctx.fillStyle = '#e8eaed';
            ctx.fillText(text, rect.width - w + 1, 19);
        }
    }

    // The dots at page pixel (x, y), nearest first.
    function hit(x, y) {
        let best = null;
        let bestD = PICK_PX * PICK_PX;
        for (const g of live.groups || []) {
            const d = (g.x - x) ** 2 + (g.y - y) ** 2;
            if (d <= bestD) {
                best = g;
                bestD = d;
            }
        }
        return best;
    }

    function local(e) {
        const rect = canvas().getBoundingClientRect();
        return { x: e.clientX - rect.left, y: e.clientY - rect.top };
    }

    let down = null;
    window.addEventListener('pointerdown', (e) => {
        down = { ...local(e), at: performance.now() };
    }, true);
    window.addEventListener('pointerup', (e) => {
        const up = local(e);
        // A click, not the end of a drag.
        if (down && Math.abs(up.x - down.x) < 4 && Math.abs(up.y - down.y) < 4 && performance.now() - down.at < 500) {
            const g = hit(up.x, up.y);
            if (g) {
                post({ type: 'pick', names: g.players.map((p) => p.name), x: e.clientX, y: e.clientY });
            }
        }
        down = null;
    }, true);
    window.addEventListener('pointermove', (e) => {
        live.mouse = local(e);
        canvas().style.cursor = hit(live.mouse.x, live.mouse.y) ? 'pointer' : '';
    }, true);
    window.addEventListener('pointerleave', () => {
        live.mouse = null;
    });
    // The viewer's other maps (dungeons and the like) would put the dots in
    // the wrong place.
    window.addEventListener('keydown', (e) => {
        if (e.key === '[' || e.key === ']' || e.key === '\\') {
            e.stopImmediatePropagation();
            e.preventDefault();
        }
    }, true);

    function focus(x, z) {
        const v = live.view;
        const f = live.fields;
        if (!f) {
            return;
        }
        v[f.x] = x - ORIGIN_X;
        v[f.z] = TOP_Z - z;
        if (f.redraw) {
            v[f.redraw] = true;
        }
    }

    window.addEventListener('message', (e) => {
        if (e.source !== window.parent || !e.data || typeof e.data !== 'object') {
            return;
        }
        const msg = e.data;
        if (msg.type === 'players' && Array.isArray(msg.players)) {
            live.players = msg.players.filter((p) => p && typeof p.x === 'number' && typeof p.z === 'number');
            live.version = (live.version || 0) + 1;
        } else if (msg.type === 'select') {
            live.selected = new Set(Array.isArray(msg.names) ? msg.names.filter((n) => typeof n === 'string') : []);
        } else if (msg.type === 'focus' && typeof msg.x === 'number' && typeof msg.z === 'number') {
            focus(msg.x, msg.z);
        }
    });

    window.__livemap = {
        state: live,
        start(view) {
            live.view = view;
            document.getElementById('map').append(overlay);
            // The fields hold their starting values only once it has set up.
            const wait = () => {
                live.fields = findFields(view);
                if (!live.fields) {
                    setTimeout(wait, 100);
                    return;
                }
                post({ type: 'ready', found: { redraw: !!live.fields.redraw } });
            };
            wait();
            requestAnimationFrame(draw);
        },
    };
})();
