// RS Admin: the staff console's frontend. Plain modules, no build step, and
// no innerHTML anywhere: every node is built with h(), so nothing a world or
// player sends can become markup.

'use strict';

// ---------------------------------------------------------------------------
// DOM helpers

function h(tag, props, ...children) {
    const el = document.createElement(tag);
    for (const [key, value] of Object.entries(props || {})) {
        if (value === null || value === undefined || value === false) continue;
        if (key === 'class') el.className = value;
        else if (key === 'on') for (const [event, fn] of Object.entries(value)) el.addEventListener(event, fn);
        else if (key === 'style') for (const [prop, v] of Object.entries(value)) el.style.setProperty(prop, v);
        else if (key === 'dataset') Object.assign(el.dataset, value);
        else if (key === 'value') el.value = value;
        else if (key === 'checked') el.checked = value;
        else el.setAttribute(key, value === true ? '' : value);
    }
    append(el, children);
    return el;
}

function append(el, ...children) {
    for (const child of children.flat(Infinity)) {
        if (child === null || child === undefined || child === false) continue;
        el.append(child instanceof Node ? child : String(child));
    }
    return el;
}

function replace(el, ...children) {
    el.replaceChildren();
    return append(el, children);
}

const SVG = 'http://www.w3.org/2000/svg';

const ICONS = {
    home: ['M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z'],
    globe: ['M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z', 'M3 12h18', 'M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9c-2.5-2.6-3.8-5.6-3.8-9S9.5 5.6 12 3z'],
    users: ['M16 20v-1.5a3.5 3.5 0 0 0-3.5-3.5h-5A3.5 3.5 0 0 0 4 18.5V20', 'M10 11.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z', 'M20 20v-1.5a3.5 3.5 0 0 0-2.5-3.35', 'M15.5 4.65a3.5 3.5 0 0 1 0 6.7'],
    list: ['M8 6h13', 'M8 12h13', 'M8 18h13', 'M3.5 6h.01', 'M3.5 12h.01', 'M3.5 18h.01'],
    user: ['M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8z', 'M5 20a7 7 0 0 1 14 0'],
    power: ['M12 3v9', 'M6.3 6.3a8 8 0 1 0 11.4 0'],
    x: ['M6 6l12 12', 'M18 6 6 18'],
    check: ['M5 12.5l4.5 4.5L19 7.5'],
    alert: ['M12 9v4', 'M12 17h.01', 'M10.3 3.9 2.4 18a2 2 0 0 0 1.7 3h15.8a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z'],
    search: ['M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14z', 'M20 20l-4-4'],
    megaphone: ['M3 11v2a1 1 0 0 0 1 1h2l5 4V6L6 10H4a1 1 0 0 0-1 1z', 'M15.5 8.5a5 5 0 0 1 0 7', 'M18.5 5.5a9 9 0 0 1 0 13'],
    refresh: ['M20 11a8 8 0 0 0-14.7-4.3L3 9', 'M3 4v5h5', 'M4 13a8 8 0 0 0 14.7 4.3L21 15', 'M21 20v-5h-5'],
    logout: ['M15 4h3a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-3', 'M10 17l-5-5 5-5', 'M5 12h11'],
    sun: ['M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8z', 'M12 2v2', 'M12 20v2', 'M4.9 4.9l1.4 1.4', 'M17.7 17.7l1.4 1.4', 'M2 12h2', 'M20 12h2', 'M4.9 19.1l1.4-1.4', 'M17.7 6.3l1.4-1.4'],
    moon: ['M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z'],
    monitor: ['M3 5h18v11H3z', 'M8 20h8', 'M12 16v4'],
    phone: ['M7 3h10v18H7z', 'M11 18h2'],
    clock: ['M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z', 'M12 7v5l3 2'],
    shield: ['M12 3l7 3v5c0 4.6-3 8.4-7 10-4-1.6-7-5.4-7-10V6z'],
    ban: ['M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z', 'M5.6 5.6l12.8 12.8'],
    mute: ['M11 5 6 9H3v6h3l5 4z', 'M22 9l-6 6', 'M16 9l6 6'],
    kick: ['M15 20v-1.5a3.5 3.5 0 0 0-3.5-3.5h-5A3.5 3.5 0 0 0 3 18.5V20', 'M9 11.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z', 'M17 8l4 4', 'M21 8l-4 4'],
    undo: ['M9 14 4 9l5-5', 'M4 9h10a6 6 0 0 1 0 12h-3'],
    lock: ['M6 11h12v9H6z', 'M8.5 11V8a3.5 3.5 0 0 1 7 0v3'],
    copy: ['M9 9h11v11H9z', 'M5 15H4V4h11v1'],
    chevron: ['M9 6l6 6-6 6'],
    activity: ['M3 12h4l3-8 4 16 3-8h4'],
    zap: ['M13 2 4 14h7l-1 8 9-12h-7z'],
    key: ['M15.5 7.5a3 3 0 1 1-6 0 3 3 0 0 1 6 0z', 'M10.3 9.7 3 17v4h4v-2h2v-2h2l1.7-1.7'],
    chat: ['M4 5h16v11H9l-5 4z'],
    more: ['M5 12h.01', 'M12 12h.01', 'M19 12h.01'],
    unlock: ['M6 11h12v9H6z', 'M8.5 11V8a3.5 3.5 0 0 1 6.8-1.2'],
    plus: ['M12 5v14', 'M5 12h14'],
    news: ['M4 5h13v14H6a2 2 0 0 1-2-2z', 'M17 9h3v8a2 2 0 0 1-2 2', 'M7.5 9h6', 'M7.5 12.5h6', 'M7.5 16h4'],
    pin: ['M12 21s-6-5.6-6-10.5a6 6 0 0 1 12 0C18 15.4 12 21 12 21z', 'M12 12.5a2 2 0 1 0 0-4 2 2 0 0 0 0 4z'],
    trash: ['M4 7h16', 'M9 7V4h6v3', 'M6 7l1 13h10l1-13'],
    edit: ['M4 20h4L19 9l-4-4L4 16z'],
    play: ['M7 4.5v15l12-7.5z'],
    pause: ['M7 5h3.5v14H7z', 'M13.5 5H17v14h-3.5z'],
    back: ['M3.5 12a8.5 8.5 0 1 0 2.5-6', 'M3.5 3v5h5'],
    forward: ['M20.5 12a8.5 8.5 0 1 1-2.5-6', 'M20.5 3v5h-5'],
    eye: ['M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z', 'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z'],
    briefcase: ['M4 8h16v11H4z', 'M9 8V5h6v3', 'M4 13h16'],
    inbox: ['M3 13h5l1.5 3h5l1.5-3h5', 'M5.5 5h13L21 13v6H3v-6z'],
    flag: ['M5 21V4', 'M5 4h12l-2 4 2 4H5'],
    map: ['M9 4 3 6v14l6-2 6 2 6-2V4l-6 2z', 'M9 4v14', 'M15 6v14'],
    target: ['M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z', 'M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8z', 'M12 12h.01'],
    radio: ['M12 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2z', 'M8.5 8.5a5 5 0 0 0 0 7', 'M15.5 8.5a5 5 0 0 1 0 7', 'M5.6 5.6a9 9 0 0 0 0 12.8', 'M18.4 5.6a9 9 0 0 1 0 12.8'],
    clip: ['M20 11.5 12 19.5a5 5 0 0 1-7-7l8.5-8.5a3.3 3.3 0 0 1 4.7 4.7L9.7 17.2a1.7 1.7 0 0 1-2.4-2.4L15 7'],
    star: ['M12 3.5l2.6 5.4 5.9.8-4.3 4.1 1 5.8-5.2-2.8-5.2 2.8 1-5.8-4.3-4.1 5.9-.8z'],
    download: ['M12 4v11', 'M7.5 10.5L12 15l4.5-4.5', 'M5 19h14'],
    bookmark: ['M7 4h10v16l-5-3.5L7 20z'],
    chart: ['M4 19h16', 'M7 16v-5', 'M12 16V7', 'M17 16v-8'],
    scissors: ['M6 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6z', 'M6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6z', 'M20 4 8.1 15.9', 'M14.5 14.5 20 20', 'M8.1 8.1 12 12'],
    link: ['M10 14a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1 1', 'M14 10a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l1-1'],
    coins: ['M4 6c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3z', 'M4 6v6c0 1.7 3.6 3 8 3s8-1.3 8-3V6', 'M4 12v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6'],
};

function icon(name, cls) {
    const svg = document.createElementNS(SVG, 'svg');
    svg.setAttribute('viewBox', '0 0 24 24');
    svg.setAttribute('class', cls ? `icon ${cls}` : 'icon');
    svg.setAttribute('aria-hidden', 'true');
    for (const d of ICONS[name] || []) {
        const path = document.createElementNS(SVG, 'path');
        path.setAttribute('d', d);
        svg.append(path);
    }
    return svg;
}

function button(label, props = {}) {
    const {icon: iconName, kind, small, ...rest} = props;
    const classes = ['btn', kind, small && 'small', !label && 'icon-only'].filter(Boolean).join(' ');
    return h('button', {type: 'button', class: classes, ...rest}, iconName && icon(iconName), label);
}

// ---------------------------------------------------------------------------
// Formatting

const plural = (n, word, words = word + 's') => `${n.toLocaleString()} ${n === 1 ? word : words}`;

function clock(secs) {
    secs = Math.max(0, Math.round(secs));
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = secs % 60;
    const pad = (n) => String(n).padStart(2, '0');
    return h ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

function span(secs) {
    secs = Math.max(0, Math.round(secs));
    if (secs < 60) return `${secs}s`;
    const d = Math.floor(secs / 86400);
    const h = Math.floor((secs % 86400) / 3600);
    const m = Math.floor((secs % 3600) / 60);
    if (d) return h ? `${d}d ${h}h` : `${d}d`;
    if (h) return m ? `${h}h ${m}m` : `${h}h`;
    return `${m}m`;
}

function ago(ms) {
    const secs = (Date.now() - ms) / 1000;
    if (secs < 45) return 'just now';
    if (secs < 3600) return `${Math.round(secs / 60)}m ago`;
    if (secs < 86400) return `${Math.round(secs / 3600)}h ago`;
    if (secs < 7 * 86400) return `${Math.round(secs / 86400)}d ago`;
    return dateOnly(ms);
}

// Every time shows in the browser's zone, or in UTC for those who chose it
// (a setting that follows them, see state.prefs).
const inUtc = () => !!state.prefs?.utc;
const zoned = (opts = {}) => ({...opts, timeZone: inUtc() ? 'UTC' : undefined});
const fullTime = (ms) => `${new Date(ms).toLocaleString([], zoned())}${inUtc() ? ' UTC' : ''}`;
const dateOnly = (ms) => new Date(ms).toLocaleDateString([], zoned());
// The calendar day `ms` falls on, as `2026-10-20`.
const dayKey = (ms) => new Date(ms).toLocaleDateString('en-CA', zoned());
const shortTime = (ms) => {
    const d = new Date(ms);
    return dayKey(ms) === dayKey(Date.now()) ? d.toLocaleTimeString([], zoned({hour: '2-digit', minute: '2-digit'}))
        : d.toLocaleString([], zoned({month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'}));
};
// The weekday (0 is Monday) and hour `ms` falls in.
const weekHour = (() => {
    const formats = {};
    const DAYS = {Mon: 0, Tue: 1, Wed: 2, Thu: 3, Fri: 4, Sat: 5, Sun: 6};
    return (ms) => {
        const key = inUtc() ? 'utc' : 'local';
        formats[key] ??= new Intl.DateTimeFormat('en-US', zoned({weekday: 'short', hour: 'numeric', hourCycle: 'h23'}));
        const parts = Object.fromEntries(formats[key].formatToParts(ms).map((p) => [p.type, p.value]));
        return [DAYS[parts.weekday], Number(parts.hour) % 24];
    };
})();

// Until `ms`, as "in 2d 4h" (or "permanent" past ten years).
function until(ms) {
    const secs = (ms - Date.now()) / 1000;
    if (secs > 9 * 365 * 86400) return 'permanent';
    return `${span(secs)} left`;
}

const megabytes = (bytes) => `${Math.round(bytes / (1024 * 1024)).toLocaleString()} MB`;
const NAME_PATTERN = '[A-Za-z0-9 _]{1,12}';
const NAME_RULE = 'Names are 1 to 12 letters, numbers, spaces or underscores.';
const validName = (name) => new RegExp(`^${NAME_PATTERN}$`).test(name);
const playerLink = (name, props = {}) => link(`/player?name=${encodeURIComponent(name)}`, {class: 'name-link', ...props}, name);

function device(ua) {
    const browser = /HeadlessChrome\//.test(ua) ? 'Headless Chrome' : /Edg\//.test(ua) ? 'Edge' : /OPR\//.test(ua) ? 'Opera' : /Firefox\//.test(ua) ? 'Firefox'
        : /Chrome\//.test(ua) ? 'Chrome' : /Safari\//.test(ua) ? 'Safari' : 'Browser';
    const os = /iPhone|iPad/.test(ua) ? 'iOS' : /Android/.test(ua) ? 'Android' : /Windows/.test(ua) ? 'Windows'
        : /Mac OS/.test(ua) ? 'macOS' : /Linux/.test(ua) ? 'Linux' : '';
    return os ? `${browser} on ${os}` : browser;
}

const initial = (name) => (name || '?').trim().charAt(0).toUpperCase();

function debounce(fn, ms) {
    let timer;
    return (...args) => {
        clearTimeout(timer);
        timer = setTimeout(() => fn(...args), ms);
    };
}

// Suggests names as a player name is typed into `input`: registered names
// starting with it and anyone online with it in their name (see
// /api/players/names). One datalist serves every input, since only one is
// typed in at a time. `onPick` runs with the name when one is chosen.
const NAME_LIST = 'player-names';
let nameSeq = 0;

function nameSuggest(input, onPick) {
    input.setAttribute('list', NAME_LIST);
    const options = () => document.getElementById(NAME_LIST)
        || document.body.appendChild(h('datalist', {id: NAME_LIST}));
    const fill = debounce(async () => {
        const q = input.value.trim();
        const mine = ++nameSeq;
        if (!validName(q)) return replace(options());
        try {
            const found = await api('GET', `/players/names?${query({q})}`);
            if (mine !== nameSeq || document.activeElement !== input) return;
            replace(options(), found.map((p) => h('option', {
                value: p.name,
                label: p.online_world ? `Online on world ${p.online_world}` : 'Offline',
            })));
        } catch {
            // Suggestions are a convenience; typing the whole name still works.
        }
    }, 150);
    input.addEventListener('input', (e) => {
        // Choosing a suggestion fills the input as one replacement.
        const chose = (!(e instanceof InputEvent) || e.inputType === 'insertReplacementText')
            && [...options().options].some((o) => o.value === input.value);
        if (chose) {
            ++nameSeq;
            replace(options());
            onPick?.(input.value);
        } else {
            fill();
        }
    });
    return input;
}

// ---------------------------------------------------------------------------
// Preferences (per browser; the page works without them)

const prefs = {
    get(key, fallback) {
        try {
            return localStorage.getItem(`rs-admin:${key}`) ?? fallback;
        } catch {
            return fallback;
        }
    },
    set(key, value) {
        try {
            localStorage.setItem(`rs-admin:${key}`, value);
        } catch { /* storage blocked */
        }
    },
};

function applyTheme(theme) {
    if (theme === 'auto') delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = theme;
}

applyTheme(prefs.get('theme', 'dark'));

// ---------------------------------------------------------------------------
// API

const state = {
    session: null,
    // The signed-in staff member's settings that follow them: pinned and
    // recent players, saved filters, time zone.
    prefs: null,
    worlds: [],
    canCommand: false,
    worldsLoaded: false,
    caseSummary: null,
    templates: null,
};

class ApiError extends Error {
    constructor(message, status) {
        super(message);
        this.status = status;
    }
}

async function api(method, path, body) {
    const headers = {};
    if (method !== 'GET') {
        headers['Content-Type'] = 'application/json';
        if (state.session?.csrf) headers['X-CSRF-Token'] = state.session.csrf;
    }
    let res;
    try {
        res = await fetch(`/api${path}`, {
            method,
            headers,
            credentials: 'same-origin',
            body: method === 'GET' ? undefined : JSON.stringify(body ?? {}),
        });
    } catch {
        throw new ApiError('Could not reach the console. Check your connection.', 0);
    }
    let data = null;
    try {
        data = await res.json();
    } catch { /* not JSON */
    }
    if (!res.ok) {
        if (res.status === 401 && state.session?.stage === 'signed_in' && !path.startsWith('/auth/')) {
            sessionEnded();
        }
        throw new ApiError(data?.error || `Request failed (${res.status})`, res.status);
    }
    return data;
}

function sessionEnded() {
    clearReplayStore();
    state.session = {stage: 'signed_out'};
    stopPolling();
    toast('Your session ended. Sign in again.', 'error');
    render();
}

// ---------------------------------------------------------------------------
// Toasts

const toasts = h('div', {class: 'toasts', role: 'status', 'aria-live': 'polite'});
document.body.append(toasts);

// `kind` is ok, error, or alert: something staff must look at, which
// stays up longest.
function toast(message, kind = 'ok') {
    const el = h('div', {class: `toast ${kind}`}, icon(kind === 'ok' ? 'check' : 'alert'), h('div', {}, message));
    toasts.append(el);
    setTimeout(() => {
        el.classList.add('leaving');
        setTimeout(() => el.remove(), 220);
    }, kind === 'alert' ? 15000 : kind === 'error' ? 6000 : 3500);
}

// ---------------------------------------------------------------------------
// Dialogs

// What's wrong with a dialog field, or null: empty when it's required, or
// failing its pattern, length or range (`data-invalid` says how).
function fieldProblem(el) {
    const label = el.getAttribute('aria-label') || el.closest('.field')?.querySelector('span')?.textContent || 'This field';
    if (el.required && !el.value.trim()) return `${label} is required.`;
    return el.checkValidity() ? null : el.dataset.invalid || `${label} isn't valid.`;
}

/** Marks and focuses the first visible field with a problem; returns it. */
function firstProblem(root) {
    for (const el of root.querySelectorAll('input, textarea')) {
        if (el.closest('.hidden')) continue;
        const problem = fieldProblem(el);
        if (problem) {
            el.setAttribute('aria-invalid', 'true');
            el.focus();
            return problem;
        }
    }
    return null;
}

function openDialog({title, subtitle, body = [], actions = [], className, onClose, width}) {
    const error = h('p', {class: 'error-text', role: 'alert'});
    const foot = h('div', {class: 'dialog-foot'});
    const dialog = h('dialog', {class: className, 'aria-label': title});
    if (width) dialog.style.setProperty('width', width);
    const close = () => {
        if (dialog.open) dialog.close();
    };
    dialog.addEventListener('close', () => {
        dialog.remove();
        onClose?.();
    });
    dialog.addEventListener('click', (e) => {
        if (e.target === dialog) close();
    });
    dialog.addEventListener('input', (e) => e.target.removeAttribute('aria-invalid'));

    const buttons = actions.map((action) => {
        const btn = button(action.label, {kind: action.kind, icon: action.icon});
        btn.addEventListener('click', async () => {
            if (!action.run) return close();
            // Nothing is sent until the form is filled out.
            error.textContent = firstProblem(dialog) ?? '';
            if (error.textContent) return;
            const all = foot.querySelectorAll('button');
            all.forEach((b) => (b.disabled = true));
            const label = btn.textContent;
            btn.replaceChildren(h('span', {class: 'spinner'}), label);
            try {
                const keepOpen = await action.run({close, error});
                if (keepOpen !== true) close();
            } catch (e) {
                error.textContent = e.message;
            } finally {
                all.forEach((b) => (b.disabled = false));
                replace(btn, action.icon && icon(action.icon), label);
                action.refresh?.(btn);
            }
        });
        action.button = btn;
        action.refresh?.(btn);
        return btn;
    });
    append(foot, buttons);

    append(
        dialog,
        h('div', {class: 'dialog-form'},
            h('div', {class: 'dialog-head'},
                h('div', {class: 'grow'}, h('h2', {}, title), subtitle && h('p', {}, subtitle)),
                button(null, {kind: 'ghost', icon: 'x', 'aria-label': 'Close', on: {click: close}})),
            h('div', {class: 'dialog-body'}, body, error),
            actions.length ? foot : null),
    );
    document.body.append(dialog);
    dialog.showModal();
    dialog.querySelector('[autofocus]')?.focus();
    return {dialog, close, error};
}

function confirmDialog({title, subtitle, body, label, kind = 'primary', run}) {
    return openDialog({
        title,
        subtitle,
        body,
        actions: [{label: 'Cancel'}, {label, kind, run}],
    });
}

// ---------------------------------------------------------------------------
// Settings that follow a staff member from browser to browser: pinned and
// recently viewed players, saved filters, and the time zone.

const NO_PREFS = {pinned: [], recent: [], filters: [], utc: false};
const prefsWatchers = new Set();

// Calls `fn` when the settings change, while `el` is on the page.
function watchPrefs(el, fn) {
    prefsWatchers.add({el, fn});
}

async function loadPrefs() {
    try {
        state.prefs = await api('GET', '/prefs');
    } catch {
        state.prefs = {...NO_PREFS};
    }
}

// Makes one change (see PrefsOp in prefs.rs); every time on the page is
// drawn again when the zone changes.
async function changePrefs(op) {
    const before = state.prefs?.utc;
    state.prefs = await api('POST', '/prefs', op);
    for (const w of prefsWatchers) {
        if (w.el.isConnected) w.fn();
        else prefsWatchers.delete(w);
    }
    if (state.prefs.utc !== before) renderPage();
    return state.prefs;
}

const isPinned = (name) => (state.prefs?.pinned || []).some((p) => nameKey(p) === nameKey(name));

function pinButton(name) {
    const btn = button('Pin', {kind: 'ghost'});
    const draw = () => {
        const on = isPinned(name);
        replace(btn, icon('star'), on ? 'Pinned' : 'Pin');
        btn.setAttribute('aria-pressed', String(on));
        btn.title = on ? 'Unpin: take them off your pinned players' : 'Pin them to your players page and the search';
    };
    btn.addEventListener('click', async () => {
        try {
            await changePrefs({op: isPinned(name) ? 'unpin' : 'pin', name});
        } catch (e) {
            toast(e.message, 'error');
        }
    });
    draw();
    watchPrefs(btn, draw);
    return btn;
}

// A filter's settings as text, the same whatever order they're in.
const filterText = (params) => JSON.stringify(Object.entries(params).filter(([, v]) => v !== '' && v != null)
    .map(([k, v]) => [k, String(v)]).sort());

// The filters saved for `page`, as chips that apply one, and a button that
// saves what's set now. `current()` gives the settings, `apply(params)`
// sets them.
function savedFilters(page, current, apply) {
    const el = h('div', {class: 'chips saved-filters'});
    const save = () => {
        const name = h('input', {
            class: 'input',
            maxlength: '60',
            required: true,
            autofocus: true,
            placeholder: 'e.g. Bans this week',
            'data-invalid': 'Name it.'
        });
        openDialog({
            title: 'Save this filter',
            subtitle: 'Saved to your account: it follows you to any browser. Saving under a name you used replaces it.',
            body: [h('label', {class: 'field'}, h('span', {}, 'Name'), name)],
            actions: [{label: 'Cancel'}, {
                label: 'Save', kind: 'primary', icon: 'bookmark', run: async () => {
                    await changePrefs({op: 'save_filter', filter: {id: '', name: name.value, page, params: current()}});
                    toast('Filter saved.');
                }
            }],
        });
    };
    const draw = () => {
        const now = filterText(current());
        const saved = (state.prefs?.filters || []).filter((f) => f.page === page);
        const empty = filterText(current()) === '[]';
        replace(el,
            saved.map((f) => h('span', {class: 'saved-filter', 'aria-pressed': String(filterText(f.params) === now)},
                h('button', {
                    type: 'button',
                    title: 'Apply this saved filter',
                    on: {
                        click: () => {
                            apply(f.params);
                            draw();
                        }
                    }
                }, icon('bookmark'), f.name),
                h('button', {
                    type: 'button',
                    class: 'saved-forget',
                    'aria-label': `Forget ${f.name}`,
                    title: 'Forget this saved filter',
                    on: {click: () => changePrefs({op: 'remove_filter', id: f.id}).catch((e) => toast(e.message, 'error'))}
                }, icon('x')))),
            button('Save filter', {
                small: true,
                kind: 'ghost',
                icon: 'plus',
                disabled: empty,
                title: empty ? 'Set a filter first' : 'Save these settings to apply again later',
                on: {click: save}
            }));
    };
    draw();
    watchPrefs(el, draw);
    return {el, draw};
}

// A button that downloads `path()` (an /api/export address) as a CSV file.
function exportButton(path, title = 'Download what this shows as a CSV file (recorded in the audit log)') {
    return button('Export CSV', {
        small: true,
        kind: 'ghost',
        icon: 'download',
        title,
        on: {
            click: () => {
                const a = h('a', {href: `/api${path()}`, download: ''});
                document.body.append(a);
                a.click();
                a.remove();
                toast('Exporting: the download starts when it\'s ready.');
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Worlds data, polled while signed in

const worldListeners = new Set();
let pollTimer = null;

function onWorlds(fn) {
    worldListeners.add(fn);
    return () => worldListeners.delete(fn);
}

function setWorlds(worlds, canCommand) {
    const fetchedAt = Date.now();
    state.worlds = worlds.map((w) => ({
        ...w,
        // When its reboot fires, by this browser's clock.
        rebootAt: w.status && w.status.reboot_in_secs != null
            ? fetchedAt - w.checked_ago_ms + w.status.reboot_in_secs * 1000
            : null,
    }));
    if (canCommand !== undefined) state.canCommand = canCommand;
    state.worldsLoaded = true;
    updateNavCounts();
    for (const fn of worldListeners) fn(state.worlds);
}

async function loadWorlds() {
    try {
        const data = await api('GET', '/worlds');
        setWorlds(data.worlds, data.can_command);
    } catch (e) {
        if (e.status !== 401) console.warn(e);
    }
}

function startPolling() {
    stopPolling();
    loadWorlds();
    pollTimer = setInterval(() => {
        if (!document.hidden) loadWorlds();
    }, 5000);
    loadCaseSummary();
    // Hidden tabs keep asking too, so the title shows new detections.
    caseTimer = setInterval(loadCaseSummary, 15000);
}

function stopPolling() {
    clearInterval(pollTimer);
    clearInterval(caseTimer);
    pollTimer = null;
    caseTimer = null;
}

const worldName = (w) => `World ${w.id}`;
const isRebooting = (w) => w.online && (w.rebootAt !== null || w.status?.shutting_down);

function totals() {
    const online = state.worlds.filter((w) => w.online);
    return {
        online,
        players: online.reduce((n, w) => n + w.status.players, 0),
        rebooting: state.worlds.filter(isRebooting),
    };
}

// Every countdown on the page ticks from one timer.
setInterval(() => {
    for (const el of document.querySelectorAll('[data-deadline]')) {
        const left = (Number(el.dataset.deadline) - Date.now()) / 1000;
        el.textContent = left > 0 ? clock(left) : 'now';
    }
}, 1000);

function countdown(deadline) {
    const left = (deadline - Date.now()) / 1000;
    return h('span', {class: 'countdown', dataset: {deadline: String(deadline)}}, left > 0 ? clock(left) : 'now');
}

// ---------------------------------------------------------------------------
// World commands

function commandResults(title, replies, retry) {
    const failed = replies.filter((r) => r.status !== 'answered' || !r.answer.ok);
    const ok = replies.length - failed.length;
    const rows = replies.map((r) => {
        const good = r.status === 'answered' && r.answer.ok;
        const message = r.status === 'answered' ? r.answer.message : r.error;
        return h('div', {class: `result ${good ? 'ok' : 'fail'}`},
            icon(good ? 'check' : 'x'), h('b', {}, `World ${r.world}`), h('span', {}, message));
    });
    const actions = [{label: 'Done', kind: failed.length ? undefined : 'primary'}];
    if (failed.length && retry) {
        actions.unshift({
            label: `Retry ${failed.length} failed`,
            kind: 'primary',
            icon: 'refresh',
            run: async () => retry(failed.map((r) => r.node)),
        });
    }
    openDialog({
        title,
        subtitle: failed.length ? `${ok} of ${replies.length} worlds succeeded.` : `All ${plural(replies.length, 'world')} succeeded.`,
        body: [h('div', {class: 'results'}, rows)],
        actions,
    });
}

async function sendCommand(path, body, title) {
    const run = async (nodes) => {
        const result = await api('POST', path, {...body, nodes});
        setWorlds(result.worlds);
        commandResults(title, result.replies, run);
    };
    await run(body.nodes);
}

function targetsSummary(worlds) {
    if (worlds.length === state.worlds.length && worlds.length > 1) return `All ${worlds.length} worlds`;
    if (worlds.length <= 6) return worlds.map(worldName).join(', ');
    return `${plural(worlds.length, 'world')}`;
}

// Reboots `worlds`, or with `stay`, shuts them down: they stay off until a
// maintainer starts them again.
function rebootDialog(worlds, onDone, stay = false) {
    const presets = [[60, '1 min'], [300, '5 min'], [600, '10 min'], [1800, '30 min'], [3600, '1 hour']];
    const staggers = [[0, 'None'], [15, '15s'], [30, '30s'], [60, '1 min'], [300, '5 min']];
    let seconds = 300;
    let stagger = 0;
    const many = worlds.length > 1;

    const custom = h('input', {
        class: 'input',
        type: 'number',
        min: '0',
        max: '600',
        step: '1',
        placeholder: 'Minutes',
        inputmode: 'numeric',
        'data-invalid': 'Countdowns are 0 to 600 whole minutes.'
    });
    const delayChips = h('div', {class: 'chips'});
    const staggerChips = h('div', {class: 'chips'});
    const timeline = h('div', {class: 'timeline'});
    const reason = h('input', {
        class: 'input',
        maxlength: '80',
        placeholder: 'Optional, e.g. "update"',
        autocomplete: 'off'
    });
    const preview = h('p', {class: 'hint'});
    const verb = stay ? 'Shut down' : 'Reboot';
    // A shutdown is typed out even for one world: it doesn't come back.
    const confirm = many || stay;
    const confirmText = `${verb.toLowerCase()} ${many ? `${worlds.length} worlds` : worldName(worlds[0]).toLowerCase()}`;
    const confirmInput = h('input', {
        class: 'input',
        placeholder: confirmText,
        autocomplete: 'off',
        spellcheck: 'false'
    });
    let action;

    const update = () => {
        replace(delayChips, presets.map(([s, label]) =>
            h('button', {
                type: 'button',
                class: 'chip',
                'aria-pressed': String(seconds === s && !custom.value),
                on: {
                    click: () => {
                        custom.value = '';
                        seconds = s;
                        update();
                    }
                }
            }, label)));
        replace(staggerChips, staggers.map(([s, label]) =>
            h('button', {
                type: 'button', class: 'chip', 'aria-pressed': String(stagger === s), on: {
                    click: () => {
                        stagger = s;
                        update();
                    }
                }
            }, label)));
        const shown = worlds.slice(0, 50);
        replace(timeline, shown.map((w, i) => h('div', {}, h('span', {}, worldName(w)), h('span', {class: 'num'}, `in ${clock(seconds + stagger * i)}`))),
            worlds.length > shown.length && h('div', {class: 'muted'}, `and ${worlds.length - shown.length} more`));
        preview.textContent = reason.value.trim()
            ? `Players also get the message: "${stay ? 'Shutting down' : 'Rebooting'} in ${clock(seconds)}: ${reason.value.trim()}"`
            : 'Players see the countdown. Add a reason to also send them a message with it.';
        if (action?.button) action.refresh(action.button);
    };
    custom.addEventListener('input', () => {
        const minutes = Number(custom.value);
        if (custom.value && Number.isFinite(minutes) && minutes >= 0) seconds = Math.round(minutes * 60);
        update();
    });
    confirmInput.addEventListener('input', () => action.refresh(action.button));
    reason.addEventListener('input', update);
    update();

    action = {
        label: many ? `${verb} ${worlds.length} worlds` : `${verb} ${worldName(worlds[0])}`,
        kind: 'danger',
        icon: 'power',
        refresh: (btn) => {
            btn.disabled = confirm && confirmInput.value.trim().toLowerCase() !== confirmText;
        },
        run: async () => {
            await sendCommand(stay ? '/worlds/shutdown' : '/worlds/reboot', {
                nodes: worlds.map((w) => w.node),
                seconds,
                stagger_secs: many ? stagger : 0,
                reason: reason.value.trim() || null
            }, stay ? 'Shutdown scheduled' : 'Reboot scheduled');
            onDone?.();
        },
    };

    openDialog({
        title: many ? `${verb} ${worlds.length} worlds` : `${verb} ${worldName(worlds[0])}`,
        subtitle: many ? targetsSummary(worlds) : worlds[0].region,
        body: [
            h('div', {class: 'field'}, h('span', {}, 'Countdown'), delayChips, custom),
            many && h('div', {class: 'field'}, h('span', {}, 'Stagger'), staggerChips,
                h('p', {class: 'hint'}, 'Spaces the worlds out so they don\'t all go down, and fill back up, at once.')),
            h('div', {class: 'field'}, h('span', {}, 'Schedule'), timeline),
            h('label', {class: 'field'}, h('span', {}, 'Reason'), reason, preview),
            h('div', {class: `callout ${stay ? 'danger' : 'warn'}`}, icon('alert'),
                h('div', {}, stay
                    ? 'Players see the countdown, then are saved and logged out. The world then stays off: nobody can play on it until a maintainer starts it again from this page.'
                    : 'Players see the countdown, then are saved and logged out. The world process exits when it ends; your service manager starts it again.')),
            confirm && h('label', {class: 'field'}, h('span', {}, `Type "${confirmText}" to confirm`), confirmInput),
        ],
        actions: [{label: 'Cancel'}, action],
    });
}

// Starts the worlds in `worlds` that are off and that the console can start
// (it knows their systemd unit).
function startDialog(worlds, onDone) {
    const off = worlds.filter((w) => !w.online);
    const targets = off.filter((w) => w.startable);
    const notes = [
        off.length < worlds.length && `${plural(worlds.length - off.length, 'world')} already running, left as ${worlds.length - off.length === 1 ? 'it is' : 'they are'}.`,
        targets.length < off.length && `${plural(off.length - targets.length, 'world')} can't be started from the console: its worlds file names no systemd unit for ${off.length - targets.length === 1 ? 'it' : 'them'}.`,
    ].filter(Boolean);
    if (!targets.length) {
        toast(notes.join(' ') || 'Pick a world that is off.', 'error');
        return;
    }
    confirmDialog({
        title: targets.length === 1 ? `Start ${worldName(targets[0])}` : `Start ${targets.length} worlds`,
        subtitle: targetsSummary(targets),
        body: [h('p', {class: 'muted'}, 'Starts the world process. It loads for a minute or so, then shows as online and players can log in.'),
            notes.length > 0 && h('p', {class: 'hint'}, notes.join(' '))],
        label: 'Start',
        run: async () => {
            await sendCommand('/worlds/start', {nodes: targets.map((w) => w.node)}, 'Start');
            onDone?.();
        },
    });
}

function cancelRebootDialog(worlds, onDone) {
    const pending = worlds.filter(isRebooting);
    const targets = pending.length ? pending : worlds;
    confirmDialog({
        title: 'Cancel reboot',
        subtitle: targetsSummary(targets),
        body: [h('p', {class: 'muted'}, pending.length
            ? `Stops the countdown on ${plural(pending.length, 'world')}. Worlds already logging players out can't be stopped.`
            : 'None of these worlds has a reboot scheduled that the console knows of. Send the cancel anyway?')],
        label: 'Cancel reboot',
        run: async () => {
            await sendCommand('/worlds/cancel-reboot', {nodes: targets.map((w) => w.node)}, 'Reboot cancelled');
            onDone?.();
        },
    });
}

function broadcastDialog(worlds, onDone) {
    const max = 400;
    const text = h('textarea', {
        class: 'input',
        maxlength: String(max),
        placeholder: 'The game will be updating shortly...',
        required: true,
        autofocus: true
    });
    const counter = h('span', {class: 'hint num'}, `0 / ${max}`);
    text.addEventListener('input', () => {
        counter.textContent = `${text.value.length} / ${max}`;
    });
    const action = {
        label: 'Send broadcast',
        kind: 'primary',
        icon: 'megaphone',
        run: async () => {
            await sendCommand('/worlds/broadcast', {
                nodes: worlds.map((w) => w.node),
                message: text.value.trim()
            }, 'Broadcast sent');
            onDone?.();
        },
    };
    openDialog({
        title: 'Broadcast',
        subtitle: `To everyone on ${targetsSummary(worlds).replace(/^All/, 'all')}`,
        body: [h('label', {class: 'field'}, h('span', {}, 'Message'), text, counter)],
        actions: [{label: 'Cancel'}, action],
    });
}

function maintenanceDialog(worlds, onDone) {
    const open = worlds.filter((w) => !w.status?.maintenance);
    let enabled = open.length > 0;
    const choice = h('div', {class: 'segmented', role: 'group', 'aria-label': 'Maintenance'});
    const effect = h('div', {class: 'callout'});
    let action;
    const update = () => {
        replace(choice,
            h('button', {
                type: 'button', 'aria-pressed': String(enabled), on: {
                    click: () => {
                        enabled = true;
                        update();
                    }
                }
            }, 'Turn on'),
            h('button', {
                type: 'button', 'aria-pressed': String(!enabled), on: {
                    click: () => {
                        enabled = false;
                        update();
                    }
                }
            }, 'Turn off'));
        replace(effect, icon(enabled ? 'lock' : 'unlock'), h('div', {}, enabled
            ? 'New logins are refused with "the server is being updated". Players already on stay, and staff can still log in. A restart opens the world again.'
            : 'Anyone can log in again.'));
        if (action?.button) replace(action.button, icon(enabled ? 'lock' : 'unlock'), enabled ? 'Turn on maintenance' : 'Turn off maintenance');
    };
    action = {
        label: enabled ? 'Turn on maintenance' : 'Turn off maintenance',
        kind: 'primary',
        icon: enabled ? 'lock' : 'unlock',
        run: async () => {
            await sendCommand('/worlds/maintenance', {
                nodes: worlds.map((w) => w.node),
                enabled
            }, enabled ? 'Maintenance on' : 'Maintenance off');
            onDone?.();
        },
    };
    update();
    openDialog({
        title: 'Maintenance mode',
        subtitle: targetsSummary(worlds),
        body: [choice, effect],
        actions: [{label: 'Cancel'}, action],
    });
    update();
}

// ---------------------------------------------------------------------------
// Moderation

const DURATIONS = [[1, '1 hour'], [24, '1 day'], [48, '2 days'], [168, '1 week'], [720, '30 days'], [8760, '1 year'], [87600, 'Permanent']];
// The longest mute a trial moderator gives (TRIAL_MAX_MUTE_HOURS).
const TRIAL_MAX_HOURS = 48;

// `caseId`: the case it's sent from, whose timeline notes it. `also`: linked
// accounts to tick for the same mute or ban.
function moderateDialog(name, preset = 'kick', onDone, caseId, {also = []} = {}) {
    // Trial moderators kick and give short mutes, and nothing else.
    const trial = isTrial();
    const ACTIONS = trial ? {kick: 'Kick', mute: 'Mute'} : {kick: 'Kick', mute: 'Mute', ban: 'Ban', unmute: 'Unmute', unban: 'Unban'};
    const durations = trial ? DURATIONS.filter(([h0]) => h0 <= TRIAL_MAX_HOURS) : DURATIONS;
    const maxHours = trial ? TRIAL_MAX_HOURS : 87600;
    let action = ACTIONS[preset] ? preset : 'mute';
    let hours = 48;
    // A sanction template's id, and where the player stands on it.
    let template = '';
    let standing = null;
    let linked = null;
    const tickedAlso = new Set(also);
    const nameInput = h('input', {
        class: 'input',
        value: name || '',
        placeholder: 'Player name',
        maxlength: '12',
        pattern: NAME_PATTERN,
        'data-invalid': NAME_RULE,
        required: true,
        autocomplete: 'off',
        spellcheck: 'false',
        autofocus: !name
    });
    nameSuggest(nameInput);
    const reason = h('textarea', {
        class: 'input',
        placeholder: 'Optional. Saved in the audit log and the moderation history.',
        maxlength: '300',
        value: caseId ? `Case #${caseId}: ` : ''
    });
    const templatePick = h('select', {class: 'input', 'aria-label': 'Template'});
    const templateField = h('label', {class: 'field'}, h('span', {}, 'Template'), templatePick);
    const templateInfo = h('div', {});
    const segmented = h('div', {class: 'segmented', role: 'group', 'aria-label': 'Action'});
    const durationField = h('div', {class: 'field'});
    const custom = h('input', {
        class: 'input',
        type: 'number',
        min: '1',
        max: String(maxHours),
        step: '1',
        placeholder: 'Custom length in hours',
        inputmode: 'numeric',
        'data-invalid': `Lengths are 1 to ${maxHours} whole hours.`
    });
    const effect = h('div', {class: 'callout'});
    const alsoField = h('div', {class: 'field hidden'});
    let confirm;
    let seq = 0;

    // With a template, the ladder decides what happens.
    const step = () => (template && standing ? standing.step : null);
    const effective = () => (step() ? {action: step().action, hours: step().hours} : {action, hours});
    const length = (n) => hoursLabel(n);
    const timed = (a) => a === 'mute' || a === 'ban';
    // A template's next step can be past what a trial moderator may give.
    const beyondTrial = (now) => trial && (!ACTIONS[now.action] || (now.action === 'mute' && now.hours > TRIAL_MAX_HOURS));

    const update = () => {
        const now = effective();
        templateField.classList.toggle('hidden', !(state.templates || []).length || (!template && !timed(action)));
        segmented.classList.toggle('hidden', !!template);
        replace(segmented, Object.entries(ACTIONS).map(([key, label]) =>
            h('button', {
                type: 'button', 'aria-pressed': String(action === key), on: {
                    click: () => {
                        action = key;
                        update();
                        lookUp();
                    }
                }
            }, label)));
        durationField.classList.toggle('hidden', !!template || !timed(action));
        replace(durationField, h('span', {}, 'Length'), h('div', {class: 'chips'}, durations.map(([h0, label]) =>
            h('button', {
                type: 'button',
                class: 'chip',
                'aria-pressed': String(hours === h0 && !custom.value),
                on: {
                    click: () => {
                        custom.value = '';
                        hours = h0;
                        update();
                    }
                }
            }, label))), custom);

        const t = (state.templates || []).find((x) => x.id === template);
        replace(templateInfo, t && (standing
            ? h('div', {class: 'field'},
                h('span', {}, standing.offences.length ? `${plural(standing.offences.length, 'earlier offence')} on record: this is offence ${standing.offences.length + 1}` : 'First offence'),
                ladder(t.steps, standing.next))
            : validName(nameInput.value.trim()) && h('div', {class: 'spinner'})));

        const who = nameInput.value.trim() || 'They';
        const effects = {
            kick: `${who} is logged out of whichever world they're on. They can log straight back in.`,
            mute: `${who} can't use public or private chat for ${length(now.hours)}.`,
            ban: `${who} is logged out now and can't log in for ${length(now.hours)}.`,
            unmute: `${who} can chat again straight away.`,
            unban: `${who} can log in again straight away.`,
        };
        effect.className = `callout ${now.action === 'ban' || beyondTrial(now) ? 'danger' : ''}`;
        replace(effect, icon(now.action === 'ban' ? 'ban' : 'alert'), h('div', {}, template && !standing ? 'Looking up their record...'
            : beyondTrial(now) ? `${effects[now.action]} That's past what a trial moderator can give: ask a moderator to do it.` : effects[now.action]));

        const reach = !trial && timed(now.action) && linked && linked.length;
        alsoField.classList.toggle('hidden', !reach);
        if (reach) {
            replace(alsoField, h('span', {}, `Also ${now.action} linked accounts`),
                linked.map((a) => {
                    const box = h('input', {type: 'checkbox', checked: tickedAlso.has(a.name)});
                    box.addEventListener('change', () => {
                        if (box.checked) tickedAlso.add(a.name); else tickedAlso.delete(a.name);
                        update();
                    });
                    return h('label', {class: 'check-row'}, box, h('span', {}, a.name), sanctionBadges(a),
                        h('span', {class: 'muted'}, ` shares ${plural(a.shared.length, 'address', 'addresses')}`));
                }),
                h('p', {class: 'hint'}, 'Accounts that logged in or registered from the same address.'));
        }
        if (confirm?.button) {
            const n = reach ? [...tickedAlso].filter((x) => linked.some((a) => a.name === x)).length : 0;
            replace(confirm.button, ACTIONS[now.action] + (nameInput.value.trim() ? ` ${nameInput.value.trim()}` : '') + (n ? ` and ${n} linked` : ''));
            confirm.button.className = `btn ${now.action === 'ban' || now.action === 'kick' ? 'danger' : 'primary'}`;
            confirm.button.disabled = (!!template && !standing) || beyondTrial(now);
        }
    };

    // The player's standing on the chosen template, and their linked accounts.
    const lookUp = debounce(async () => {
        const mine = ++seq;
        const target = nameInput.value.trim();
        standing = null;
        linked = null;
        update();
        if (!validName(target)) return;
        const [stand, links] = await Promise.allSettled([
            template ? api('GET', `/templates/standing?${query({name: target})}`) : Promise.resolve(null),
            api('GET', `/lookup/links?${query({name: target})}`),
        ]);
        if (mine !== seq) return;
        if (stand.status === 'fulfilled' && stand.value) standing = stand.value.find((s) => s.template === template) || null;
        if (links.status === 'fulfilled') linked = links.value.accounts;
        update();
    }, 250);

    const drawTemplates = () => {
        replace(templatePick, h('option', {value: ''}, 'None: choose by hand'),
            (state.templates || []).map((t) => h('option', {value: t.id}, t.name)));
        templatePick.value = template;
        update();
    };
    templatePick.addEventListener('change', () => {
        template = templatePick.value;
        standing = null;
        update();
        lookUp();
    });
    (state.templates ? Promise.resolve() : loadTemplates()).then(drawTemplates).catch(() => {
    });

    custom.addEventListener('input', () => {
        const value = Math.round(Number(custom.value));
        if (custom.value && value >= 1) hours = value;
        update();
    });
    nameInput.addEventListener('input', () => {
        update();
        lookUp();
    });

    confirm = {
        label: 'Confirm',
        kind: 'danger',
        run: async () => {
            const target = nameInput.value.trim();
            // "Case #12: " left as it was is just "Case #12".
            const why = reason.value.trim().replace(/:$/, '');
            const now = effective();
            const others = timed(now.action) && linked ? [...tickedAlso].filter((x) => linked.some((a) => a.name === x)) : [];
            const result = action === 'kick' && !template
                ? await api('POST', '/players/kick', {name: target, reason: why, case: caseId ?? null})
                : await api('POST', '/players/moderate', {
                    name: target,
                    action: template ? null : action,
                    hours: !template && timed(action) ? hours : null,
                    template: template || null,
                    also: others,
                    reason: why,
                    case: caseId ?? null,
                });
            const ok = action === 'kick' && !template ? result.kicked_from.length > 0 : result.ok;
            toast(result.message, ok ? 'ok' : 'error');
            if (result.also?.length) {
                const done = result.also.filter((r) => r.ok).length;
                toast(`Linked accounts: ${done} of ${result.also.length} done.${done < result.also.length ? ` ${result.also.find((r) => !r.ok).message}` : ''}`, done === result.also.length ? 'ok' : 'error');
            }
            if (ok) onDone?.();
            return !ok;
        },
    };
    update();
    openDialog({
        title: 'Moderate player',
        subtitle: caseId ? `From case #${caseId}: noted in its timeline.` : name ? `Acting on ${name}` : 'Online or offline: it applies wherever they are.',
        body: [
            h('label', {class: 'field'}, h('span', {}, 'Player'), nameInput),
            templateField,
            templateInfo,
            segmented,
            durationField,
            effect,
            alsoField,
            h('label', {class: 'field'}, h('span', {}, 'Reason'), reason),
        ],
        actions: [{label: 'Cancel'}, confirm],
    });
    update();
    lookUp();
}

// ---------------------------------------------------------------------------
// Routing

const NAV = [
    {path: '/', label: 'Overview', icon: 'home', page: overviewPage},
    {path: '/worlds', label: 'Worlds', icon: 'globe', page: worldsPage},
    {path: '/players', label: 'Players', icon: 'users', page: playersPage},
    {path: '/moderation', label: 'Moderation', icon: 'shield', page: moderationPage},
    {path: '/investigate', label: 'Investigate', icon: 'target', page: investigatePage},
    {path: '/economy', label: 'Economy', icon: 'coins', page: economyPage},
    {path: '/controls', label: 'Controls', icon: 'zap', page: controlsPage},
    {path: '/schedule', label: 'Schedule', icon: 'clock', page: schedulePage},
    {path: '/chat', label: 'Chat logs', icon: 'chat', page: chatPage},
    {path: '/replays', label: 'Replays', icon: 'play', page: replaysPage},
    {path: '/audit', label: 'Audit log', icon: 'list', page: auditPage},
    {path: '/news', label: 'News', icon: 'news', page: newsPage, admin: true},
    {path: '/staff', label: 'Staff', icon: 'key', page: staffPage, maintainer: true},
    {path: '/account', label: 'Account', icon: 'user', page: accountPage},
];
// Pages reached from others, shown under their parent in the nav.
const PAGES = [
    ...NAV,
    {path: '/player', label: 'Player', page: playerPage, parent: '/players'},
    {path: '/replay', label: 'Replay', page: replayPage, parent: '/replays'},
    {path: '/case', label: 'Case', page: casePage, parent: '/moderation'},
    {path: '/staff/member', label: 'Staff member', page: staffMemberPage, parent: '/staff', maintainer: true},
];
// The first few fit the phone tab bar; the rest go under More.
const TABS = 4;

const isMaintainer = () => state.session?.role === 'maintainer';
const isTrial = () => state.session?.role === 'trial';
// The game staff level each role acts at; staff above yours keep their
// sessions and logs from you.
const ROLE_LEVELS = {trial: 1, moderator: 1, admin: 2, maintainer: 3};
const mayWatch = (staffLevel) => (staffLevel || 0) <= (ROLE_LEVELS[state.session?.role] || 0);
const mayOpen = (n) => (!n.maintainer || isMaintainer()) && (!n.admin || state.canCommand);
const navItems = () => NAV.filter(mayOpen);

function navigate(path) {
    if (path !== location.pathname + location.search) history.pushState(null, '', path);
    renderPage();
}

window.addEventListener('popstate', () => renderPage());

document.addEventListener('click', (e) => {
    const link = e.target.closest('a[data-link]');
    if (!link || e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    e.preventDefault();
    navigate(link.getAttribute('href'));
});

const link = (href, props, ...children) => h('a', {href, 'data-link': true, ...props}, ...children);

// ---------------------------------------------------------------------------
// App shell

const app = document.getElementById('app');
let shell = null;
let pageCleanup = [];

function render() {
    app.className = '';
    shell = null;
    for (const fn of pageCleanup) fn();
    pageCleanup = [];
    if (state.session?.stage === 'signed_in') {
        if (!state.prefs) {
            loadPrefs().then(render);
            return;
        }
        state.canCommand = state.session.role === 'admin' || state.session.role === 'maintainer';
        renderShell();
        renderPage();
        startPolling();
    } else {
        stopPolling();
        state.prefs = null;
        replace(app, authView());
        app.querySelector('[autofocus]')?.focus();
    }
}

function themeButton() {
    const icons = {auto: 'monitor', light: 'sun', dark: 'moon'};
    const next = {auto: 'light', light: 'dark', dark: 'auto'};
    const btn = button(null, {kind: 'ghost', 'aria-label': 'Theme'});
    const set = (theme) => {
        replace(btn, icon(icons[theme]));
        btn.title = `Theme: ${theme}`;
    };
    set(prefs.get('theme', 'dark'));
    btn.addEventListener('click', () => {
        const theme = next[prefs.get('theme', 'dark')];
        prefs.set('theme', theme);
        applyTheme(theme);
        set(theme);
    });
    return btn;
}

// Switches every time between this browser's zone and UTC.
function zoneButton() {
    const btn = h('button', {type: 'button', class: 'btn ghost zone-button'});
    const draw = () => {
        replace(btn, h('span', {class: 'num'}, inUtc() ? 'UTC' : 'Local'));
        btn.title = inUtc() ? 'Times are in UTC. Click for your own time zone.' : `Times are in your time zone (${Intl.DateTimeFormat().resolvedOptions().timeZone}). Click for UTC.`;
        btn.setAttribute('aria-label', btn.title);
    };
    btn.addEventListener('click', () => changePrefs({op: 'utc', utc: !inUtc()}).catch((e) => toast(e.message, 'error')));
    draw();
    watchPrefs(btn, draw);
    return btn;
}

async function signOut() {
    try {
        await api('POST', '/auth/logout');
    } catch { /* signed out regardless */
    }
    await clearReplayStore();
    state.session = {stage: 'signed_out'};
    history.replaceState(null, '', '/');
    render();
}

function renderShell() {
    const {name, role} = state.session;
    const navLinks = navItems().map((item) => link(item.path, {dataset: {nav: item.path}},
        icon(item.icon), h('span', {}, item.label), h('span', {class: 'count num', dataset: {count: item.path}})));
    const sidebar = h('aside', {class: 'sidebar'},
        h('div', {class: 'brand'}, h('img', {
            src: '/icon.svg',
            alt: ''
        }), h('div', {}, 'Rust City', h('small', {}, 'Staff console'))),
        h('button', {
            type: 'button',
            class: 'palette-trigger',
            on: {click: openPalette}
        }, icon('search'), h('span', {}, 'Search'), h('kbd', {}, navigator.platform.includes('Mac') ? '⌘K' : 'Ctrl K')),
        h('nav', {class: 'nav', 'aria-label': 'Main'}, navLinks),
        h('div', {class: 'sidebar-foot'},
            h('div', {class: 'me'},
                h('div', {class: 'me-who'},
                    h('div', {class: 'avatar'}, initial(name)),
                    h('div', {class: 'grow'}, h('b', {}, name), h('small', {}, ROLE_NAMES[role] || role))),
                h('div', {class: 'me-actions'},
                    zoneButton(),
                    themeButton(),
                    button(null, {
                        kind: 'ghost',
                        icon: 'logout',
                        'aria-label': 'Sign out',
                        title: 'Sign out',
                        on: {click: signOut}
                    })))));

    const topbar = h('header', {class: 'topbar'},
        h('img', {src: '/icon.svg', alt: ''}), h('b', {}, 'RS Admin'),
        h('div', {class: 'actions'},
            button(null, {kind: 'ghost', icon: 'search', 'aria-label': 'Search', on: {click: openPalette}}),
            zoneButton(),
            themeButton()));

    const more = h('button', {
        type: 'button',
        class: 'tab-more',
        dataset: {more: ''},
        on: {click: moreSheet}
    }, icon('more'), h('span', {}, 'More'));
    const tabbar = h('nav', {class: 'tabbar', 'aria-label': 'Main'},
        navItems().slice(0, TABS).map((item) => link(item.path, {dataset: {nav: item.path}}, icon(item.icon), h('span', {}, item.short || item.label))),
        more);

    const main = h('main', {class: 'main', id: 'main'});
    shell = {main};
    replace(app, h('div', {class: 'shell'}, sidebar, topbar, main, tabbar));
    updateNavCounts();
}

function moreSheet() {
    const {close} = openDialog({
        title: 'More',
        body: [h('nav', {class: 'nav', 'aria-label': 'More pages'},
            navItems().slice(TABS).map((item) => link(item.path, {
                    on: {click: () => close()},
                    'aria-current': location.pathname === item.path ? 'page' : null
                },
                icon(item.icon), h('span', {}, item.label))))],
    });
}

function moderationCount() {
    const s = state.caseSummary;
    if (!s || !s.open) return '';
    return s.yours ? `${s.yours}/${s.open}` : String(s.open);
}

function updateNavCounts() {
    if (!shell) return;
    const {online, players} = totals();
    const counts = {
        '/worlds': state.worldsLoaded ? `${online.length}/${state.worlds.length}` : '',
        '/players': state.worldsLoaded ? players.toLocaleString() : '',
        // Open cases, with how many are yours first when any are.
        '/moderation': moderationCount(),
        '/economy': state.economyAlerts ? String(state.economyAlerts) : '',
    };
    for (const el of document.querySelectorAll('[data-count]')) el.textContent = counts[el.dataset.count] || '';
    // A dot when a case changed since its staff member last looked.
    for (const el of document.querySelectorAll('[data-count="/economy"]')) {
        el.classList.toggle('unread', !!state.economyAlerts);
        el.title = state.economyAlerts ? `${plural(state.economyAlerts, 'economy alert')} nobody has looked at` : '';
    }
    // Red when the console opened a case nobody is on yet.
    const s = state.caseSummary;
    for (const el of document.querySelectorAll('[data-count="/moderation"]')) {
        el.classList.toggle('unread', !!s?.unread && !s?.detected);
        el.classList.toggle('alert', !!s?.detected);
        el.title = [s?.detected && `${plural(s.detected, 'new detection')} nobody is on yet`,
            s?.unread && `${plural(s.unread, 'case')} changed since you last looked`].filter(Boolean).join(' · ');
    }
    markTitle();
}

// The tab's title starts with how many detections nobody is on yet, so
// they show from another tab too.
function markTitle() {
    const n = state.caseSummary?.detected || 0;
    const bare = document.title.replace(/^\(\d+\) /, '');
    document.title = n ? `(${n}) ${bare}` : bare;
}

function renderPage() {
    if (!shell) return render();
    for (const fn of pageCleanup) fn();
    pageCleanup = [];
    const path = location.pathname;
    const allowed = PAGES.filter(mayOpen);
    const item = allowed.find((n) => n.path === path) || NAV[0];
    if (!allowed.some((n) => n.path === path)) history.replaceState(null, '', '/');
    const current = item.parent || item.path;
    for (const a of document.querySelectorAll('[data-nav]')) {
        if (a.dataset.nav === current) a.setAttribute('aria-current', 'page');
        else a.removeAttribute('aria-current');
    }
    const underMore = !navItems().slice(0, TABS).some((n) => n.path === current);
    document.querySelector('[data-more]')?.setAttribute('aria-current', underMore ? 'page' : 'false');
    document.title = `${item.label} · RS Admin`;
    markTitle();
    const ctx = {
        cleanup: (fn) => pageCleanup.push(fn),
        params: new URLSearchParams(location.search),
    };
    replace(shell.main, item.page(ctx));
    window.scrollTo(0, 0);
}

function pageHead(title, subtitle, ...actions) {
    return h('div', {class: 'page-head'},
        h('div', {}, h('h1', {}, title), subtitle && h('p', {}, subtitle)),
        actions.length ? h('div', {class: 'actions'}, actions) : null);
}

// ---------------------------------------------------------------------------
// Sign-in

function authView() {
    const stage = state.session?.stage || 'signed_out';
    const card = h('div', {class: 'auth-card'});
    const error = h('p', {class: 'error-text', role: 'alert'});

    const header = (title, text) => h('header', {}, h('img', {
        src: '/icon.svg',
        alt: ''
    }), h('h1', {}, title), h('p', {}, text));

    const submitting = (form, fn) => form.addEventListener('submit', async (e) => {
        e.preventDefault();
        const submit = form.querySelector('button[type=submit]');
        const label = submit.textContent;
        submit.disabled = true;
        submit.replaceChildren(h('span', {class: 'spinner'}), label);
        error.textContent = '';
        try {
            await fn();
        } catch (err) {
            error.textContent = err.message;
            form.querySelector('input.code')?.select();
        } finally {
            submit.disabled = false;
            submit.textContent = label;
        }
    });

    const startOver = h('p', {class: 'auth-foot'}, h('button', {
        type: 'button',
        on: {click: signOut}
    }, 'Use a different account'));

    const codeInput = () => {
        const input = h('input', {
            class: 'input code',
            name: 'code',
            inputmode: 'numeric',
            autocomplete: 'one-time-code',
            maxlength: '6',
            pattern: '[0-9]{6}',
            placeholder: '000000',
            required: true,
            autofocus: true,
            'aria-label': 'Authenticator code'
        });
        input.addEventListener('input', () => {
            input.value = input.value.replace(/\D/g, '').slice(0, 6);
            if (input.value.length === 6) input.form.requestSubmit();
        });
        return input;
    };

    const signedIn = (result) => {
        state.session = {stage: 'signed_in', name: result.name, role: result.role, csrf: result.csrf};
        render();
    };

    if (stage === 'signed_out') {
        const name = h('input', {
            class: 'input',
            name: 'username',
            autocomplete: 'username',
            required: true,
            autofocus: true,
            maxlength: '64'
        });
        const password = h('input', {
            class: 'input',
            type: 'password',
            name: 'password',
            autocomplete: 'current-password',
            required: true,
            maxlength: '256'
        });
        const form = h('form', {},
            h('label', {class: 'field'}, h('span', {}, 'Name'), name),
            h('label', {class: 'field'}, h('span', {}, 'Password'), password),
            error,
            h('button', {type: 'submit', class: 'btn primary block'}, 'Continue'));
        submitting(form, async () => {
            const result = await api('POST', '/auth/login', {name: name.value, password: password.value});
            state.session = {stage: result.stage, name: result.name, csrf: result.csrf};
            render();
        });
        append(card, header('Staff console', 'Sign in to manage the worlds.'), form,
            h('p', {class: 'auth-foot'}, 'No account? Ask a maintainer to add you.'));
    } else if (stage === 'totp') {
        const code = codeInput();
        const form = h('form', {}, code, error, h('button', {type: 'submit', class: 'btn primary block'}, 'Verify'));
        submitting(form, async () => signedIn(await api('POST', '/auth/totp', {code: code.value})));
        append(card, header('Two-factor authentication', `Enter the 6-digit code from your authenticator app for ${state.session.name}.`), form, startOver);
    } else if (stage === 'enroll') {
        const code = codeInput();
        const qr = h('div', {class: 'qr'});
        const key = h('span', {}, '...');
        const copy = button(null, {
            kind: 'ghost',
            small: true,
            icon: 'copy',
            'aria-label': 'Copy key',
            title: 'Copy key'
        });
        copy.addEventListener('click', async () => {
            try {
                await navigator.clipboard.writeText(key.textContent.replace(/\s/g, ''));
                toast('Key copied.');
            } catch {
                toast('Copy the key by hand.', 'error');
            }
        });
        api('GET', '/auth/enroll').then((data) => {
            key.textContent = data.key;
            qr.append(h('img', {
                src: `/api/auth/enroll/qr.svg?t=${Date.now()}`,
                alt: 'QR code for your authenticator app'
            }));
        }).catch((e) => {
            error.textContent = e.message;
        });
        const form = h('form', {}, code, error, h('button', {
            type: 'submit',
            class: 'btn primary block'
        }, 'Turn on and sign in'));
        submitting(form, async () => signedIn(await api('POST', '/auth/enroll', {code: code.value})));
        append(card,
            header('Set up two-factor authentication', 'Every console account needs an authenticator app.'),
            h('ol', {class: 'steps'},
                h('li', {}, 'Open an authenticator app (1Password, Google Authenticator, Authy...).'),
                h('li', {}, 'Scan this code, or enter the key by hand.'),
                h('li', {}, 'Type the 6-digit code it shows.')),
            qr,
            h('div', {class: 'secret'}, key, copy),
            form,
            startOver);
    }
    return h('div', {class: 'auth'}, card);
}

// ---------------------------------------------------------------------------
// Overview

function overviewPage(ctx) {
    const admin = state.canCommand;
    const stats = h('div', {class: 'stats'});
    const bars = h('div', {class: 'bars'});
    const activity = h('div', {class: 'list'});

    const stat = (iconName, label, value, sub) => h('div', {class: 'card stat'},
        h('div', {class: 'label'}, icon(iconName), label), h('div', {class: 'value num'}, value), h('div', {class: 'sub'}, sub));

    const draw = (worlds) => {
        if (!state.worldsLoaded) return;
        const {online, players, rebooting} = totals();
        const latencies = online.map((w) => w.latency_ms).filter((n) => n !== null);
        const avg = latencies.length ? Math.round(latencies.reduce((a, b) => a + b, 0) / latencies.length) : null;
        const soonest = rebooting.filter((w) => w.rebootAt).sort((a, b) => a.rebootAt - b.rebootAt)[0];
        const busiest = online.slice().sort((a, b) => b.status.players - a.status.players)[0];
        replace(stats,
            stat('users', 'Players online', players.toLocaleString(), busiest ? `Busiest: ${worldName(busiest)} (${busiest.status.players})` : 'No worlds online'),
            stat('globe', 'Worlds online', h('span', {}, online.length, h('small', {}, ` / ${worlds.length}`)),
                online.length === worlds.length ? 'All worlds reachable' : `${worlds.length - online.length} unreachable`),
            stat('clock', 'Reboots scheduled', rebooting.length, soonest ? h('span', {}, `Next: ${worldName(soonest)} in `, countdown(soonest.rebootAt)) : 'None pending'),
            stat('zap', 'Response time', avg === null ? '-' : h('span', {}, avg, h('small', {}, ' ms')), 'Average across online worlds'));
        const max = Math.max(1, ...online.map((w) => w.status.players));
        replace(bars, worlds.length ? worlds.map((w) => {
            const fill = h('i');
            fill.style.setProperty('width', `${w.online ? Math.max(2, (w.status.players / max) * 100) : 100}%`);
            return h('div', {class: 'bar-row'},
                link('/worlds', {}, h('b', {}, worldName(w))),
                h('div', {class: `meter ${w.online ? '' : 'offline'}`}, fill),
                h('span', {class: 'num muted'}, w.online ? w.status.players.toLocaleString() : 'offline'));
        }) : h('div', {class: 'empty'}, 'No worlds configured.'));
    };

    const loadActivity = async () => {
        try {
            const entries = await api('GET', '/audit?limit=8');
            replace(activity, entries.length ? entries.map(auditItem) : h('div', {class: 'empty'}, icon('list'), 'Nothing yet.'));
        } catch (e) {
            replace(activity, h('div', {class: 'empty'}, e.message));
        }
    };
    loadActivity();
    const timer = setInterval(() => {
        if (!document.hidden) loadActivity();
    }, 15000);
    ctx.cleanup(() => clearInterval(timer));
    ctx.cleanup(onWorlds(draw));
    draw(state.worlds);

    const banner = h('div', {});
    controlsBanner(banner);

    const hour = new Date().getHours();
    const greeting = hour < 12 ? 'Good morning' : hour < 18 ? 'Good afternoon' : 'Good evening';
    return h('div', {class: 'page'},
        pageHead('Overview', `${greeting}, ${state.session.name}.`,
            admin && button('Broadcast', {icon: 'megaphone', on: {click: () => broadcastDialog(totals().online)}}),
            admin && button('Reboot worlds', {icon: 'power', on: {click: () => navigate('/worlds')}})),
        banner,
        stats,
        h('div', {class: 'split'},
            h('section', {class: 'card'},
                h('div', {class: 'card-head'}, h('h2', {}, 'Population'), h('div', {class: 'actions'}, link('/worlds', {class: 'btn small ghost'}, 'All worlds', icon('chevron')))),
                h('div', {class: 'card-body'}, bars)),
            h('section', {class: 'card'},
                h('div', {class: 'card-head'}, h('h2', {}, 'Recent activity'), h('div', {class: 'actions'}, link('/audit', {class: 'btn small ghost'}, 'Audit log', icon('chevron')))),
                activity)));
}

// ---------------------------------------------------------------------------
// Worlds

function worldsPage(ctx) {
    const selected = new Set();
    let filter = ctx.params.get('filter') || 'all';
    let query = '';
    const tiles = h('div', {class: 'tiles'});
    const subtitle = h('p');
    const filters = h('div', {class: 'chips'});
    const countLabel = h('span', {class: 'count'});
    const bar = h('div', {class: 'selection-bar', role: 'toolbar', 'aria-label': 'Selected worlds'});

    const visible = () => state.worlds.filter((w) => {
        if (filter === 'online' && !w.online) return false;
        if (filter === 'offline' && w.online) return false;
        if (filter === 'rebooting' && !isRebooting(w)) return false;
        if (filter === 'maintenance' && !w.status?.maintenance) return false;
        if (!query) return true;
        const q = query.toLowerCase();
        return String(w.id).startsWith(q) || worldName(w).toLowerCase().includes(q) || (w.region || '').toLowerCase().includes(q);
    });
    const chosen = () => state.worlds.filter((w) => selected.has(w.node));
    const clear = () => {
        selected.clear();
        draw();
    };

    const tile = (w) => {
        const status = w.status;
        const pressed = selected.has(w.node);
        const fill = h('i');
        fill.style.setProperty('width', `${status ? Math.min(100, (status.players / 2000) * 100) : 0}%`);
        const meta = w.online
            ? [h('span', {class: 'dot ok'}), 'Online',
                h('span', {class: 'sep'}), `${w.latency_ms} ms`,
                h('span', {class: 'sep'}), `up ${span(status.uptime_secs)}`]
            : [h('span', {class: 'dot danger'}), w.error || 'Offline'];
        const el = h('div', {
                class: `tile ${w.online ? '' : 'offline'}`,
                role: state.canCommand ? 'button' : null,
                tabindex: '0',
                dataset: {node: String(w.node)},
                'aria-pressed': state.canCommand ? String(pressed) : null,
                'aria-label': `${worldName(w)}, ${w.online ? `${status.players} players` : 'offline'}`,
            },
            h('div', {class: 'tile-top'},
                h('div', {}, h('div', {class: 'tile-num'}, w.id, h('small', {}, `node ${w.node}`)), h('div', {class: 'tile-region'}, w.region || 'No region')),
                state.canCommand && h('span', {class: 'check'}, icon('check'))),
            status
                ? h('div', {class: 'tile-stats'}, h('b', {class: 'num'}, status.players.toLocaleString()), h('span', {class: 'muted'}, 'players'))
                : h('div', {class: 'tile-stats'}, h('b', {class: 'muted'}, 'Offline')),
            h('div', {class: `meter ${w.online ? '' : 'offline'}`}, fill),
            h('div', {class: 'tile-meta'}, meta),
            status?.health && healthLine(status.health),
            (status?.shutting_down || w.rebootAt || status?.maintenance) && h('div', {class: 'tile-badges'},
                status?.shutting_down
                    ? h('span', {class: 'badge danger'}, icon('power'), 'Shutting down')
                    : w.rebootAt && h('span', {
                    class: `badge ${status.stays_down ? 'danger' : 'warn'}`,
                    title: status.reboot_reason || ''
                }, icon('clock'), status.stays_down ? 'Shutting down in ' : 'Reboot in ', countdown(w.rebootAt)),
                status?.maintenance && h('span', {class: 'badge warn'}, icon('lock'), 'Maintenance')),
            status?.reboot_reason && w.rebootAt && h('div', {class: 'tile-reason'}, status.reboot_reason),
            w.online && button(null, {
                kind: 'ghost',
                small: true,
                icon: 'users',
                class: 'btn ghost small icon-only tile-link',
                'aria-label': `Players on ${worldName(w)}`,
                title: 'Players',
                on: {
                    click: (e) => {
                        e.stopPropagation();
                        worldPlayersDialog(w);
                    }
                }
            }));
        const activate = () => {
            if (!state.canCommand) {
                if (w.online) worldPlayersDialog(w);
                return;
            }
            if (selected.has(w.node)) selected.delete(w.node);
            else selected.add(w.node);
            el.setAttribute('aria-pressed', String(selected.has(w.node)));
            updateBar();
        };
        el.addEventListener('click', activate);
        el.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                activate();
            }
        });
        return el;
    };

    const draw = () => {
        const shown = visible();
        for (const node of [...selected]) if (!state.worlds.some((w) => w.node === node)) selected.delete(node);
        const {online, players} = totals();
        subtitle.textContent = state.worldsLoaded
            ? `${online.length} of ${plural(state.worlds.length, 'world')} online · ${plural(players, 'player')}`
            : 'Checking worlds...';
        const counts = {
            all: state.worlds.length,
            online: online.length,
            offline: state.worlds.length - online.length,
            rebooting: state.worlds.filter(isRebooting).length,
            maintenance: state.worlds.filter((w) => w.status?.maintenance).length,
        };
        replace(filters, Object.entries({
            all: 'All',
            online: 'Online',
            offline: 'Offline',
            rebooting: 'Rebooting',
            maintenance: 'Maintenance'
        })
            .filter(([key]) => key !== 'maintenance' || counts.maintenance > 0).map(([key, label]) =>
                h('button', {
                    type: 'button', class: 'chip', 'aria-pressed': String(filter === key), on: {
                        click: () => {
                            filter = key;
                            draw();
                        }
                    }
                }, `${label} `, h('span', {class: 'num muted'}, counts[key]))));
        // Redrawn every poll: keep keyboard focus on the same world.
        const focused = document.activeElement?.closest?.('.tile')?.dataset.node;
        replace(tiles, !state.worldsLoaded
            ? h('div', {class: 'empty'}, h('div', {class: 'spinner'}))
            : shown.length ? shown.map(tile) : h('div', {class: 'empty'}, icon('globe'), 'No worlds match.'));
        if (focused) tiles.querySelector(`[data-node="${focused}"]`)?.focus();
        updateBar();
    };

    const updateBar = () => {
        const picked = chosen();
        countLabel.textContent = `${plural(picked.length, 'world')} selected`;
        bar.classList.toggle('open', picked.length > 0);
    };

    const search = h('input', {
        class: 'input',
        type: 'search',
        placeholder: 'World number or region',
        'aria-label': 'Filter worlds'
    });
    search.addEventListener('input', () => {
        query = search.value.trim();
        draw();
    });

    append(bar,
        countLabel,
        button('Broadcast', {icon: 'megaphone', on: {click: () => broadcastDialog(chosen(), clear)}}),
        button('Reboot', {icon: 'power', on: {click: () => rebootDialog(chosen().sort((a, b) => a.id - b.id), clear)}}),
        button('Cancel reboot', {icon: 'undo', on: {click: () => cancelRebootDialog(chosen(), clear)}}),
        button('Schedule', {
            icon: 'clock',
            on: {click: () => scheduleDialog('reboot', chosen().sort((a, b) => a.id - b.id), clear)}
        }),
        button('Maintenance', {icon: 'lock', on: {click: () => maintenanceDialog(chosen(), clear)}}),
        isMaintainer() && button('Shut down', {
            icon: 'power',
            title: 'Count down, save everyone and turn the worlds off until they\'re started again',
            on: {click: () => rebootDialog(chosen().sort((a, b) => a.id - b.id), clear, true)}
        }),
        isMaintainer() && button('Start', {
            icon: 'play',
            title: 'Turn on worlds that are off',
            on: {click: () => startDialog(chosen().sort((a, b) => a.id - b.id), clear)}
        }),
        isMaintainer() && button('Reload scripts', {
            icon: 'refresh',
            title: 'Recompile one world\'s scripts and swap them in',
            on: {
                click: () => {
                    const picked = chosen();
                    if (picked.length !== 1) return toast('Pick one world: scripts are reloaded on one at a time.', 'error');
                    reloadScriptsDialog(picked[0], clear);
                },
            },
        }),
        button(null, {icon: 'x', 'aria-label': 'Clear selection', title: 'Clear selection', on: {click: clear}}));

    const selectShown = () => {
        const shown = visible();
        const all = shown.every((w) => selected.has(w.node));
        for (const w of shown) all ? selected.delete(w.node) : selected.add(w.node);
        draw();
    };

    const onKey = (e) => {
        if (e.key === 'Escape' && selected.size && !document.querySelector('dialog[open]')) clear();
    };
    document.addEventListener('keydown', onKey);
    ctx.cleanup(() => document.removeEventListener('keydown', onKey));
    ctx.cleanup(onWorlds(draw));
    ctx.cleanup(() => bar.remove());
    document.body.append(bar);
    draw();

    const refresh = button('Refresh', {
        icon: 'refresh', on: {
            click: async () => {
                refresh.disabled = true;
                await loadWorlds();
                refresh.disabled = false;
            }
        }
    });
    return h('div', {class: 'page'},
        pageHead('Worlds', null, refresh, state.canCommand && button('Select shown', {
            icon: 'check',
            on: {click: selectShown}
        })),
        subtitle,
        h('div', {class: 'toolbar'}, h('div', {class: 'search'}, icon('search'), search), filters),
        !state.canCommand && h('div', {class: 'callout'}, icon('shield'), h('div', {}, 'Rebooting and broadcasting need the admin role. Tap a world to see who\'s on it.')),
        tiles);
}

// Tick time, memory and connections over the last minute.
function healthLine(health) {
    const tone = health.overruns > 0 || health.tick_max_ms > 600 ? 'danger' : health.tick_ms > 300 ? 'warn' : '';
    return h('div', {
            class: 'tile-health',
            title: `Ticks over the last minute: ${health.tick_ms.toFixed(1)} ms on average, ${health.tick_max_ms.toFixed(0)} ms at worst, ${health.overruns} over 600 ms`
        },
        h('span', {class: tone}, `${health.tick_ms < 10 ? health.tick_ms.toFixed(1) : Math.round(health.tick_ms)} ms tick`),
        health.memory_bytes != null && h('span', {}, megabytes(health.memory_bytes)),
        h('span', {}, plural(health.connections, 'conn')));
}

function worldPlayersDialog(world) {
    const list = h('div', {class: 'list'}, h('div', {class: 'empty'}, h('div', {class: 'spinner'})));
    const filterInput = h('input', {
        class: 'input',
        type: 'search',
        placeholder: 'Filter by name',
        'aria-label': 'Filter players'
    });
    let players = [];
    const draw = () => {
        const q = filterInput.value.trim().toLowerCase().replace(/_/g, ' ');
        const shown = players.filter((p) => p.name.toLowerCase().includes(q)).slice(0, 300);
        replace(list, shown.length ? shown.map((p) => playerItem(p, null, close)) : h('div', {class: 'empty'}, 'Nobody matches.'));
    };
    filterInput.addEventListener('input', draw);
    const {close} = openDialog({
        title: `${worldName(world)} players`,
        subtitle: world.region,
        body: [h('div', {class: 'search'}, icon('search'), filterInput), list],
        width: 'min(560px, calc(100vw - 32px))',
    });
    api('GET', `/worlds/${world.node}/players`).then((data) => {
        players = data;
        draw();
    }).catch((e) => replace(list, h('div', {class: 'empty'}, e.message)));
}

// ---------------------------------------------------------------------------
// Players

const STAFF = ['', 'Mod', 'Admin', 'Maintainer'];

function playerBadges(p) {
    return [
        p.staff_level > 0 && h('span', {class: 'badge accent'}, icon('shield'), STAFF[p.staff_level] || 'Staff'),
        p.muted && h('span', {class: 'badge warn'}, icon('mute'), 'Muted'),
    ];
}

function playerItem(p, world, onAct, refresh) {
    const act = (action) => () => {
        onAct?.();
        moderateDialog(p.name, action, refresh);
    };
    return h('div', {class: 'list-item'},
        h('div', {class: 'avatar'}, initial(p.name)),
        h('div', {class: 'list-main'},
            h('div', {class: 'list-title'}, h('b', {}, playerLink(p.name, {on: {click: () => onAct?.()}})), ' ', playerBadges(p)),
            h('div', {class: 'list-sub num'}, world ? `World ${world} · ` : '', `${p.x}, ${p.z}${p.level ? ` · floor ${p.level}` : ''}`)),
        h('div', {class: 'chips'},
            button(null, {
                small: true,
                icon: 'kick',
                'aria-label': `Kick ${p.name}`,
                title: 'Kick',
                on: {click: act('kick')}
            }),
            button(null, {
                small: true,
                icon: 'mute',
                'aria-label': `Mute ${p.name}`,
                title: 'Mute',
                on: {click: act('mute')}
            }),
            !isTrial() && button(null, {
                small: true,
                icon: 'ban',
                'aria-label': `Ban ${p.name}`,
                title: 'Ban',
                on: {click: act('ban')}
            })));
}

function playersPage(ctx) {
    const results = h('div', {class: 'list'});
    const summary = h('span', {class: 'muted'});
    const query = h('input', {
        class: 'input',
        type: 'search',
        placeholder: 'Search players by name',
        autofocus: true,
        'aria-label': 'Search players',
        value: ctx.params.get('q') || ''
    });
    nameSuggest(query, (name) => navigate(`/player?name=${encodeURIComponent(name)}`));
    const offline = h('div', {class: 'list'});
    const offlineCard = h('section', {class: 'card hidden'},
        h('div', {class: 'card-head'}, h('h2', {}, 'Not online'),
            h('div', {class: 'actions'}, h('span', {class: 'muted'}, 'Accounts whose names start with the search'))),
        offline);
    let seq = 0;

    const load = async () => {
        const mine = ++seq;
        const q = query.value.trim();
        try {
            // Accounts that aren't online, with names starting with the query.
            const accounts = validName(q)
                ? api('GET', `/players/names?${new URLSearchParams({q})}`).catch(() => [])
                : Promise.resolve([]);
            const data = await api('GET', `/players?q=${encodeURIComponent(q)}`);
            const away = (await accounts).filter((p) => !p.online_world);
            if (mine !== seq) return;
            offlineCard.classList.toggle('hidden', !away.length);
            replace(offline, away.map((p) => h('div', {class: 'list-item'},
                h('div', {class: 'avatar'}, initial(p.name)),
                h('div', {class: 'list-main'},
                    h('div', {class: 'list-title'}, playerLink(p.name)),
                    h('div', {class: 'list-sub'}, 'Offline')))));
            if (!q) {
                online = new Map(data.players.map((p) => [nameKey(p.name), p.world]));
                drawMine();
            }
            summary.textContent = data.total > data.players.length
                ? `Showing ${data.players.length} of ${plural(data.total, 'player')}`
                : plural(data.total, 'player') + (q ? ' match' : ' online');
            if (data.unreachable.length) summary.textContent += ` · ${plural(data.unreachable.length, 'world')} didn't answer`;
            replace(results, data.players.length
                ? data.players.map((p) => playerItem(p, p.world, null, load))
                : h('div', {class: 'empty'}, icon('users'), q ? `Nobody online matches "${q}".` : 'Nobody is online.',
                    q && validName(q) && !away.length && button(`Look up ${q}`, {
                        small: true,
                        icon: 'search',
                        on: {click: () => navigate(`/player?name=${encodeURIComponent(q)}`)}
                    })));
        } catch (e) {
            if (mine === seq) replace(results, h('div', {class: 'empty'}, e.message));
        }
    };
    const later = debounce(() => {
        // Choosing a suggestion leaves the page before this runs.
        if (!query.isConnected) return;
        const q = query.value.trim();
        history.replaceState(null, '', q ? `/players?q=${encodeURIComponent(q)}` : '/players');
        load();
    }, 250);
    query.addEventListener('input', later);
    replace(results, h('div', {class: 'empty'}, h('div', {class: 'spinner'})));
    load();
    const timer = setInterval(() => {
        if (!document.hidden && !document.querySelector('dialog[open]')) load();
    }, 15000);
    ctx.cleanup(() => clearInterval(timer));

    // Pinned and recently viewed players: whether each is online, from the
    // lists the search just fetched.
    const mineBox = h('div', {class: 'split hidden'});
    // By name key, the world each is on; null until everyone online is known.
    let online = null;
    const drawMine = () => {
        const p = state.prefs || NO_PREFS;
        const row = (name, side) => h('div', {class: 'list-item'},
            h('div', {class: 'avatar'}, initial(name)),
            h('div', {class: 'list-main'},
                h('div', {class: 'list-title'}, playerLink(name)),
                online && h('div', {class: 'list-sub'}, online.has(nameKey(name)) ? `Online on world ${online.get(nameKey(name))}` : 'Offline')),
            side);
        const unpin = (name) => button(null, {
            small: true,
            kind: 'ghost',
            icon: 'x',
            'aria-label': `Unpin ${name}`,
            title: 'Unpin',
            on: {click: () => changePrefs({op: 'unpin', name}).catch((e) => toast(e.message, 'error'))}
        });
        mineBox.classList.toggle('hidden', !p.pinned.length && !p.recent.length);
        replace(mineBox,
            card('Pinned', h('span', {class: 'hint'}, 'Pin from a player\'s page'),
                h('div', {class: 'list'}, p.pinned.length ? p.pinned.map((n) => row(n, unpin(n)))
                    : h('div', {class: 'empty'}, icon('star'), 'Nobody pinned yet.'))),
            card('Recently viewed', p.recent.length > 0 && button('Clear', {
                    small: true,
                    kind: 'ghost',
                    on: {click: () => changePrefs({op: 'forget_recent'}).catch((e) => toast(e.message, 'error'))}
                }),
                h('div', {class: 'list'}, p.recent.length ? p.recent.map((r) => row(r.name, h('span', {class: 'list-side'}, ago(r.at_ms))))
                    : h('div', {class: 'empty'}, 'Players you look up show here.'))));
    };
    watchPrefs(mineBox, drawMine);
    drawMine();

    return h('div', {class: 'page'},
        pageHead('Players', 'Everyone online, across every world. Look up anyone, online or not.',
            button('Moderate by name', {icon: 'shield', on: {click: () => moderateDialog('', 'mute', load)}})),
        h('form', {
                class: 'toolbar', on: {
                    submit: (e) => {
                        e.preventDefault();
                        const q = query.value.trim();
                        if (validName(q)) navigate(`/player?name=${encodeURIComponent(q)}`);
                    }
                }
            },
            h('div', {class: 'search grow'}, icon('search'), query),
            h('button', {type: 'submit', class: 'btn'}, icon('user'), 'Look up')),
        mineBox,
        h('section', {class: 'card'},
            h('div', {class: 'card-head'}, h('h2', {}, 'Online'), h('div', {class: 'actions'}, summary)),
            scrollWindow(results)),
        offlineCard);
}

// ---------------------------------------------------------------------------
// Audit log

const VERBS = {
    sign_in: ['signed in', 'key', 'ok'],
    sign_out: ['signed out', 'logout'],
    sign_in_failed: ['failed to sign in', 'lock', 'danger'],
    sign_in_locked: ['was locked out after repeated failures', 'lock', 'danger'],
    '2fa_enrolled': ['set up two-factor authentication', 'shield', 'ok'],
    reboot: ['scheduled a reboot of', 'power', 'accent'],
    cancel_reboot: ['cancelled the reboot of', 'undo', 'accent'],
    shutdown: ['scheduled a shutdown of', 'power', 'danger'],
    start_world: ['started', 'play', 'accent'],
    broadcast: ['broadcast to', 'megaphone', 'accent'],
    kick: ['kicked', 'kick'],
    mute: ['muted', 'mute'],
    unmute: ['unmuted', 'undo'],
    ban: ['banned', 'ban', 'danger'],
    unban: ['unbanned', 'undo'],
    password_changed: ['changed their password', 'lock'],
    session_ended: ['ended a session', 'logout'],
    maintenance_on: ['turned on maintenance for', 'lock', 'accent'],
    maintenance_off: ['turned off maintenance for', 'unlock', 'accent'],
    chat_viewed: ['viewed the chat logs of', 'chat'],
    records_viewed: ['viewed the records of', 'list'],
    staff_added: ['added the account', 'key', 'accent'],
    staff_role: ['changed the role of', 'key', 'accent'],
    staff_password: ['set a new password for', 'lock'],
    staff_2fa_reset: ['reset two-factor authentication for', 'shield'],
    staff_disabled: ['disabled the account', 'lock', 'danger'],
    staff_enabled: ['enabled the account', 'key'],
    staff_removed: ['removed the account', 'x', 'danger'],
    player_edited: ['changed', 'zap', 'accent'],
    news_posted: ['posted the news', 'news', 'accent'],
    news_edited: ['edited the news', 'edit'],
    news_deleted: ['deleted the news', 'trash', 'danger'],
    news_media_uploaded: ['uploaded an image or video for the news', 'plus'],
    replay_viewed: ['watched a session of', 'play'],
    case_opened: ['opened a case on', 'briefcase', 'accent'],
    case_closed: ['closed a case on', 'check', 'ok'],
    case_reopened: ['reopened a case on', 'undo', 'accent'],
    kill_switch_on: ['switched off', 'zap', 'danger'],
    kill_switch_off: ['switched back on', 'zap', 'accent'],
    lockdown_on: ['locked down logins on', 'lock', 'danger'],
    lockdown_off: ['lifted the login lockdown on', 'unlock', 'accent'],
    schedule_added: ['scheduled', 'clock', 'accent'],
    schedule_removed: ['cancelled the scheduled', 'undo'],
    scheduled_reboot: ['had a scheduled reboot go out,', 'power', 'accent'],
    scheduled_broadcast: ['had a scheduled broadcast go out,', 'megaphone', 'accent'],
    scheduled_missed: ['had a scheduled job skipped,', 'alert', 'danger'],
    templates_saved: ['saved the sanction templates', 'shield'],
    canned_saved: ['saved the canned texts', 'edit'],
    exported: ['exported', 'download'],
    timeline_viewed: ['viewed the timeline of', 'list'],
    addresses_viewed: ['viewed the login addresses of', 'key'],
    appeal_received: ['took in an appeal from', 'flag', 'warn'],
    case_detected: ['opened a case on', 'briefcase', 'danger'],
    case_detection_added: ['added a new detection to the case on', 'briefcase', 'warn'],
    appeal_decided: ['decided the appeal of', 'flag', 'accent'],
    note_added: ['added a staff note on', 'edit'],
    note_removed: ['removed a staff note on', 'trash'],
    session_freed: ['freed the session of', 'unlock', 'accent'],
    sent_home: ['sent home', 'home'],
    password_reset: ['issued a password reset for', 'key', 'accent'],
    items_restored: ['restored items to', 'plus', 'accent'],
    save_rolled_back: ['rolled back the save of', 'undo', 'danger'],
};

function auditItem(e) {
    let [verb, iconName, tone] = VERBS[e.action] || [e.action.replace(/_/g, ' '), 'activity'];
    // Lookups made together share an entry, its detail listing what each showed.
    const looked = e.action === 'records_viewed' && e.detail ? e.detail.split(', ') : null;
    if (looked) verb = `viewed the ${looked.slice(0, -1).join(', ')} and ${looked.at(-1)} of`;
    const failed = !e.ok;
    // Sign-ins under a name that isn't an account have no actor, only the
    // name tried. Anything else without one, the console did itself.
    const title = !e.actor && e.target && e.action.startsWith('sign_in')
        ? (e.action === 'sign_in_locked'
            ? [h('b', {}, e.target), ' was locked out after repeated failed sign-ins']
            : ['Failed sign-in as ', h('b', {}, e.target)])
        : [h('b', {}, e.actor && isMaintainer() ? staffLink(e.actor) : e.actor || 'The console'), ` ${verb}`, e.target ? [' ', h('b', {}, e.target)] : null];
    // A sign-in's detail is the browser's user agent.
    const detail = looked ? null : e.action === 'sign_in' && e.detail ? device(e.detail) : e.detail;
    return h('div', {class: 'list-item'},
        h('div', {class: `list-icon ${failed ? 'danger' : tone || ''}`}, icon(failed ? 'alert' : iconName)),
        h('div', {class: 'list-main'},
            h('div', {class: 'list-title'}, title,
                failed && e.actor && [' ', h('span', {class: 'badge danger'}, 'Failed')]),
            detail && h('div', {class: 'list-sub'}, detail)),
        h('div', {class: 'list-side', title: `${fullTime(e.at_ms)}${e.ip ? ` from ${e.ip}` : ''}`}, ago(e.at_ms)));
}

function auditPage(ctx) {
    const list = h('div', {class: 'list'});
    const more = h('div', {class: 'more hidden'});
    const box = scrollWindow(list, more);
    const search = h('input', {
        class: 'input',
        type: 'search',
        placeholder: 'Search actions, players, details',
        'aria-label': 'Search the audit log',
        value: ctx.params.get('q') || ''
    });
    let mine = ctx.params.get('mine') === '1';
    let last = null;
    let seq = 0;
    const chips = h('div', {class: 'chips'});
    const filter = () => ({q: search.value.trim(), mine: mine ? '1' : ''});
    const filterQuery = () => query({q: filter().q, actor: mine ? state.session.name : null});

    let fetching = false;
    const load = async (append_) => {
        if (append_ && fetching) return;
        const current = ++seq;
        fetching = true;
        if (!append_) {
            box.scrollTop = 0;
            const q = query(filter());
            history.replaceState(null, '', `/audit${q ? `?${q}` : ''}`);
            saved.draw();
        }
        try {
            const entries = await api('GET', `/audit?${query({limit: 50, before: append_ && last ? last : null})}&${filterQuery()}`);
            if (current !== seq) return;
            if (!append_) list.replaceChildren();
            append(list, entries.map(auditItem));
            if (!list.children.length) append(list, h('div', {class: 'empty'}, icon('list'), 'No matching entries.'));
            last = entries.length ? entries[entries.length - 1].id : last;
            more.classList.toggle('hidden', entries.length < 50);
        } catch (e) {
            replace(list, h('div', {class: 'empty'}, e.message));
        } finally {
            if (current === seq) fetching = false;
        }
    };
    const setMine = (on) => {
        mine = on;
        drawChips();
        load();
    };
    const drawChips = () => replace(chips,
        h('button', {type: 'button', class: 'chip', 'aria-pressed': String(!mine), on: {click: () => setMine(false)}}, 'Everyone'),
        h('button', {type: 'button', class: 'chip', 'aria-pressed': String(mine), on: {click: () => setMine(true)}}, 'Only mine'));
    const saved = savedFilters('audit', filter, (params) => {
        search.value = params.q || '';
        setMine(params.mine === '1');
    });
    drawChips();
    search.addEventListener('input', debounce(() => load(), 250));
    append(more, button('Load older', {small: true, on: {click: () => load(true)}}));
    loadOnScroll(box, more, () => load(true));
    load();

    return h('div', {class: 'page'},
        pageHead('Audit log', 'Everything done through the console, sign-ins included.',
            exportButton(() => `/export/audit?${filterQuery()}`, 'Download every entry this search finds (up to 10,000) as a CSV file')),
        h('div', {class: 'toolbar'}, h('div', {class: 'search'}, icon('search'), search), chips),
        saved.el,
        h('section', {class: 'card'}, box));
}

// ---------------------------------------------------------------------------
// Account

function accountPage() {
    const {name, role} = state.session;
    const sessions = h('div', {});
    const endOthers = button('Sign out other sessions', {small: true, kind: 'ghost'});

    const loadSessions = async () => {
        try {
            const account = await api('GET', '/account');
            const others = account.sessions.filter((s) => !s.current).length;
            endOthers.classList.toggle('hidden', others === 0);
            replace(sessions, account.sessions.map((s) => h('div', {class: 'session-row'},
                h('div', {class: 'list-icon'}, icon('monitor')),
                h('div', {class: 'list-main'},
                    h('div', {class: 'list-title'}, h('b', {}, device(s.user_agent)), ' ', s.current && h('span', {class: 'badge ok'}, 'This device')),
                    h('div', {class: 'list-sub'}, `${s.ip} · signed in ${ago(s.created_ms)} · active ${ago(s.last_seen_ms)}`)),
                !s.current && button('Sign out', {
                    small: true, on: {
                        click: async () => {
                            try {
                                await api('POST', `/account/sessions/${s.id}/end`);
                                toast('Session signed out.');
                                loadSessions();
                            } catch (e) {
                                toast(e.message, 'error');
                            }
                        }
                    }
                }))));
        } catch (e) {
            replace(sessions, h('div', {class: 'empty'}, e.message));
        }
    };
    endOthers.addEventListener('click', async () => {
        try {
            const result = await api('POST', '/account/sessions/end-others');
            toast(`Signed out ${plural(result.ended, 'other session')}.`);
            loadSessions();
        } catch (e) {
            toast(e.message, 'error');
        }
    });
    loadSessions();

    const current = h('input', {class: 'input', type: 'password', autocomplete: 'current-password', required: true});
    const next = h('input', {
        class: 'input',
        type: 'password',
        autocomplete: 'new-password',
        minlength: '12',
        required: true
    });
    const again = h('input', {class: 'input', type: 'password', autocomplete: 'new-password', required: true});
    const code = h('input', {
        class: 'input',
        inputmode: 'numeric',
        autocomplete: 'one-time-code',
        maxlength: '6',
        pattern: '[0-9]{6}',
        required: true,
        placeholder: '000000'
    });
    const error = h('p', {class: 'error-text', role: 'alert'});
    const form = h('form', {class: 'form-grid'},
        h('label', {class: 'field'}, h('span', {}, 'Current password'), current),
        h('label', {class: 'field'}, h('span', {}, 'New password'), next, h('span', {class: 'hint'}, 'At least 12 characters. A passphrase works well.')),
        h('label', {class: 'field'}, h('span', {}, 'New password again'), again),
        h('label', {class: 'field'}, h('span', {}, 'Authenticator code'), code),
        error,
        h('div', {}, h('button', {type: 'submit', class: 'btn primary'}, 'Change password')));
    form.addEventListener('submit', async (e) => {
        e.preventDefault();
        error.textContent = '';
        if (next.value !== again.value) {
            error.textContent = 'The new passwords don\'t match.';
            return;
        }
        const submit = form.querySelector('button[type=submit]');
        submit.disabled = true;
        try {
            await api('POST', '/account/password', {current: current.value, new: next.value, code: code.value});
            form.reset();
            toast('Password changed. Your other sessions were signed out.');
            loadSessions();
        } catch (err) {
            error.textContent = err.message;
        } finally {
            submit.disabled = false;
        }
    });

    const themes = h('div', {class: 'segmented', role: 'group', 'aria-label': 'Theme'});
    const drawThemes = () => replace(themes, ['auto', 'light', 'dark'].map((t) =>
        h('button', {
                type: 'button', 'aria-pressed': String(prefs.get('theme', 'dark') === t), on: {
                    click: () => {
                        prefs.set('theme', t);
                        applyTheme(t);
                        drawThemes();
                    }
                }
            },
            t === 'auto' ? 'System' : t[0].toUpperCase() + t.slice(1))));
    drawThemes();
    const zones = h('div', {class: 'segmented', role: 'group', 'aria-label': 'Times'});
    const drawZones = () => replace(zones, [[false, `Your zone (${Intl.DateTimeFormat().resolvedOptions().timeZone})`], [true, 'UTC']].map(([utc, label]) =>
        h('button', {
            type: 'button',
            'aria-pressed': String(inUtc() === utc),
            on: {click: () => changePrefs({op: 'utc', utc}).catch((e) => toast(e.message, 'error'))}
        }, label)));
    drawZones();

    return h('div', {class: 'page'},
        pageHead('Account', null, button('Sign out', {icon: 'logout', on: {click: signOut}})),
        h('section', {class: 'card'},
            h('div', {class: 'card-body'},
                h('div', {class: 'list-item'},
                    h('div', {class: 'avatar'}, initial(name)),
                    h('div', {class: 'list-main'},
                        h('div', {class: 'list-title'}, h('b', {}, name), ' ', h('span', {class: 'badge accent'}, ROLE_NAMES[role] || role)),
                        h('div', {class: 'list-sub'}, {
                            maintainer: 'Can do everything, including managing staff accounts.',
                            admin: 'Can reboot, broadcast, toggle maintenance, kick, mute and ban.',
                            moderator: 'Can kick, mute and ban. World controls need the admin role.',
                            trial: 'A trial moderator: can kick, and mute for up to 2 days. Bans, lifting sanctions, bulk dismissing and appeal decisions need a moderator.',
                        }[role])),
                    h('span', {class: 'badge ok'}, icon('shield'), '2FA on')))),
        h('div', {class: 'split'},
            h('section', {class: 'card'},
                h('div', {class: 'card-head'}, h('h2', {}, 'Sessions'), h('div', {class: 'actions'}, endOthers)),
                sessions),
            h('section', {class: 'card'},
                h('div', {class: 'card-head'}, h('h2', {}, 'Appearance')),
                h('div', {class: 'card-body column'},
                    h('div', {class: 'field'}, h('span', {}, 'Theme'), themes),
                    h('div', {class: 'field'}, h('span', {}, 'Times'), zones,
                        h('p', {class: 'hint'}, 'Every time in the console shows in this zone. It follows your account to any browser.'))))),
        h('section', {class: 'card'},
            h('div', {class: 'card-head'}, h('h2', {}, 'Change password')),
            h('div', {class: 'card-body'}, form)));
}

// ---------------------------------------------------------------------------
// Lookups shared by the player, moderation and chat pages

const MOD_VERBS = {
    mute: ['muted', 'mute', 'warn'],
    unmute: ['unmuted', 'undo', ''],
    ban: ['banned', 'ban', 'danger'],
    unban: ['unbanned', 'undo', ''],
    kick: ['kicked', 'kick', ''],
    unstick: ['freed the session of', 'unlock', 'accent'],
    send_home: ['sent home', 'home', ''],
    password_reset: ['issued a password reset for', 'key', 'accent'],
    restore: ['restored items to', 'plus', 'accent'],
    rollback: ['rolled back the save of', 'undo', 'danger'],
    hiscores: ['changed the hiscores of', 'chart', ''],
};

// `336` -> "2 weeks"; the named lengths as the pickers name them.
const hoursLabel = (hours) => {
    const named = DURATIONS.find(([h0]) => h0 === hours)?.[1].toLowerCase();
    if (named) return named;
    if (hours % 8760 === 0) return plural(hours / 8760, 'year');
    if (hours % 168 === 0) return plural(hours / 168, 'week');
    if (hours % 24 === 0) return plural(hours / 24, 'day');
    return plural(hours, 'hour');
};

function moderationItem(m) {
    const [verb, iconName, tone] = MOD_VERBS[m.action] || [m.action, 'shield', ''];
    return h('div', {class: 'list-item'},
        h('div', {class: `list-icon ${tone}`}, icon(iconName)),
        h('div', {class: 'list-main'},
            h('div', {class: 'list-title'}, h('b', {}, playerLink(m.moderator)), ` ${verb} `, h('b', {}, playerLink(m.target)),
                m.hours ? ` for ${hoursLabel(m.hours)}` : ''),
            h('div', {class: 'list-sub'}, m.world ? `World ${m.world}` : `Node ${m.node}`, m.reason ? ` · ${m.reason}` : '')),
        h('div', {class: 'list-side', title: fullTime(m.at_ms)}, ago(m.at_ms)));
}

function chatItem(c) {
    return h('div', {class: 'chat-row'},
        h('span', {class: 'chat-time', title: fullTime(c.at_ms)}, shortTime(c.at_ms)),
        h('span', {
            class: 'chat-world',
            title: `${c.x}, ${c.z}${c.level ? `, floor ${c.level}` : ''}`
        }, c.world ? `W${c.world}` : `N${c.node}`),
        h('span', {class: 'chat-who'}, playerLink(c.from),
            c.to && [h('span', {class: 'muted'}, ' to '), playerLink(c.to)]),
        h('span', {class: `chat-text ${c.to ? 'private' : ''}`}, c.message));
}

// A list that loads a page at a time, newest first, with "Load older".
// A box a long list scrolls in, so it doesn't run down the page. The wheel
// stays in it only while it has something to scroll: over a short list, the
// page scrolls.
function scrollWindow(...children) {
    const box = h('div', {class: 'scroll-window'}, ...children);
    const sync = () => box.classList.toggle('overflows', box.scrollHeight > box.clientHeight + 1);
    const sizes = new ResizeObserver(sync);
    sizes.observe(box);
    for (const child of children.flat()) if (child instanceof Element) sizes.observe(child);
    return box;
}

// A scroll window for a wide table: it scrolls sideways too, and its header
// row stays in view.
function tableWindow(...children) {
    const box = scrollWindow(...children);
    box.classList.add('table-wrap');
    return box;
}

// Calls `load` as `more` (in scroll window `box`) comes into view, unless
// it's hidden: older rows load as the bottom is reached.
function loadOnScroll(box, more, load) {
    new IntersectionObserver((seen) => {
        if (seen.some((s) => s.isIntersecting) && !more.classList.contains('hidden')) load();
    }, {root: box, rootMargin: '200px'}).observe(more);
}

function pagedList(fetchPage, item, emptyText, cursor = (row) => row.id) {
    const list = h('div', {class: 'list'}, h('div', {class: 'empty'}, h('div', {class: 'spinner'})));
    const more = h('div', {class: 'more hidden'});
    const box = scrollWindow(list, more);
    let last = null;
    let seq = 0;
    let fetching = false;
    const load = async (older) => {
        if (older && fetching) return;
        const mine = ++seq;
        if (!older) box.scrollTop = 0;
        fetching = true;
        try {
            const rows = await fetchPage(older ? last : null);
            if (mine !== seq) return;
            if (!older) list.replaceChildren();
            append(list, rows.map((row) => item(row)));
            if (!list.children.length) append(list, h('div', {class: 'empty'}, emptyText));
            if (rows.length) last = cursor(rows[rows.length - 1]);
            more.classList.toggle('hidden', rows.length < 50);
        } catch (e) {
            if (mine === seq) replace(list, h('div', {class: 'empty'}, e.message));
        } finally {
            if (mine === seq) fetching = false;
        }
    };
    append(more, button('Load older', {small: true, on: {click: () => load(true)}}));
    loadOnScroll(box, more, () => load(true));
    load(false);
    return {el: box, list, reload: () => load(false)};
}

const query = (params) => {
    const q = new URLSearchParams();
    for (const [k, v] of Object.entries(params)) if (v !== null && v !== undefined && v !== '') q.set(k, String(v));
    return q.toString();
};

// ---------------------------------------------------------------------------
// Player

function playerPage(ctx) {
    const name = (ctx.params.get('name') || '').trim();
    const page = h('div', {class: 'page'});
    if (!validName(name)) {
        append(page, pageHead('Player', null), h('div', {class: 'empty'}, icon('user'), 'That isn\'t a valid player name.'));
        return page;
    }
    const head = h('div', {}, pageHead(name, 'Looking them up...'));
    const facts = h('div', {class: 'stats'});
    const support = supportSection(name);
    const fact = (iconName, label, value, sub) => h('div', {class: 'card stat'},
        h('div', {class: 'label'}, icon(iconName), label), h('div', {class: 'value'}, value), h('div', {class: 'sub'}, sub));

    // Their sessions and logs, once it's known whether they're staff above
    // the one looking.
    const logsTop = h('div', {class: 'column'});
    const logsBottom = h('div', {class: 'column'});
    let logsShown = false;
    const showLogs = (allowed) => {
        if (logsShown) return;
        logsShown = true;
        const mods = pagedList(
            (before) => api('GET', `/lookup/moderation?${query({name, before})}`),
            moderationItem, 'No moderation on record.');
        const modsCard = h('section', {class: 'card'},
            h('div', {class: 'card-head'}, h('h2', {}, 'Moderation history')),
            mods.el);
        if (!allowed) {
            replace(logsTop, h('div', {class: 'callout'}, icon('lock'),
                h('div', {}, `${name} is staff above your role, so their sessions, chat, timeline, progress and login addresses are hidden from you.`)));
            replace(logsBottom, modsCard);
            return;
        }
        const chat = pagedList(
            (before) => api('GET', `/lookup/chat?${query({name, before})}`),
            chatItem, 'No chat on record.');
        const sessions = pagedList(
            (before) => api('GET', `/replays?${query({name, before})}`).then((rows) => {
                if (!before) dockWarm(rows);
                return rows;
            }),
            replayItem, 'No recorded sessions.', (r) => r.started_ms);
        replace(logsTop, progressSection(name), economySection(name), timelineCard(name), linksSection(name, load));
        replace(logsBottom,
            h('section', {class: 'card'},
                h('div', {class: 'card-head'}, h('h2', {}, 'Recorded sessions'),
                    h('div', {class: 'actions'}, link(`/replays?${query({name})}`, {class: 'btn small ghost'}, 'All their sessions', icon('chevron')))),
                sessions.el),
            h('div', {class: 'split'},
                h('section', {class: 'card'},
                    h('div', {class: 'card-head'}, h('h2', {}, 'Chat'),
                        h('div', {class: 'actions'}, link(`/chat?${query({name})}`, {class: 'btn small ghost'}, 'All chat logs', icon('chevron')))),
                    h('div', {class: 'chat-list'}, chat.el)),
                modsCard));
    };

    const load = async () => {
        let p;
        try {
            p = await api('GET', `/lookup/player?${query({name})}`);
        } catch (e) {
            replace(head, pageHead(name, e.message));
            showLogs(true);
            return;
        }
        showLogs(!p || mayWatch(p.staff_level));
        if (!p) {
            replace(head, pageHead(name, 'No registered player has this name.'));
            facts.classList.add('hidden');
            return;
        }
        document.title = `${p.name} · RS Admin`;
        markTitle();
        const muted = p.muted_until_ms != null;
        const banned = p.banned_until_ms != null;
        const act = (action) => () => moderateDialog(p.name, action, load);
        support.update(p);
        replace(head, pageHead(h('span', {}, p.name, ' ', playerBadges({staff_level: p.staff_level})),
            p.online_node != null ? `Online on ${p.online_world ? `world ${p.online_world}` : `node ${p.online_node}`}` : 'Offline',
            p.online_node != null && mayWatch(p.staff_level) && button('Watch live', {icon: 'radio', on: {click: () => watchLive(p.name)}}),
            p.online_node != null && button('Kick', {icon: 'kick', on: {click: act('kick')}}),
            !(muted && isTrial()) && button(muted ? 'Unmute' : 'Mute', {
                icon: muted ? 'undo' : 'mute',
                on: {click: act(muted ? 'unmute' : 'mute')}
            }),
            !isTrial() && button(banned ? 'Unban' : 'Ban', {
                icon: banned ? 'undo' : 'ban',
                kind: banned ? undefined : 'danger',
                on: {click: act(banned ? 'unban' : 'ban')}
            }),
            button('Open case', {icon: 'briefcase', on: {click: () => newCaseDialog({player: p.name})}}),
            pinButton(p.name)));
        changePrefs({op: 'viewed', name: p.name}).catch(() => {
        });
        const seen = p.online_node != null
            ? fact('activity', 'Status', 'Online', `${p.online_world ? `World ${p.online_world}` : `Node ${p.online_node}`}${p.online_since_ms ? ` since ${ago(p.online_since_ms)}` : ''}`)
            : fact('clock', 'Last seen', p.last_seen_ms ? ago(p.last_seen_ms) : 'Never', p.last_seen_ms ? `${p.last_world ? `World ${p.last_world}` : 'Logged out'} · ${fullTime(p.last_seen_ms)}` : 'No logout on record');
        replace(facts,
            seen,
            fact('user', 'Registered', dateOnly(p.registered_ms), ago(p.registered_ms)),
            fact('mute', 'Mute', muted ? h('span', {class: 'tone-warn'}, 'Muted') : 'None', muted ? `${until(p.muted_until_ms)} · ends ${fullTime(p.muted_until_ms)}` : 'Can chat'),
            fact('ban', 'Ban', banned ? h('span', {class: 'tone-danger'}, 'Banned') : 'None', banned ? `${until(p.banned_until_ms)} · ends ${fullTime(p.banned_until_ms)}` : 'Can log in'));
    };
    load();

    const cases = spinnerList();
    const loadCases = () => api('GET', `/cases?${query({player: name, status: 'all'})}`).then((rows) =>
        replace(cases, rows.length ? rows.map((c) => caseItem(c)) : h('div', {class: 'empty'}, icon('briefcase'), 'No cases on them.')))
        .catch((e) => failed(cases, e));
    loadCases();
    const reports = pagedList(
        (before) => api('GET', `/lookup/reports?${query({name, before})}`),
        (r) => reportItem(r, () => {
            reports.reload();
            loadCases();
        }), 'No abuse reports against them.');

    const watched = h('div', {class: 'hidden'}, h('div', {class: 'callout warn'}, icon('eye'),
        h('div', {}, 'On the staff watchlist. See the notes below for why.')));
    append(page, head, watched, facts,
        notesSection(name, (on) => watched.classList.toggle('hidden', !on)),
        support.el,
        isMaintainer() && editSection(name),
        logsTop,
        h('div', {class: 'split'},
            card('Cases', button('Open case', {
                small: true,
                icon: 'briefcase',
                on: {click: () => newCaseDialog({player: name})}
            }), cases),
            card('Abuse reports against them', link(`/moderation?${query({
                tab: 'triage',
                all: 1,
                name
            })}`, {class: 'btn small ghost'}, 'In the queue', icon('chevron')), reports.el)),
        logsBottom);
    return page;
}

// ---------------------------------------------------------------------------
// Charts on the player page: XP and wealth at each save point kept, and
// when they play.

// `n` short: 1.2M, 34K.
const compact = (n) => new Intl.NumberFormat([], {notation: 'compact', maximumFractionDigits: 1}).format(n);
const POINT_LABELS = {logout: 'Logout', session: 'During a session', staff: 'Before a staff change', now: 'Now'};

// A line of `key` over time across `points`, with the value under the
// pointer shown below it.
function lineChart(points, key, title, format) {
    const W = 600;
    const H = 150;
    const PAD = 6;
    const values = points.map((p) => p[key]);
    const lo = Math.min(...values);
    const hi = Math.max(...values);
    const t0 = points[0].at_ms;
    const t1 = points[points.length - 1].at_ms;
    const x = (t) => PAD + ((t - t0) / Math.max(1, t1 - t0)) * (W - 2 * PAD);
    const y = (v) => H - PAD - ((v - lo) / Math.max(1, hi - lo)) * (H - 2 * PAD);
    const xy = points.map((p) => [x(p.at_ms), y(p[key])]);
    const svg = document.createElementNS(SVG, 'svg');
    svg.setAttribute('viewBox', `0 0 ${W} ${H}`);
    svg.setAttribute('preserveAspectRatio', 'none');
    svg.setAttribute('role', 'img');
    svg.setAttribute('aria-label', `${title}: ${format(values[0])} to ${format(values[values.length - 1])}`);
    const make = (tag, attrs) => {
        const el = document.createElementNS(SVG, tag);
        for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
        svg.append(el);
        return el;
    };
    const line = xy.map(([px, py], i) => `${i ? 'L' : 'M'}${px.toFixed(1)} ${py.toFixed(1)}`).join(' ');
    make('path', {d: `${line} L${xy[xy.length - 1][0].toFixed(1)} ${H} L${xy[0][0].toFixed(1)} ${H} Z`, class: 'chart-area'});
    make('path', {d: line, class: 'chart-line', 'vector-effect': 'non-scaling-stroke'});
    const cursor = make('line', {y1: 0, y2: H, class: 'chart-cursor hidden', 'vector-effect': 'non-scaling-stroke'});
    const last = values[values.length - 1];
    const change = last - values[0];
    const readout = h('span', {class: 'muted'});
    const at = (i) => {
        const p = points[i];
        replace(readout, h('b', {class: 'num'}, format(p[key])), ` · ${POINT_LABELS[p.kind] || p.kind}, ${fullTime(p.at_ms)}`);
    };
    const rest = () => {
        cursor.classList.add('hidden');
        replace(readout, `${change >= 0 ? '+' : '-'}${format(Math.abs(change))} since ${dateOnly(t0)}`);
    };
    svg.addEventListener('pointermove', (e) => {
        const box = svg.getBoundingClientRect();
        const px = ((e.clientX - box.left) / box.width) * W;
        let best = 0;
        xy.forEach(([qx], i) => {
            if (Math.abs(qx - px) < Math.abs(xy[best][0] - px)) best = i;
        });
        cursor.setAttribute('x1', xy[best][0]);
        cursor.setAttribute('x2', xy[best][0]);
        cursor.classList.remove('hidden');
        at(best);
    });
    svg.addEventListener('pointerleave', rest);
    rest();
    return h('div', {class: 'chart'},
        h('div', {class: 'chart-head'}, h('span', {class: 'label'}, title), h('b', {class: 'num'}, format(last))),
        h('div', {class: 'chart-plot'},
            h('div', {class: 'chart-axis num', 'aria-hidden': 'true'}, h('span', {}, format(hi)), h('span', {}, format(lo))), svg),
        h('div', {class: 'chart-foot'}, readout));
}

const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

// Events by weekday and hour, in the zone times are shown in.
function heatmap(hours, days) {
    const grid = WEEKDAYS.map(() => new Array(24).fill(0));
    let total = 0;
    for (const {hour_ms, events} of hours) {
        const [d, hr] = weekHour(hour_ms);
        grid[d][hr] += events;
        total += events;
    }
    const top = Math.max(1, ...grid.flat());
    let peak = [0, 0];
    grid.forEach((row, d) => row.forEach((n, hr) => {
        if (n > grid[peak[0]][peak[1]]) peak = [d, hr];
    }));
    const hourLabel = (hr) => `${String(hr).padStart(2, '0')}:00`;
    const zone = inUtc() ? 'UTC' : 'your time';
    return h('div', {class: 'chart'},
        h('div', {class: 'chart-head'}, h('span', {class: 'label'}, `When they play · last ${days} days`),
            h('b', {class: 'num'}, plural(total, 'event'))),
        h('div', {class: 'heat', role: 'img', 'aria-label': total ? `Most active ${WEEKDAYS[peak[0]]} ${hourLabel(peak[1])} ${zone}` : 'No activity'},
            h('span', {}),
            Array.from({length: 24}, (_, hr) => h('span', {class: 'heat-hour num'}, hr % 6 === 0 ? hourLabel(hr).slice(0, 2) : '')),
            grid.map((row, d) => [
                h('span', {class: 'heat-day'}, WEEKDAYS[d]),
                row.map((n, hr) => h('span', {
                    class: `heat-cell${n ? ' on' : ''}`,
                    title: `${WEEKDAYS[d]} ${hourLabel(hr)} ${zone}: ${plural(n, 'event')}`,
                    style: n ? {opacity: String(0.2 + (0.8 * n) / top)} : null,
                })),
            ])),
        h('div', {class: 'chart-foot muted'}, total
            ? `Busiest: ${WEEKDAYS[peak[0]]} around ${hourLabel(peak[1])} (${zone}). Counts what their scripts logged, such as banking and drops.`
            : 'Their scripts logged nothing in this time.'));
}

function progressSection(name) {
    const DAYS = 90;
    const body = h('div', {class: 'card-body charts'}, h('div', {class: 'spinner'}));
    const draw = (progress, hours) => {
        const charts = [];
        if (progress.status === 'rejected') charts.push(h('div', {class: 'empty'}, progress.reason.message));
        else if (progress.value.length < 2) charts.push(h('div', {class: 'empty'}, icon('chart'), 'Not enough saves kept yet to chart their progress.'));
        else {
            charts.push(lineChart(progress.value, 'xp', 'Total XP', (n) => compact(n)));
            charts.push(lineChart(progress.value, 'wealth', 'Wealth', (n) => `${compact(n)} gp`));
        }
        charts.push(hours.status === 'rejected' ? h('div', {class: 'empty'}, hours.reason.message) : heatmap(hours.value, DAYS));
        replace(body, charts);
    };
    // A change of zone draws the page again, heatmap and all.
    Promise.allSettled([
        api('GET', `/lookup/progress?${query({name})}`),
        api('GET', `/lookup/activity?${query({name, days: DAYS})}`),
    ]).then(([progress, hours]) => draw(progress, hours));
    return card('Progress', h('span', {class: 'hint hide-mobile'}, 'From the saves kept for rollbacks; wealth at item values'), body);
}

// ---------------------------------------------------------------------------
// Player editing on the player page: stats, items and position, changed
// live while they're logged in, or in their save while they're not.

const INV_LABELS = {inv: 'Backpack', worn: 'Worn', bank: 'Bank'};
const invLabel = (name) => INV_LABELS[name] || name;

async function editPlayer(name, edit, onDone) {
    const result = await api('POST', '/players/edit', {name, edit});
    if (!result.ok) throw new Error(result.message);
    toast(result.message);
    onDone();
}

function setStatDialog(name, stat, onDone) {
    const level = h('input', {
        class: 'input',
        type: 'number',
        min: '1',
        max: '99',
        step: '1',
        required: true,
        autofocus: true,
        value: String(stat.base),
        inputmode: 'numeric',
        'data-invalid': 'Levels are 1 to 99.'
    });
    openDialog({
        title: `${stat.name}: ${name}`,
        subtitle: `Level ${stat.base}, ${stat.xp.toLocaleString()} xp now.`,
        body: [h('label', {class: 'field'}, h('span', {}, 'Level'), level,
            h('span', {class: 'hint'}, 'Sets the level and its experience, as ::setstat does.'))],
        actions: [{label: 'Cancel'}, {
            label: 'Set level',
            kind: 'primary',
            run: () => editPlayer(name, {kind: 'set_stat', stat: stat.id, level: Number(level.value)}, onDone),
        }],
    });
}

function giveDialog(name, sheet, onDone) {
    const targets = sheet.invs.filter((i) => i.name !== 'worn').map((i) => i.name);
    let inv = targets.includes('inv') ? 'inv' : targets[0];
    const invChips = h('div', {class: 'segmented'});
    const drawInvs = () => replace(invChips, targets.map((t) =>
        h('button', {
            type: 'button', 'aria-pressed': String(inv === t), on: {
                click: () => {
                    inv = t;
                    drawInvs();
                }
            }
        }, invLabel(t))));
    drawInvs();
    const {search, results, chosen} = objPicker();
    const count = h('input', {
        class: 'input',
        type: 'number',
        min: '1',
        max: '2147483647',
        step: '1',
        value: '1',
        required: true,
        inputmode: 'numeric',
        'data-invalid': 'Counts are 1 to 2147483647.'
    });
    openDialog({
        title: `Give an item: ${name}`,
        subtitle: sheet.online ? 'They get it straight away.' : 'Goes into their save; they\'ll have it at their next login.',
        body: [
            h('div', {class: 'field'}, h('span', {}, 'Into'), invChips),
            h('label', {class: 'field'}, h('span', {}, 'Item'), search, results),
            h('label', {class: 'field'}, h('span', {}, 'Count'), count),
        ],
        actions: [{label: 'Cancel'}, {
            label: 'Give',
            kind: 'primary',
            icon: 'plus',
            run: () => editPlayer(name, {kind: 'give', inv, obj: chosen().id, count: Number(count.value)}, onDone),
        }],
    });
}

function takeDialog(name, inv, item, onDone) {
    const count = h('input', {
        class: 'input',
        type: 'number',
        min: '1',
        max: '2147483647',
        step: '1',
        value: String(item.count),
        required: true,
        autofocus: true,
        inputmode: 'numeric',
        'data-invalid': 'Counts are 1 to 2147483647.'
    });
    openDialog({
        title: `Take ${item.name}: ${name}`,
        subtitle: `${item.count.toLocaleString()} in ${invLabel(inv.name).toLowerCase()} slot ${item.slot}.`,
        body: [h('label', {class: 'field'}, h('span', {}, 'Count'), count)],
        actions: [{label: 'Cancel'}, {
            label: 'Take',
            kind: 'danger',
            icon: 'trash',
            run: () => editPlayer(name, {
                kind: 'take',
                inv: inv.name,
                obj: item.id,
                count: Number(count.value)
            }, onDone),
        }],
    });
}

function moveDialog(name, sheet, onDone) {
    const coord = (label, value, max) => h('input', {
        class: 'input',
        type: 'number',
        min: '0',
        max: String(max),
        step: '1',
        value: String(value),
        required: true,
        inputmode: 'numeric',
        'aria-label': label,
        'data-invalid': `${label} is 0 to ${max}.`
    });
    const x = coord('X', sheet.x, 16383);
    const z = coord('Z', sheet.z, 16383);
    const level = coord('Level', sheet.level, 3);
    openDialog({
        title: `Move: ${name}`,
        subtitle: sheet.online ? 'Teleports them now.' : 'Changes where they log in.',
        body: [h('div', {class: 'coord-grid'},
            h('label', {class: 'field'}, h('span', {}, 'X'), x),
            h('label', {class: 'field'}, h('span', {}, 'Z'), z),
            h('label', {class: 'field'}, h('span', {}, 'Level'), level))],
        actions: [{label: 'Cancel'}, {
            label: 'Move',
            kind: 'primary',
            icon: 'pin',
            run: () => editPlayer(name, {
                kind: 'move',
                x: Number(x.value),
                z: Number(z.value),
                level: Number(level.value)
            }, onDone),
        }],
    });
}

function editSection(name) {
    const body = h('div', {class: 'card-body'}, h('div', {class: 'spinner'}));
    const where = h('span', {class: 'badge'});
    const actions = h('div', {class: 'actions'});
    const load = async () => {
        let sheet;
        try {
            sheet = await api('GET', `/players/sheet?${query({name})}`);
        } catch (e) {
            replace(body, h('p', {class: 'error-text'}, e.message));
            return;
        }
        if (!sheet) {
            replace(body, h('p', {class: 'muted'}, 'No registered player has this name.'));
            return;
        }
        where.className = `badge ${sheet.online ? 'ok' : ''}`;
        replace(where, sheet.online ? `Live on world ${sheet.world ?? '?'}` : 'From their save');
        replace(actions,
            button('Give item', {small: true, icon: 'plus', on: {click: () => giveDialog(sheet.name, sheet, load)}}),
            button('Move', {small: true, icon: 'pin', on: {click: () => moveDialog(sheet.name, sheet, load)}}),
            button(null, {small: true, icon: 'refresh', 'aria-label': 'Reload', title: 'Reload', on: {click: load}}));
        const total = sheet.stats.reduce((sum, s) => sum + s.base, 0);
        replace(body,
            h('p', {class: 'hint'}, `At ${sheet.x}, ${sheet.z}${sheet.level ? `, level ${sheet.level}` : ''} · total level ${total}. Click a stat to set it, or an item to take it.`),
            h('div', {class: 'skill-grid'}, sheet.stats.map((s) =>
                h('button', {
                        type: 'button',
                        class: 'skill',
                        title: `${s.xp.toLocaleString()} xp`,
                        on: {click: () => setStatDialog(sheet.name, s, load)}
                    },
                    h('span', {class: 'skill-name'}, s.name),
                    h('span', {class: 'num'}, s.level === s.base ? String(s.base) : `${s.level}/${s.base}`)))),
            sheet.invs.map((inv) => h('div', {class: 'inv'},
                h('div', {class: 'inv-head'}, h('b', {}, invLabel(inv.name)), h('span', {class: 'muted num'}, `${inv.items.length} / ${inv.size}`)),
                inv.items.length
                    ? h('div', {class: 'item-grid'}, inv.items.map((item) =>
                        h('button', {
                                type: 'button',
                                class: 'item',
                                title: `Slot ${item.slot} · id ${item.id}`,
                                on: {click: () => takeDialog(sheet.name, inv, item, load)}
                            },
                            h('span', {class: 'item-name'}, item.name),
                            item.count > 1 && h('span', {class: 'num muted'}, `x${item.count.toLocaleString()}`))))
                    : h('p', {class: 'hint'}, 'Empty.'))));
    };
    load();
    return h('section', {class: 'card'},
        h('div', {class: 'card-head wraps'}, h('h2', {}, 'Stats and items'), where, actions),
        body);
}

// ---------------------------------------------------------------------------
// Player support on the player page: a player who can't log in (their
// session, their password), is stuck in the game, or lost items to a bug
// or a dupe. Restorations and rollbacks need an open case, and every action
// is audited.

const POINT_KINDS = {
    logout: ['Logout', 'logout'],
    session: ['During a session', 'clock'],
    staff: ['Before a staff change', 'shield'],
};
// The wealth events that show a player came to have an item (or had it,
// for a drop).
const GAINED_WEALTH = [5, 6, 10, 103, 104];

const worldLabel = (node, world) => (world ? `world ${world}` : `node ${node}`);

// The case a support action is for: the player's open cases, `caseId`
// picked when given. `required` for restorations and rollbacks.
function casePicker(name, caseId, required) {
    const select = h('select', {
        class: 'input',
        required,
        'aria-label': 'Case',
        'data-invalid': 'Pick the case this is for.'
    });
    replace(select, h('option', {value: ''}, 'Loading their cases...'));
    api('GET', `/cases?${query({player: name, status: 'open'})}`).then((rows) => {
        const options = rows.map((c) => h('option', {value: String(c.id)}, `#${c.id} ${c.title}`));
        if (caseId && !rows.some((c) => c.id === caseId)) options.unshift(h('option', {value: String(caseId)}, `#${caseId}`));
        replace(select, h('option', {value: ''}, required ? (options.length ? 'Pick a case' : 'No open cases on them') : 'None'), options);
        // Their only open case is the one it's for.
        if (caseId) select.value = String(caseId);
        else if (required && rows.length === 1) select.value = String(rows[0].id);
    }).catch((e) => replace(select, h('option', {value: ''}, e.message)));
    const field = h('label', {class: 'field'}, h('span', {}, required ? 'Case' : 'Case (optional)'), select,
        h('span', {class: 'hint'}, required ? 'Needed for this, and noted in its timeline. Open a case on them first if there isn\'t one.' : 'Noted in its timeline.'));
    return {field, value: () => (select.value ? Number(select.value) : null)};
}

const reasonField = (required, placeholder) => {
    const input = h('textarea', {class: 'input', maxlength: '300', required, placeholder, 'data-invalid': 'Say why.'});
    return {
        field: h('label', {class: 'field'}, h('span', {}, required ? 'Reason' : 'Reason (optional)'), input),
        value: () => input.value.trim()
    };
};

function sendHomeDialog(name, online, onDone) {
    const why = reasonField(false, 'e.g. Stuck behind the wall at the Barbarian Village mine.');
    const picked = casePicker(name, null, false);
    openDialog({
        title: `Send home: ${name}`,
        subtitle: online ? 'Teleports them now.' : 'Changes where they log in.',
        body: [
            h('div', {class: 'callout'}, icon('home'), h('div', {}, 'To Lumbridge, where players respawn. A player still on Tutorial Island goes back to its start instead, so they can\'t skip it.')),
            why.field, picked.field,
        ],
        actions: [{label: 'Cancel'}, {
            label: 'Send home',
            kind: 'primary',
            icon: 'home',
            run: async () => {
                const result = await api('POST', '/support/send-home', {
                    name,
                    reason: why.value(),
                    case: picked.value()
                });
                if (!result.ok) throw new Error(result.message);
                toast(result.message);
                onDone?.();
            },
        }],
    });
}

function unstickDialog(name, message, onDone) {
    const picked = casePicker(name, null, false);
    const why = reasonField(false, 'e.g. Says they get "already logged in" on every world.');
    openDialog({
        title: `Free session: ${name}`,
        subtitle: 'Only what nothing needs is let go: it never releases a player who is online, or whose newest save hasn\'t reached the database.',
        body: [h('div', {class: 'callout'}, icon('unlock'), h('div', {}, message)), why.field, picked.field],
        actions: [{label: 'Cancel'}, {
            label: 'Free session',
            kind: 'primary',
            icon: 'unlock',
            run: async () => {
                const result = await api('POST', '/support/unstick', {name, reason: why.value(), case: picked.value()});
                if (!result.ok) throw new Error(result.message);
                toast(result.message);
                onDone?.();
            },
        }],
    });
}

function passwordResetDialog(name, onDone) {
    const lock = h('input', {type: 'checkbox', checked: true});
    const why = reasonField(true, 'e.g. Account stolen; confirmed ownership from their registration details. Ticket 1234.');
    const picked = casePicker(name, null, false);
    openDialog({
        title: `Reset password: ${name}`,
        subtitle: 'Only once you\'re sure the account is theirs.',
        body: [
            h('div', {class: 'callout warn'}, icon('alert'), h('div', {},
                'You\'ll get a one-time code to give them. They choose a new password with it at /recover on the website, within 3 days. Staff never learn the new password.')),
            h('label', {class: 'check-row'}, lock, h('span', {}, 'Lock their current password now')),
            h('p', {class: 'hint'}, 'For a stolen account: whoever has the password is logged out, and can\'t log back in with it.'),
            why.field, picked.field,
        ],
        actions: [{label: 'Cancel'}, {
            label: 'Issue code',
            kind: 'danger',
            icon: 'key',
            run: async ({close}) => {
                const issued = await api('POST', '/support/password-reset', {
                    name,
                    lock: lock.checked,
                    reason: why.value(),
                    case: picked.value()
                });
                if (!issued.ok) throw new Error(issued.message);
                close();
                resetCodeDialog(name, issued);
                onDone?.();
                return true;
            },
        }],
    });
}

// The code, shown only this once.
function resetCodeDialog(name, issued) {
    const copy = button('Copy', {small: true, kind: 'ghost', icon: 'copy'});
    copy.addEventListener('click', async () => {
        try {
            await navigator.clipboard.writeText(issued.code);
            toast('Code copied.');
        } catch {
            toast('Copy the code by hand.', 'error');
        }
    });
    openDialog({
        title: `Reset code for ${name}`,
        subtitle: `Valid until ${fullTime(issued.until_ms)}. It's shown only this once.`,
        body: [
            h('div', {class: 'secret reset-code'}, h('span', {}, issued.code), copy),
            h('p', {}, issued.message),
            h('p', {class: 'hint'}, 'Give it to them privately. Lost it? Issue another: the new code replaces this one.'),
        ],
        actions: [{label: 'Done', kind: 'primary'}],
    });
}

// A search for an item to give, by name, config name or id. `onPick` gets
// the item picked.
function objPicker({autofocus = true, required = true, placeholder = 'Item name, config name or id', onPick} = {}) {
    const search = h('input', {
        class: 'input',
        type: 'search',
        required,
        autofocus,
        autocomplete: 'off',
        spellcheck: 'false',
        placeholder,
        'data-invalid': 'Pick an item from the results.'
    });
    if (required) search.setCustomValidity('Pick an item');
    const results = h('div', {class: 'obj-results'});
    let chosen = null;
    let timer;
    let seq = 0;
    const pick = (obj) => {
        chosen = obj;
        search.setCustomValidity('');
        search.removeAttribute('aria-invalid');
        if (onPick) {
            search.value = '';
            replace(results);
            onPick(obj);
            return;
        }
        search.value = obj.name;
        replace(results, h('div', {class: 'obj-picked'}, icon('check'), h('b', {}, obj.name), h('span', {class: 'muted num'}, `${obj.debugname} · id ${obj.id}${obj.stackable ? ' · stacks' : ''}`)));
    };
    search.addEventListener('input', () => {
        chosen = null;
        if (required) search.setCustomValidity('Pick an item');
        clearTimeout(timer);
        const q = search.value.trim();
        if (!q) return replace(results);
        timer = setTimeout(async () => {
            const mine = ++seq;
            try {
                const objs = await api('GET', `/objs?${query({q})}`);
                if (mine !== seq) return;
                replace(results, objs.length
                    ? objs.map((o) => h('button', {type: 'button', class: 'obj-row', on: {click: () => pick(o)}},
                        h('span', {}, o.name), h('span', {class: 'muted num'}, `${o.debugname} · ${o.id}`)))
                    : h('p', {class: 'hint'}, 'No items match.'));
            } catch (e) {
                if (mine === seq) replace(results, h('p', {class: 'error-text'}, e.message));
            }
        }, 200);
    });
    return {search, results, chosen: () => chosen};
}

// Gives back items a player lost: all of them, or none if any doesn't fit.
// `items` start the list ({ obj, name, debugname, count }); `proof` is what
// shows they had them (a wealth event or an earlier save), which the
// console checks covers every item.
function restoreDialog(name, {items = [], proof = null, proofLabel = '', caseId = null} = {}, onDone) {
    const rows = [];
    const list = h('div', {class: 'restore-list'});
    const addRow = (item) => {
        const existing = rows.find((r) => r.obj === item.obj);
        if (existing) {
            existing.count.value = String(Number(existing.count.value) + (item.count || 1));
            return;
        }
        const row = {
            obj: item.obj,
            name: item.name,
            debugname: item.debugname,
            inv: 'bank',
            count: h('input', {
                class: 'input',
                type: 'number',
                min: '1',
                max: '2147483647',
                step: '1',
                value: String(item.count || 1),
                required: true,
                inputmode: 'numeric',
                'aria-label': `How many ${item.name}`,
                'data-invalid': 'Counts are 1 to 2147483647.'
            }),
        };
        rows.push(row);
        draw();
    };
    const draw = () => replace(list, rows.length
        ? rows.map((row) => {
            const into = h('div', {class: 'segmented small'});
            const drawInto = () => replace(into, ['bank', 'inv'].map((inv) =>
                h('button', {
                    type: 'button', 'aria-pressed': String(row.inv === inv), on: {
                        click: () => {
                            row.inv = inv;
                            drawInto();
                        }
                    }
                }, invLabel(inv))));
            drawInto();
            return h('div', {class: 'restore-row'},
                h('div', {class: 'grow'}, h('b', {}, row.name), h('div', {class: 'muted num'}, row.debugname ? `${row.debugname} · id ${row.obj}` : `id ${row.obj}`)),
                row.count, into,
                button(null, {
                    small: true,
                    kind: 'ghost',
                    icon: 'x',
                    'aria-label': `Remove ${row.name}`,
                    on: {
                        click: () => {
                            rows.splice(rows.indexOf(row), 1);
                            draw();
                        }
                    }
                }));
        })
        : h('p', {class: 'hint'}, 'Search for each item to give back.'));
    items.forEach(addRow);
    draw();
    const picker = objPicker({
        autofocus: !items.length,
        required: false,
        placeholder: 'Add an item: name, config name or id',
        onPick: (o) => addRow({obj: o.id, name: o.name, debugname: o.debugname, count: 1})
    });
    const why = reasonField(true, 'e.g. Lost when the bank bug wiped their tab; see the ticket.');
    const picked = casePicker(name, caseId, true);
    openDialog({
        title: `Restore items: ${name}`,
        subtitle: 'All of them, or nothing if any doesn\'t fit. Online, they get them straight away; offline, at their next login.',
        width: 'min(640px, calc(100vw - 32px))',
        body: [
            proof && h('div', {class: 'callout'}, icon('check'), h('div', {}, h('b', {}, 'Proof: '), proofLabel,
                h('div', {class: 'list-sub'}, 'The console checks it covers every item you restore.'))),
            h('div', {class: 'field'}, h('span', {}, 'Items'), list, picker.search, picker.results),
            why.field, picked.field,
        ],
        actions: [{label: 'Cancel'}, {
            label: 'Restore',
            kind: 'primary',
            icon: 'plus',
            run: async () => {
                if (!rows.length) throw new Error('Add at least one item.');
                const result = await api('POST', '/support/restore', {
                    name,
                    items: rows.map((r) => ({inv: r.inv, obj: r.obj, count: Number(r.count.value)})),
                    reason: why.value(),
                    case: picked.value(),
                    proof,
                });
                if (!result.ok) throw new Error(result.message);
                toast(result.message);
                onDone?.();
            },
        }],
    });
}

// A wealth event as the proof for restoring what it shows them getting.
async function restoreFromWealth(name, e) {
    try {
        const objs = await api('GET', `/objs?${query({q: e.text})}`);
        const obj = objs.find((o) => o.debugname === e.text);
        if (!obj) throw new Error(`There's no item called ${e.text} any more.`);
        restoreDialog(name, {
            items: [{obj: obj.id, name: obj.name, debugname: obj.debugname, count: Number(e.count)}],
            proof: {kind: 'wealth', id: e.id},
            proofLabel: `${WEALTH_TYPES[e.event_type] || 'Got'}: ${Number(e.count).toLocaleString()} x ${obj.name}, ${fullTime(e.at_ms)}${e.world ? ` on world ${e.world}` : ''}.`,
        });
    } catch (err) {
        toast(err.message, 'error');
    }
}

// Every item in a sheet, by id, all inventories together.
function itemTotals(sheet) {
    const totals = new Map();
    for (const inv of sheet.invs) {
        for (const item of inv.items) {
            const t = totals.get(item.id) || {obj: item.id, name: item.name, count: 0};
            t.count += item.count;
            totals.set(item.id, t);
        }
    }
    return totals;
}

// What changed between an earlier save and now.
function sheetDiff(then, now) {
    const before = itemTotals(then);
    const after = itemTotals(now);
    const lost = [];
    const gained = [];
    for (const [id, t] of before) {
        const left = after.get(id)?.count || 0;
        if (left < t.count) lost.push({...t, count: t.count - left});
    }
    for (const [id, t] of after) {
        const had = before.get(id)?.count || 0;
        if (t.count > had) gained.push({...t, count: t.count - had});
    }
    const stats = then.stats.map((s) => [s, now.stats.find((n) => n.id === s.id)]).filter(([s, n]) => n && n.xp !== s.xp);
    return {lost, gained, stats, moved: then.x !== now.x || then.z !== now.z || then.level !== now.level};
}

const itemLines = (items) => h('div', {class: 'diff-items'}, items.map((i) =>
    h('div', {class: 'diff-item'}, h('span', {class: 'num'}, `${i.count.toLocaleString()} x`), ' ', i.name)));

const pointTitle = (p) => {
    const [label] = POINT_KINDS[p.kind] || [p.kind];
    return p.kind === 'staff' ? label : `${label} on ${worldLabel(p.node, p.world)}`;
};

// An earlier save next to their current one, to restore what they lost
// since or roll back to it.
async function pointDialog(name, point, onDone) {
    let then;
    let now;
    try {
        [then, now] = await Promise.all([
            api('GET', `/support/history/sheet?${query({name, id: point.id})}`),
            api('GET', `/support/sheet?${query({name})}`),
        ]);
    } catch (e) {
        toast(e.message, 'error');
        return;
    }
    if (!then || !now) {
        toast('That save is gone (pruned), or they are.', 'error');
        return;
    }
    const d = sheetDiff(then, now);
    const section = (title, ...content) => h('div', {class: 'field'}, h('span', {}, title), ...content);
    openDialog({
        title: `Their save from ${fullTime(point.saved_ms)}`,
        subtitle: `${pointTitle(point)} · ${ago(point.saved_ms)}${now.online ? ' · compared with them live now' : ' · compared with their save now'}`,
        width: 'min(640px, calc(100vw - 32px))',
        body: [
            point.note && h('div', {class: 'callout'}, icon('shield'), h('div', {}, point.note)),
            section('Had then, gone now', d.lost.length ? itemLines(d.lost) : h('p', {class: 'hint'}, 'Nothing.')),
            section('Have now, not then', d.gained.length ? itemLines(d.gained) : h('p', {class: 'hint'}, 'Nothing.')),
            section('Stats', d.stats.length
                ? h('div', {class: 'diff-items'}, d.stats.map(([s, n]) => h('div', {class: 'diff-item'}, `${s.name} `, h('span', {class: 'num'}, `${s.base} → ${n.base}`), h('span', {class: 'muted num'}, ` (${(n.xp - s.xp).toLocaleString()} xp)`))))
                : h('p', {class: 'hint'}, 'The same.')),
            d.moved && section('Position', h('p', {class: 'num'}, `${where(then.x, then.z, then.level)} then, ${where(now.x, now.z, now.level)} now`)),
        ],
        actions: [
            {label: 'Close'},
            d.lost.length > 0 && {
                label: 'Restore what\'s gone',
                icon: 'plus',
                run: () => {
                    restoreDialog(name, {
                        items: d.lost.map((i) => ({obj: i.obj, name: i.name, count: i.count})),
                        proof: {kind: 'point', id: point.id},
                        proofLabel: `their save from ${fullTime(point.saved_ms)} had them.`,
                    }, onDone);
                },
            },
            {
                label: 'Roll back to this',
                kind: 'danger',
                icon: 'undo',
                run: () => rollbackDialog(name, point, now.online, onDone),
            },
        ].filter(Boolean),
    });
}

function rollbackDialog(name, point, online, onDone) {
    const why = reasonField(true, 'e.g. Duped coins through the trade bug; rolling back to before the first dupe.');
    const picked = casePicker(name, null, true);
    openDialog({
        title: `Roll back: ${name}`,
        subtitle: `To their save from ${fullTime(point.saved_ms)} (${ago(point.saved_ms)}).`,
        body: [
            h('div', {class: 'callout danger'}, icon('undo'), h('div', {},
                'Everything since is undone: items, experience, quests and where they are. ',
                online ? 'They\'re online, so they\'re logged out first. ' : '',
                'Their save as it is now is kept in the history, so this can be undone.')),
            why.field, picked.field,
        ],
        actions: [{label: 'Cancel'}, {
            label: 'Roll back',
            kind: 'danger',
            icon: 'undo',
            run: async () => {
                const result = await api('POST', '/support/rollback', {
                    name,
                    point: point.id,
                    reason: why.value(),
                    case: picked.value()
                });
                if (!result.ok) throw new Error(result.message);
                toast(result.message);
                onDone?.();
            },
        }],
    });
}

// What a player's session state means, and whether freeing it can help.
function sessionSummary(s) {
    if (!s.owner) return {tone: 'ok', text: 'No world holds their session: they can log in on any world.', free: false};
    const o = s.owner;
    const at = worldLabel(o.node, o.world);
    const since = o.since_ms ? ` since ${ago(o.since_ms)}` : '';
    if (!o.reachable || !s.hold) {
        const silent = o.heartbeat_age_ms == null || o.heartbeat_age_ms > o.takeover_after_ms;
        return silent
            ? {
                tone: 'warn',
                text: `Held by ${at}${since}, which has stopped (${o.heartbeat_age_ms == null ? 'no heartbeat' : `no heartbeat for ${span(o.heartbeat_age_ms / 1000)}`}). Logging in on another world takes it over; freeing it lets them in anywhere now.`,
                free: true
            }
            : {
                tone: 'danger',
                text: `Held by ${at}${since}, which the console can't reach but which is still running (heartbeat ${span(o.heartbeat_age_ms / 1000)} ago). It may still have them; it can only be freed once it has been silent for ${span(o.takeover_after_ms / 1000)}.`,
                free: true
            };
    }
    const hold = s.hold;
    const name = at.charAt(0).toUpperCase() + at.slice(1);
    if (hold.online) return {tone: 'ok', text: `Online on ${at}${since}.`, free: false};
    if (hold.logging_in) return {tone: '', text: `Logging in on ${at} right now.`, free: false};
    if (hold.editing) return {tone: '', text: `Staff are changing their save on ${at} right now.`, free: false};
    if (hold.logout_save === true) return {
        tone: 'warn',
        text: `Logging out of ${at}, but their final save didn't reach the database. ${name} keeps their session until it does, and resends it when its database is back; freeing it sends it now.`,
        free: true
    };
    if (hold.logout_save === false) return {
        tone: '',
        text: `Logging out of ${at}: their final save is on its way.`,
        free: false
    };
    if (hold.local_save) return {
        tone: 'warn',
        text: `${name} kept their newest save in a local file while its database was down, and holds their session until that's stored. Freeing it stores it now.`,
        free: true
    };
    return {
        tone: 'warn',
        text: `${name} has a claim on their session but doesn't have them: it's left over, and goes at that world's next heartbeat. Freeing it releases it now.`,
        free: true
    };
}

function supportSection(name) {
    const session = h('div', {});
    const reset = h('div', {});
    const actions = h('div', {class: 'actions'});
    let online = false;
    const history = state.canCommand && pagedList(
        (before) => api('GET', `/support/history?${query({name, before})}`),
        (p) => {
            const [, iconName] = POINT_KINDS[p.kind] || ['', 'clock'];
            return h('button', {
                    type: 'button',
                    class: 'list-item list-button',
                    on: {click: () => pointDialog(name, p, reload)}
                },
                h('div', {class: `list-icon ${p.kind === 'staff' ? 'accent' : ''}`}, icon(iconName)),
                h('div', {class: 'list-main'},
                    h('div', {class: 'list-title'}, pointTitle(p)),
                    h('div', {class: 'list-sub'}, p.note || fullTime(p.saved_ms))),
                h('div', {class: 'list-side', title: fullTime(p.saved_ms)}, ago(p.saved_ms)));
        },
        'No earlier saves kept yet: one is kept at logout and every hour they play.');

    const loadSession = async () => {
        let s;
        try {
            s = await api('GET', `/support/session?${query({name})}`);
        } catch (e) {
            replace(session, h('div', {class: 'callout danger'}, icon('alert'), h('div', {}, e.message)));
            return;
        }
        if (!s) {
            replace(session, h('p', {class: 'muted'}, 'No registered player has this name.'));
            return;
        }
        online = !!s.hold?.online;
        const summary = sessionSummary(s);
        replace(session, h('div', {class: `callout ${summary.tone === 'ok' ? '' : summary.tone}`},
            icon(summary.tone === 'ok' ? 'check' : summary.tone ? 'alert' : 'activity'),
            h('div', {class: 'grow'}, h('b', {}, 'Session: '), summary.text),
            summary.free && button('Free session', {
                small: true,
                icon: 'unlock',
                on: {click: () => unstickDialog(name, summary.text, reload)}
            })));
    };
    const reload = () => {
        loadSession();
        history?.reload();
    };
    // The password reset standing, from their record on the player page.
    const update = (record) => {
        replace(reset, record.reset_until_ms && h('div', {class: 'callout warn'}, icon('key'),
            h('div', {class: 'grow'}, `A password reset is waiting for them to redeem, until ${fullTime(record.reset_until_ms)}.`)));
    };
    replace(actions,
        state.canCommand && button('Send home', {small: true, icon: 'home', on: {click: () => sendHomeDialog(name, online, reload)}}),
        state.canCommand && button('Reset password', {
            small: true,
            icon: 'key',
            on: {click: () => passwordResetDialog(name, reload)}
        }),
        state.canCommand && button('Restore items', {
            small: true,
            icon: 'plus',
            on: {click: () => restoreDialog(name, {}, reload)}
        }),
        button(null, {small: true, icon: 'refresh', 'aria-label': 'Reload', title: 'Reload', on: {click: reload}}));
    loadSession();
    const el = h('section', {class: 'card'},
        h('div', {class: 'card-head wraps'}, h('h2', {}, 'Player support'), actions),
        h('div', {class: 'card-body support'}, session, reset),
        history && h('div', {class: 'card-sub'}, h('h3', {}, 'Earlier saves'),
            h('span', {class: 'hint'}, 'Click one to compare it with now, restore what\'s gone, or roll back.')),
        history && history.el);
    return {el, update, reload};
}

// ---------------------------------------------------------------------------
// News on the website (admins and maintainers)

// What news posts and case comments can embed, and the most they take (50 MB).
const MEDIA_TYPES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'video/mp4', 'video/webm'];
const MAX_MEDIA = 50 * 1024 * 1024;

// Uploads an image or video to `path`: the website's news media, or the
// console's own for cases.
async function uploadMedia(file, path = '/api/news/media') {
    if (!MEDIA_TYPES.includes(file.type)) throw new Error(`${file.name} isn't a PNG, JPEG, GIF, WebP, MP4 or WebM.`);
    if (file.size > MAX_MEDIA) throw new Error(`${file.name} is over 50 MB.`);
    let res;
    try {
        res = await fetch(path, {
            method: 'POST',
            headers: {'Content-Type': 'application/octet-stream', 'X-CSRF-Token': state.session?.csrf ?? ''},
            credentials: 'same-origin',
            body: file,
        });
    } catch {
        throw new Error('Could not reach the console. Check your connection.');
    }
    let data = null;
    try {
        data = await res.json();
    } catch { /* not JSON */
    }
    if (!res.ok) {
        if (res.status === 401) sessionEnded();
        throw new Error(data?.error || `Upload failed (${res.status})`);
    }
    return data;
}

// Images and videos into a Markdown field: the button, or pasting or
// dropping files, uploads them with `upload` and puts each in where the
// cursor was when it started.
function mediaUploader(textarea, upload, label = 'Add image or video') {
    const status = h('span', {class: 'hint'});
    const input = h('input', {type: 'file', class: 'hidden', accept: MEDIA_TYPES.join(','), multiple: true});
    const add = button(label, {small: true, icon: 'clip', on: {click: () => input.click()}});
    let uploading = 0;
    const insert = (text, at) => {
        textarea.setRangeText(text, at, at, 'end');
        textarea.dispatchEvent(new Event('input', {bubbles: true}));
    };
    const send = async (list) => {
        for (const file of list) {
            const at = textarea.selectionStart ?? textarea.value.length;
            uploading++;
            add.disabled = true;
            status.textContent = `Uploading ${file.name}...`;
            try {
                const saved = await upload(file);
                const caption = file.name.replace(/\.[^.]+$/, '').replace(/[[\]]/g, '');
                const before = at > 0 && textarea.value[at - 1] !== '\n' ? '\n\n' : '';
                insert(`${before}![${caption}](${saved.url})\n\n`, at);
                status.textContent = `Added ${file.name}.`;
            } catch (e) {
                status.textContent = '';
                toast(e.message, 'error');
            } finally {
                uploading--;
                add.disabled = uploading > 0;
            }
        }
    };
    input.addEventListener('change', () => {
        send([...input.files]);
        input.value = '';
    });
    textarea.addEventListener('paste', (e) => {
        const pasted = [...(e.clipboardData?.files ?? [])];
        if (pasted.length) {
            e.preventDefault();
            send(pasted);
        }
    });
    textarea.addEventListener('dragover', (e) => {
        if (e.dataTransfer?.types.includes('Files')) e.preventDefault();
    });
    textarea.addEventListener('drop', (e) => {
        const dropped = [...(e.dataTransfer?.files ?? [])];
        if (dropped.length) {
            e.preventDefault();
            send(dropped);
        }
    });
    return {button: add, status, input, busy: () => uploading > 0};
}

// Where an embedded file loads from, or '' if it may not: news uploads come
// through the console, and anything else is left as the post has it.
const newsMedia = (src) => (src.startsWith('/news/media/') ? `/api${src}` : src);

// HTML for a post or comment, rebuilt from an allow-list of elements and
// attributes (nothing runs, nothing styles itself), with images and videos
// loaded from where `mediaSrc` says.
const PREVIEW_TAGS = new Set(['P', 'BR', 'HR', 'H1', 'H2', 'H3', 'H4', 'H5', 'H6', 'STRONG', 'B', 'EM', 'I', 'U', 'S', 'DEL',
    'CODE', 'PRE', 'BLOCKQUOTE', 'UL', 'OL', 'LI', 'A', 'IMG', 'VIDEO', 'TABLE', 'THEAD', 'TBODY', 'TR', 'TH', 'TD', 'CENTER',
    'SPAN', 'DIV', 'SUP', 'SUB', 'SMALL', 'BIG', 'FONT']);
const PREVIEW_DROP = new Set(['SCRIPT', 'STYLE', 'IFRAME', 'OBJECT', 'EMBED', 'TEMPLATE', 'SVG', 'MATH', 'FORM', 'INPUT', 'BUTTON', 'TEXTAREA', 'SELECT']);

function previewNodes(nodes, mediaSrc = newsMedia) {
    const out = [];
    for (const node of nodes) {
        if (node.nodeType === Node.TEXT_NODE) {
            out.push(document.createTextNode(node.textContent));
            continue;
        }
        if (node.nodeType !== Node.ELEMENT_NODE || PREVIEW_DROP.has(node.tagName)) continue;
        const children = previewNodes(node.childNodes, mediaSrc);
        if (!PREVIEW_TAGS.has(node.tagName)) {
            out.push(...children);
            continue;
        }
        const el = document.createElement(node.tagName.toLowerCase());
        const media = mediaSrc(node.getAttribute('src') || '');
        if (node.tagName === 'A') {
            const href = node.getAttribute('href') || '';
            if (/^(https?:|\/|#|mailto:)/i.test(href)) el.setAttribute('href', href);
            el.setAttribute('target', '_blank');
            el.setAttribute('rel', 'noopener noreferrer');
        } else if (node.tagName === 'IMG') {
            if (/^(https?:|\/)/i.test(media)) el.setAttribute('src', media);
            el.setAttribute('alt', node.getAttribute('alt') || '');
        } else if (node.tagName === 'VIDEO') {
            if (/^(https?:|\/)/i.test(media)) el.setAttribute('src', media);
            el.setAttribute('controls', '');
            el.setAttribute('preload', 'metadata');
            el.setAttribute('playsinline', '');
        }
        if (node.getAttribute('title')) el.setAttribute('title', node.getAttribute('title'));
        if (node.getAttribute('align')) el.setAttribute('align', node.getAttribute('align'));
        append(el, children);
        out.push(el);
    }
    return out;
}

function newsDialog(categories, post, onDone) {
    let category = post?.category_id ?? categories[0]?.id;
    const chips = h('div', {class: 'chips'});
    const draw = () => replace(chips, categories.map((c) =>
        h('button', {
            type: 'button', class: 'chip', 'aria-pressed': String(category === c.id), on: {
                click: () => {
                    category = c.id;
                    draw();
                }
            }
        }, c.name)));
    draw();
    const title = h('input', {
        class: 'input',
        maxlength: '120',
        required: true,
        autofocus: true,
        value: post?.title ?? '',
        placeholder: 'The Grand Exchange opens'
    });
    const content = h('textarea', {
        class: 'input code-area', required: true, rows: '14', spellcheck: 'true', 'aria-label': 'Text',
        placeholder: '## What\'s new\n\nThe **Grand Exchange** opens today.\n\n![The new trading floor](...)'
    });
    content.value = post?.markdown ?? '';

    const media = mediaUploader(content, (file) => uploadMedia(file));

    // Write and Preview.
    let mode = 'write';
    const preview = h('div', {class: 'news-preview hidden'});
    const writeArea = h('div', {class: 'news-write'},
        h('div', {class: 'news-tools'}, media.button, media.status, media.input), content,
        h('span', {class: 'hint'}, 'Markdown: **bold**, *italic*, ## headings, - lists, [links](https://...). Add images, GIFs and videos with the button, or paste or drop them in.'));
    const tabs = h('div', {class: 'segmented tabs', role: 'group', 'aria-label': 'View'});
    const showPreview = async () => {
        replace(preview, h('div', {class: 'spinner'}));
        try {
            const {html} = await api('POST', '/news/preview', {markdown: content.value});
            const doc = new DOMParser().parseFromString(html, 'text/html');
            const nodes = previewNodes(doc.body.childNodes);
            replace(preview, nodes.length ? nodes : h('p', {class: 'muted'}, 'Nothing to preview yet.'));
        } catch (e) {
            replace(preview, h('p', {class: 'error-text'}, e.message));
        }
    };
    const drawTabs = () => {
        replace(tabs, [['write', 'Write'], ['preview', 'Preview']].map(([key, label]) =>
            h('button', {
                type: 'button', 'aria-pressed': String(mode === key), on: {
                    click: () => {
                        mode = key;
                        drawTabs();
                    }
                }
            }, label)));
        writeArea.classList.toggle('hidden', mode !== 'write');
        preview.classList.toggle('hidden', mode !== 'preview');
        if (mode === 'preview') showPreview();
    };
    drawTabs();

    openDialog({
        title: post ? 'Edit post' : 'New post',
        subtitle: post ? `Posted ${fullTime(post.created_ms)}; the date stays.` : 'Goes on the website straight away.',
        width: 'min(780px, calc(100vw - 32px))',
        body: [
            h('div', {class: 'field'}, h('span', {}, 'Category'), chips),
            h('label', {class: 'field'}, h('span', {}, 'Title'), title),
            h('div', {class: 'field'}, h('span', {}, 'Text'), tabs, writeArea, preview),
        ],
        actions: [{label: 'Cancel'}, {
            label: post ? 'Save' : 'Post',
            kind: 'primary',
            icon: post ? 'check' : 'news',
            run: async () => {
                if (media.busy()) throw new Error('Wait for the upload to finish.');
                if (!content.value.trim()) {
                    mode = 'write';
                    drawTabs();
                    content.focus();
                    throw new Error('Text is required.');
                }
                await api('POST', '/news/save', {
                    id: post?.id ?? null,
                    category_id: category,
                    title: title.value.trim(),
                    markdown: content.value
                });
                toast(post ? 'Post saved.' : 'Posted.');
                onDone();
            },
        }],
    });
}

function newsPage() {
    const list = h('div', {class: 'list'}, h('div', {class: 'empty'}, h('div', {class: 'spinner'})));
    const more = h('div', {class: 'more hidden'});
    const newButton = button('New post', {icon: 'plus', kind: 'primary', disabled: true});
    let categories = [];
    let last = null;
    const categoryName = (id) => categories.find((c) => c.id === id)?.name ?? `Category ${id}`;
    const item = (p) => h('div', {class: 'list-item'},
        h('div', {class: 'list-icon accent'}, icon('news')),
        h('div', {class: 'list-main'},
            h('div', {class: 'list-title'}, h('b', {}, p.title), h('span', {class: 'badge'}, categoryName(p.category_id))),
            h('div', {class: 'list-sub'}, `#${p.id} · ${fullTime(p.created_ms)}`)),
        h('div', {class: 'chips'},
            button('Edit', {
                small: true,
                icon: 'edit',
                on: {click: () => newsDialog(categories, p, () => load(false))}
            }),
            button(null, {
                small: true, icon: 'trash', 'aria-label': `Delete ${p.title}`, title: 'Delete', on: {
                    click: () => confirmDialog({
                        title: 'Delete post',
                        subtitle: p.title,
                        body: [h('p', {class: 'muted'}, 'It comes off the website for good.')],
                        label: 'Delete',
                        kind: 'danger',
                        run: async () => {
                            const result = await api('POST', '/news/delete', {id: p.id, title: p.title});
                            toast(result.message, result.ok ? 'ok' : 'error');
                            load(false);
                        },
                    })
                }
            })));
    const load = async (older) => {
        try {
            const page = await api('GET', `/news?${query({before: older ? last : null})}`);
            categories = page.categories;
            newButton.disabled = !categories.length;
            if (!older) list.replaceChildren();
            append(list, page.posts.map(item));
            if (!list.children.length) append(list, h('div', {class: 'empty'}, icon('news'), 'No posts yet.'));
            if (page.posts.length) last = page.posts[page.posts.length - 1].id;
            more.classList.toggle('hidden', page.posts.length < 20);
        } catch (e) {
            replace(list, h('div', {class: 'empty'}, e.message));
        }
    };
    newButton.addEventListener('click', () => newsDialog(categories, null, () => load(false)));
    append(more, button('Load older', {small: true, on: {click: () => load(true)}}));
    const box = scrollWindow(list, more);
    loadOnScroll(box, more, () => load(true));
    load(false);
    return h('div', {class: 'page'},
        pageHead('News', 'Posts on the website\'s front page and news archive.', newButton),
        h('section', {class: 'card'}, box));
}

// ---------------------------------------------------------------------------
// Moderation center: cases staff work on together, the triage queue of abuse
// reports and flagged sessions, mutes and bans in force, and every action.

// The Report Abuse screen's 12 rules, by the index the client sends.
const REPORT_RULES = ['Offensive language', 'Item scamming', 'Password scamming', 'Bug abuse', 'Jagex staff impersonation',
    'Account sharing/trading', 'Macroing', 'Multiple logging in', 'Encouraging others to break rules',
    'Misuse of customer support', 'Advertising / website', 'Real world item trading'];
const ruleName = (reason) => `Rule ${reason + 1}: ${REPORT_RULES[reason] || 'unknown'}`;

const CASE_STATUS = {
    open: ['Open', 'warn'],
    investigating: ['Investigating', 'accent'],
    waiting: ['Waiting', ''],
    closed: ['Closed', 'ok']
};
const PRIORITIES = [['low', 'Low'], ['normal', 'Normal'], ['high', 'High'], ['urgent', 'Urgent']];
const PRIORITY_TONE = {urgent: 'danger', high: 'warn'};
const RESOLUTIONS = {actioned: 'Actioned', no_action: 'No action', duplicate: 'Duplicate'};

const caseHref = (id) => `/case?id=${id}`;
const caseUpload = (file) => uploadMedia(file, '/api/cases/media');
// Only the console's own case uploads load in a comment.
const caseMedia = (src) => (src.startsWith('/api/cases/media/') ? src : '');
const MARKDOWN_HINT = 'Markdown works: **bold**, *italic*, - lists, [links](https://...). Paste or drop in images, GIFs and videos.';

// A comment or note: its rendered Markdown, or the text as written.
function richText(html, text) {
    if (!html) return h('div', {class: 'comment-text'}, text);
    return h('div', {class: 'comment-text md'}, previewNodes(new DOMParser().parseFromString(html, 'text/html').body.childNodes, caseMedia));
}

const nameKey = (name) => (name || '').trim().toLowerCase().replace(/_/g, ' ');
const isMe = (name) => nameKey(name) === nameKey(state.session.name);
const worldOf = (node) => {
    const w = state.worlds.find((x) => x.node === node);
    return w ? worldName(w) : `Node ${node}`;
};
const where = (x, z, level) => `${x}, ${z}${level ? ` · floor ${level}` : ''}`;
const listNames = (names) => (names.length <= 2 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);
const evidenceKey = (e) => (e.type === 'clip' ? `clip:${e.replay}:${e.from_ms}-${e.to_ms}`
    : e.type === 'evasion' ? `evasion:${nameKey(e.player)}:${nameKey(e.banned)}` : `${e.type}:${e.id}`);
// How two accounts are linked, in words.
const linkedHow = (via) => (via === 'browser' ? 'logged in with the same browser as' : 'shares an address with');
// Who opens the cases the console opens on its own.
const DETECTION = 'Detection';
// A clip of a recording: its page, playing that stretch. Works as a link
// from anywhere (comments, chat between staff), as the console's address.
const clipHref = ({replay, from_ms, to_ms, label}) => `/replay?${query({id: replay, from: from_ms, to: to_ms, label})}`;
const clipSpan = (from, to) => `${clock(from / 1000)} to ${clock(to / 1000)}`;
const clip = (text, n) => (text.length > n ? `${text.slice(0, n - 1)}…` : text);

function statusBadge(c) {
    const [label, tone] = CASE_STATUS[c.status] || [c.status, ''];
    return h('span', {class: `badge ${tone}`}, c.status === 'closed' && c.resolution ? `${label} · ${RESOLUTIONS[c.resolution]}` : label);
}

// Shown only when it's out of the ordinary, unless `always`.
function priorityBadge(priority, always) {
    if (!always && (priority === 'normal' || priority === 'low')) return null;
    return h('span', {class: `badge ${PRIORITY_TONE[priority] || ''}`}, PRIORITIES.find(([k]) => k === priority)?.[1] || priority);
}

const caseLinks = (ids) => (ids || []).map((id) => link(caseHref(id), {
    class: 'badge accent',
    title: `Evidence in case #${id}`
}, icon('briefcase'), `#${id}`));

function avatars(names) {
    return h('div', {class: 'avatars', title: names.join(', ')},
        names.slice(0, 4).map((n) => h('div', {class: 'avatar'}, initial(n))),
        names.length > 4 && h('span', {class: 'num muted'}, `+${names.length - 4}`));
}

function evidenceCounts(c) {
    const n = {report: 0, replay: 0, chat: 0, clip: 0, evasion: 0, moderation: 0};
    for (const e of c.evidence) n[e.type]++;
    return [n.report && plural(n.report, 'report'), n.replay && plural(n.replay, 'session'), n.clip && plural(n.clip, 'clip'),
        n.chat && plural(n.chat, 'message'), n.evasion && plural(n.evasion, 'ban evasion link'), n.moderation && plural(n.moderation, 'lifted sanction')].filter(Boolean).join(' · ');
}

function caseItem(c, extra) {
    const closed = c.status === 'closed';
    const tone = closed ? 'ok' : PRIORITY_TONE[c.priority] || (c.assignees.length ? 'accent' : '');
    const counts = evidenceCounts(c);
    return h('div', {class: `list-item${c.unread ? ' unread' : ''}`},
        h('div', {class: `list-icon ${tone}`}, icon(closed ? 'check' : 'briefcase')),
        h('div', {class: 'list-main'},
            h('div', {class: 'list-title'},
                c.unread && h('span', {class: 'unread-dot', title: 'Changed since you last looked', 'aria-label': 'Changed since you last looked'}),
                h('span', {class: 'case-id num'}, `#${c.id}`),
                link(caseHref(c.id), {class: 'name-link'}, h('b', {}, c.title)), ' ', statusBadge(c), ' ', priorityBadge(c.priority), appealBadge(c),
                c.created_by === DETECTION && [' ', h('span', {class: 'badge accent', title: 'The console opened this case on its own'}, 'Detected')]),
            h('div', {class: 'list-sub'}, playerLink(c.player),
                closed ? ` · closed by ${c.closed_by} ${ago(c.closed_ms)}` : ` · opened by ${c.created_by} ${ago(c.created_ms)}`,
                counts && ` · ${counts}`, c.comments ? ` · ${plural(c.comments, 'comment')}` : ''),
            extra),
        h('div', {class: 'list-side stack'},
            c.assignees.length ? avatars(c.assignees) : !closed && h('span', {class: 'badge warn'}, 'Nobody'),
            h('span', {title: `Last change ${fullTime(c.updated_ms)}`}, ago(c.updated_ms))));
}

// An appeal case's badge: waiting on a decision, or how it went.
function appealBadge(c) {
    const decision = c.appeal?.decision;
    if (!c.appeal) return null;
    return decision
        ? h('span', {class: `badge ${decision === 'overturned' ? 'accent' : 'ok'}`}, icon('flag'), decision === 'overturned' ? 'Appeal overturned' : 'Appeal upheld')
        : h('span', {class: 'badge warn'}, icon('flag'), 'Appeal');
}

// Case counts for the nav and the center, refreshed while signed in.
const caseListeners = new Set();
let caseTimer = null;

function onCases(fn) {
    caseListeners.add(fn);
    return () => caseListeners.delete(fn);
}

async function loadCaseSummary() {
    try {
        const [cases, economy] = await Promise.all([
            api('GET', '/cases/summary'),
            api('GET', '/economy/summary').catch(() => null),
        ]);
        state.caseSummary = cases;
        if (economy) state.economyAlerts = economy.open_alerts;
        updateNavCounts();
        announceDetections(cases);
        for (const fn of caseListeners) fn(state.caseSummary);
    } catch (e) {
        if (e.status !== 401) console.warn(e);
    }
}

// Tells staff of cases the console opened on its own since this browser last
// told them, while nobody is on them.
function announceDetections(s) {
    const heard = Number(prefs.get('detections-heard', '0')) || 0;
    const fresh = (s.detections || []).filter((d) => d.id > heard);
    if (!fresh.length) return;
    prefs.set('detections-heard', String(fresh[0].id));
    if (fresh.length === 1) {
        const d = fresh[0];
        toast([h('b', {}, `${d.title}: ${d.player}`), h('br'),
            'The console opened ', link(caseHref(d.id), {class: 'name-link'}, `case #${d.id}`), '. Nobody is on it yet.'], 'alert');
    } else {
        toast([h('b', {}, `${plural(s.detected, 'new detection')} nobody is on yet`), h('br'),
            link('/moderation?tab=cases&show=detected', {class: 'name-link'}, 'See the cases the console opened'), '.'], 'alert');
    }
}

// Adds `refs` to case `id` together: what's in it already is skipped.
async function addRefs(id, refs) {
    try {
        const detail = await api('POST', `/cases/${id}`, {op: 'add_evidence_many', items: refs});
        toast(`Added to case #${id}.`);
        loadCaseSummary();
        return detail;
    } catch (e) {
        toast(e.message, 'error');
        return null;
    }
}

// A new case on `player`, starting with the evidence `refs` (`what` says
// what they are). Any open case on them is offered first, to add to instead.
function newCaseDialog({player = '', title = '', refs = [], what = [], priority = 'normal'} = {}, onDone) {
    const nameInput = h('input', {
        class: 'input',
        value: player,
        placeholder: 'Player name',
        maxlength: '12',
        pattern: NAME_PATTERN,
        'data-invalid': NAME_RULE,
        required: true,
        autocomplete: 'off',
        spellcheck: 'false',
        autofocus: !player
    });
    nameSuggest(nameInput);
    let titled = !!title;
    const titleInput = h('input', {
        class: 'input',
        value: title || player,
        maxlength: '100',
        required: true,
        autocomplete: 'off',
        placeholder: 'What it\'s about, e.g. "Gold selling at the GE"',
        autofocus: !!player,
        'aria-label': 'Title'
    });
    titleInput.addEventListener('input', () => {
        titled = true;
    });
    let prio = priority;
    const prioChips = h('div', {class: 'segmented', role: 'group', 'aria-label': 'Priority'});
    const drawPrio = () => replace(prioChips, PRIORITIES.map(([k, label]) =>
        h('button', {
            type: 'button', 'aria-pressed': String(prio === k), on: {
                click: () => {
                    prio = k;
                    drawPrio();
                }
            }
        }, label)));
    drawPrio();
    const note = h('textarea', {
        class: 'input',
        maxlength: '4000',
        placeholder: 'Optional. What you know so far, for whoever picks it up.'
    });
    const noteMedia = mediaUploader(note, caseUpload, 'Attach');
    const join = h('input', {type: 'checkbox', checked: true});
    const existing = h('div', {});
    let seq = 0;
    let close = () => {
    };

    const lookExisting = debounce(async () => {
        const name = nameInput.value.trim();
        const mine = ++seq;
        if (!titled) titleInput.value = name;
        if (!validName(name)) return replace(existing);
        try {
            const cases = await api('GET', `/cases?${query({player: name})}`);
            if (mine !== seq) return;
            replace(existing, cases.length > 0 && h('div', {class: 'callout warn'}, icon('briefcase'), h('div', {class: 'grow'},
                h('div', {}, `${name} already has ${plural(cases.length, 'open case')}.`, refs.length ? ' Add to it instead?' : ''),
                h('div', {class: 'case-picks'}, cases.slice(0, 5).map((c) => h('div', {class: 'case-pick'},
                    link(caseHref(c.id), {
                        class: 'name-link',
                        on: {click: () => close()}
                    }, h('span', {class: 'num'}, `#${c.id}`), ` ${c.title}`),
                    refs.length > 0 && button(`Add to #${c.id}`, {
                        small: true, icon: 'plus', on: {
                            click: async () => {
                                await addRefs(c.id, refs);
                                close();
                                onDone?.();
                            }
                        }
                    })))))));
        } catch {
            replace(existing);
        }
    }, 250);
    nameInput.addEventListener('input', lookExisting);
    if (player) lookExisting();

    ({close} = openDialog({
        title: 'Open a case',
        subtitle: what.length ? `With ${what.join(', ')}.` : 'A case gathers the evidence on one player while staff look into it together.',
        body: [
            h('label', {class: 'field'}, h('span', {}, 'Player'), nameInput),
            existing,
            h('label', {class: 'field'}, h('span', {}, 'Title'), titleInput),
            h('div', {class: 'field'}, h('span', {}, 'Priority'), prioChips),
            h('div', {class: 'field'}, h('span', {}, 'First comment'), note,
                h('div', {class: 'news-tools'}, noteMedia.button, noteMedia.status, noteMedia.input), h('span', {class: 'hint'}, MARKDOWN_HINT)),
            h('label', {class: 'check-row'}, join, h('span', {}, 'Put me on it')),
        ],
        actions: [{label: 'Cancel'}, {
            label: 'Open case',
            kind: 'primary',
            icon: 'briefcase',
            run: async () => {
                if (noteMedia.busy()) throw new Error('Wait for the upload to finish.');
                const c = await api('POST', '/cases', {
                    player: nameInput.value.trim(),
                    title: titleInput.value.trim(),
                    priority: prio,
                    note: note.value,
                    evidence: refs,
                    join: join.checked
                });
                toast(`Opened case #${c.id}.`);
                loadCaseSummary();
                onDone?.(c);
                navigate(caseHref(c.id));
            },
        }],
    }));
}

// Adds `refs` to an open case, those on `player` listed first.
function addToCaseDialog({refs, player = '', what = [], title}, onDone) {
    const search = h('input', {
        class: 'input',
        type: 'search',
        placeholder: 'Search open cases by title, player or #',
        autofocus: true,
        'aria-label': 'Search cases'
    });
    const list = h('div', {class: 'list pick-list'}, h('div', {class: 'empty'}, h('div', {class: 'spinner'})));
    let cases = [];
    let close = () => {
    };
    const draw = () => {
        const q = search.value.trim().toLowerCase();
        const shown = cases
            .filter((c) => !q || c.title.toLowerCase().includes(q) || c.player.toLowerCase().includes(q) || `#${c.id}` === q || String(c.id) === q)
            .sort((a, b) => (nameKey(b.player) === nameKey(player)) - (nameKey(a.player) === nameKey(player)));
        replace(list, shown.length
            ? shown.map((c) => caseItem(c, h('div', {class: 'chips item-actions'}, button(`Add to #${c.id}`, {
                small: true, icon: 'plus', on: {
                    click: async () => {
                        await addRefs(c.id, refs);
                        close();
                        onDone?.();
                    }
                }
            }))))
            : h('div', {class: 'empty'}, icon('briefcase'), q ? 'No open case matches.' : 'No open cases.'));
    };
    api('GET', '/cases').then((rows) => {
        cases = rows;
        draw();
    }).catch((e) => replace(list, h('div', {class: 'empty'}, e.message)));
    search.addEventListener('input', draw);
    ({close} = openDialog({
        title: 'Add to a case',
        subtitle: what.length ? `Adding ${what.join(', ')}.` : null,
        width: '600px',
        body: [h('div', {class: 'search'}, icon('search'), search), list],
        actions: [{label: 'Cancel'}, {
            label: 'New case instead',
            icon: 'briefcase',
            run: async () => newCaseDialog({player, refs, what, title}, onDone)
        }],
    }));
}

async function setTriage(key, dismissed, onDone) {
    try {
        await api('POST', '/triage', {key, dismissed});
        toast(dismissed ? 'Dismissed: out of the queue.' : 'Back in the queue.');
        onDone?.();
    } catch (e) {
        toast(e.message, 'error');
    }
}

// What can be done with a report or flagged session in the queue.
function triageButtons({key, refs, player, what, title, priority, triage, onDone}) {
    const inCase = triage.cases?.length > 0;
    return h('div', {class: 'chips item-actions'},
        button('Open case', {
            small: true,
            icon: 'briefcase',
            on: {click: () => newCaseDialog({player, refs, what, title, priority}, onDone)}
        }),
        button('Add to case', {
            small: true,
            icon: 'plus',
            on: {click: () => addToCaseDialog({refs, player, what, title}, onDone)}
        }),
        !inCase && (triage.dismissed
            ? button('Restore', {
                small: true,
                icon: 'undo',
                title: `Dismissed by ${triage.dismissed.by} ${ago(triage.dismissed.at_ms)}`,
                on: {click: () => setTriage(key, false, onDone)}
            })
            : button('Dismiss', {
                small: true,
                icon: 'x',
                title: 'Looked at, nothing to do: takes it out of the queue',
                on: {click: () => setTriage(key, true, onDone)}
            })));
}

// The recording of `name` that was going at `atMs`, from a minute before:
// what a report's snapshot covers.
async function watchAt(name, atMs) {
    try {
        const [r] = await api('GET', `/replays?${query({name, before: atMs + 1})}`);
        if (!r || (!r.live && r.started_ms + r.duration_ms < atMs - 60000)) {
            toast(`No recording of ${name} covers then. Recordings are kept for a few days.`, 'error');
            return;
        }
        navigate(`/replay?${query({id: r.id, t: Math.max(0, atMs - r.started_ms - 60000)})}`);
    } catch (e) {
        toast(e.message, 'error');
    }
}

const watchLink = (name, atMs, label) => h('button', {
    type: 'button',
    class: 'link-button',
    title: `${name}'s recorded session, from a minute before`,
    on: {click: () => watchAt(name, atMs)}
}, icon('play'), label);

function dismissedBadge(d) {
    return d && h('span', {class: 'badge', title: `By ${d.by} ${ago(d.at_ms)}`}, 'Dismissed');
}

// `extra`: more under it, such as what a case page can do with it.
function reportItem(r, onDone, extra) {
    const triaged = r.cases?.length || r.dismissed;
    return h('div', {class: 'list-item'},
        h('div', {class: `list-icon ${triaged ? '' : 'warn'}`}, icon('flag')),
        h('div', {class: 'list-main'},
            h('div', {class: 'list-title'}, h('b', {}, playerLink(r.reporter)), ' reported ', h('b', {}, playerLink(r.offender)),
                ' ', h('span', {class: 'badge'}, ruleName(r.reason)),
                r.muted && h('span', {
                    class: 'badge warn',
                    title: 'The reporter, a moderator, muted them for 48 hours with it'
                }, 'Muted with it'),
                caseLinks(r.cases), dismissedBadge(r.dismissed)),
            h('div', {class: 'list-sub'}, `${r.world ? `World ${r.world}` : `Node ${r.node}`} · ${where(r.x, r.z, r.level)} · `,
                watchLink(r.offender, r.at_ms, 'Their session'), ' ', watchLink(r.reporter, r.at_ms, 'Reporter\'s view')),
            onDone && triageButtons({
                key: `report:${r.id}`, refs: [{type: 'report', id: r.id}], player: r.offender, what: ['the report'],
                title: REPORT_RULES[r.reason] || 'Abuse report', triage: r, onDone,
            }),
            extra),
        h('div', {class: 'list-side', title: fullTime(r.at_ms)}, ago(r.at_ms)));
}

function flaggedItem(r, onDone) {
    return replayItem(r, triageButtons({
        key: `replay:${r.id}`,
        refs: [{type: 'replay', id: r.id}],
        player: r.player,
        what: ['the flagged session'],
        title: `Flagged session: ${FLAGS[r.flags[0]]?.[0] || 'anti-cheat'}`,
        priority: r.score >= 100 ? 'high' : 'normal',
        triage: r,
        onDone,
    }));
}

const card = (title, actions, ...body) => h('section', {class: 'card'},
    h('div', {class: 'card-head'}, h('h2', {}, title), actions && h('div', {class: 'actions'}, actions)), body);
const spinnerList = () => h('div', {class: 'list'}, h('div', {class: 'empty'}, h('div', {class: 'spinner'})));
const failed = (el, e) => replace(el, h('div', {class: 'empty'}, e.message));

const CENTER_TABS = [['overview', 'Overview'], ['triage', 'Triage'], ['cases', 'Cases'], ['sanctions', 'In force'], ['history', 'History'], ['templates', 'Templates']];
// Staff activity is for admins, who coach the moderators.
const centerTabs = () => (state.canCommand ? [...CENTER_TABS, ['staff', 'Staff']] : CENTER_TABS);

function moderationPage(ctx) {
    let tab = centerTabs().some(([k]) => k === ctx.params.get('tab')) ? ctx.params.get('tab') : 'overview';
    const tabs = h('div', {class: 'segmented tabs center-tabs', role: 'group', 'aria-label': 'View'});
    const stats = h('div', {class: 'stats'});
    const body = h('div', {class: 'center-body'});
    let queue = null;
    let sanctions = null;
    let params = ctx.params;

    const go = (next, extra = {}) => {
        tab = next;
        params = new URLSearchParams(query(extra));
        const q = query({tab: next === 'overview' ? null : next, ...extra});
        history.replaceState(null, '', `/moderation${q ? `?${q}` : ''}`);
        draw();
    };

    const statLink = (iconName, label, value, sub, onClick) => h('button', {
            type: 'button',
            class: 'card stat stat-link',
            on: {click: onClick}
        },
        h('div', {class: 'label'}, icon(iconName), label), h('div', {class: 'value num'}, value), h('div', {class: 'sub'}, sub));

    const drawStats = () => {
        const s = state.caseSummary;
        const queued = queue && queue.reports.length + queue.flagged.length;
        const capped = queue && (queue.reports.length >= 50 || queue.flagged.length >= 50);
        replace(stats,
            statLink('briefcase', 'Open cases', s ? s.open : '-', s ? `${s.urgent} high priority · ${s.waiting} waiting` : '', () => go('cases')),
            statLink('user', 'Your cases', s ? s.yours : '-', s ? [s.unread ? `${s.unread} changed since you looked · ` : '', s.unassigned ? `${plural(s.unassigned, 'case')} with nobody on it` : 'Every open case has someone'].join('') : '', () => go('cases', {show: s?.unread ? 'unread' : 'mine'})),
            statLink('inbox', 'To triage', queue ? `${queued}${capped ? '+' : ''}` : '-', queue ? `${plural(queue.reports.length, 'report')} · ${plural(queue.flagged.length, 'flagged session')}` : 'Reports and flagged sessions in no case', () => go('triage')),
            statLink('shield', 'In force', sanctions ? sanctions.length : '-', sanctions
                ? `${sanctions.filter((x) => x.banned_until_ms).length} banned · ${sanctions.filter((x) => x.muted_until_ms).length} muted`
                : 'Mutes and bans', () => go('sanctions')));
    };

    const loadCounts = async () => {
        const [reports, flagged, sanctioned] = await Promise.allSettled([
            api('GET', '/lookup/reports?untriaged=true'),
            api('GET', '/replays?flagged=true&untriaged=true'),
            api('GET', '/lookup/sanctions'),
        ]);
        queue = reports.status === 'fulfilled' && flagged.status === 'fulfilled' ? {
            reports: reports.value,
            flagged: flagged.value
        } : null;
        sanctions = sanctioned.status === 'fulfilled' ? sanctioned.value : null;
        drawStats();
        return {reports, flagged};
    };

    const overview = () => {
        const mine = spinnerList();
        const detected = spinnerList();
        const nobody = spinnerList();
        const triage = spinnerList();
        const actions = spinnerList();
        const closed = spinnerList();
        const appeals = spinnerList();
        const watchlist = spinnerList();
        const take = (c) => button('Take it', {
            small: true, icon: 'plus', on: {
                click: async () => {
                    try {
                        await api('POST', `/cases/${c.id}`, {op: 'join'});
                        toast(`You're on case #${c.id}.`);
                        loadCaseSummary();
                        navigate(caseHref(c.id));
                    } catch (e) {
                        toast(e.message, 'error');
                    }
                }
            }
        });
        const load = async () => {
            api('GET', '/cases?mine=true').then((rows) => replace(mine, rows.length ? rows.map((c) => caseItem(c))
                : h('div', {class: 'empty'}, icon('briefcase'), 'You\'re not on any open case.'))).catch((e) => failed(mine, e));
            api('GET', '/cases?detected=true&unassigned=true').then((rows) => replace(detected, rows.length ? rows.slice(0, 8).map((c) => caseItem(c, h('div', {class: 'chips item-actions'}, take(c))))
                : h('div', {class: 'empty'}, icon('check'), 'Nothing new. The console opens a case on its own for sessions the anti-cheat checks score 50 or more, possible ban evasion, players several others report, and staff lifting a mute or ban on an account linked to theirs. Staff are looked at too: their cases are only for staff above them.'))).catch((e) => failed(detected, e));
            api('GET', '/cases?unassigned=true').then((rows) => replace(nobody, rows.length ? rows.slice(0, 8).map((c) => caseItem(c, h('div', {class: 'chips item-actions'}, take(c))))
                : h('div', {class: 'empty'}, icon('check'), 'Every open case has someone on it.'))).catch((e) => failed(nobody, e));
            api('GET', '/lookup/moderation').then((rows) => replace(actions, rows.length ? rows.slice(0, 8).map(moderationItem)
                : h('div', {class: 'empty'}, 'No moderation on record.'))).catch((e) => failed(actions, e));
            api('GET', '/cases?status=closed').then((rows) => replace(closed, rows.length ? rows.slice(0, 5).map((c) => caseItem(c))
                : h('div', {class: 'empty'}, 'No closed cases yet.'))).catch((e) => failed(closed, e));
            api('GET', '/cases').then((rows) => {
                const waiting = rows.filter((c) => c.appeal && !c.appeal.decision);
                replace(appeals, waiting.length ? waiting.map((c) => caseItem(c, c.appeal.issued_by && isMe(c.appeal.issued_by)
                        ? h('div', {class: 'list-sub'}, 'You issued this one: someone else decides it.') : null))
                    : h('div', {class: 'empty'}, icon('check'), 'No appeals waiting.'));
            }).catch((e) => failed(appeals, e));
            api('GET', '/notes/watched').then((rows) => replace(watchlist, rows.length ? rows.map((n) => h('div', {class: 'list-item'},
                    h('div', {class: `list-icon ${n.world ? 'ok' : ''}`}, icon('eye')),
                    h('div', {class: 'list-main'},
                        h('div', {class: 'list-title'}, h('b', {}, playerLink(n.player)), ' ',
                            n.world ? h('span', {class: 'badge ok'}, `Online · World ${n.world}`) : h('span', {class: 'badge'}, 'Offline')),
                        h('div', {class: 'list-sub'}, `${n.by}: ${clip(n.text, 140)}`)),
                    h('div', {class: 'list-side', title: fullTime(n.at_ms)}, ago(n.at_ms))))
                : h('div', {class: 'empty'}, icon('eye'), 'Nobody is on the watchlist. Add a note with "keep an eye on them" from a player\'s page.'))).catch((e) => failed(watchlist, e));
            const {reports, flagged} = await loadCounts();
            if (reports.status === 'rejected' || flagged.status === 'rejected') {
                failed(triage, reports.reason || flagged.reason);
                return;
            }
            const rows = [
                ...reports.value.map((r) => ({at: r.at_ms, el: () => reportItem(r, load)})),
                ...flagged.value.map((r) => ({at: r.started_ms, el: () => flaggedItem(r, load)})),
            ].sort((a, b) => b.at - a.at).slice(0, 8);
            replace(triage, rows.length ? rows.map((r) => r.el()) : h('div', {class: 'empty'}, icon('check'), 'Nothing waiting: every report and flagged session is in a case or dismissed.'));
        };
        load();
        const more = (label, next, extra) => button(label, {
            small: true,
            kind: 'ghost',
            on: {click: () => go(next, extra)}
        });
        return {
            load, el: h('div', {class: 'split'},
                h('div', {class: 'column'},
                    card('New detections', more('All detected', 'cases', {show: 'detected'}), detected),
                    card('Your cases', more('All open', 'cases'), mine),
                    card('Nobody on these yet', more('Cases', 'cases', {show: 'unassigned'}), nobody),
                    card('Recently closed', more('Closed', 'cases', {show: 'closed'}), closed)),
                h('div', {class: 'column'},
                    card('Appeals to decide', null, appeals),
                    card('To triage', more('Queue', 'triage'), triage),
                    card('Watchlist', null, watchlist),
                    card('Recent actions', more('History', 'history'), actions)))
        };
    };

    const triageTab = () => {
        let kind = params.get('kind') === 'sessions' ? 'sessions' : 'reports';
        let all = params.get('all') === '1';
        const nameInput = h('input', {
            class: 'input',
            type: 'search',
            placeholder: kind === 'reports' ? 'Against (optional)' : 'Player (optional)',
            'aria-label': 'Player',
            value: params.get('name') || '',
            maxlength: '12'
        });
        nameSuggest(nameInput);
        const kinds = h('div', {class: 'chips'});
        const show = h('div', {class: 'chips'});
        const list = h('div', {});
        const note = h('p', {class: 'muted'});
        let paged = null;

        // Picked to act on together, by triage key, and what every row shown
        // is (for picking a run with shift).
        const picked = new Map();
        const shown = new Map();
        let lastBox = null;
        const bulk = h('div', {class: 'bulk-bar hidden', role: 'region', 'aria-label': 'Selected'});
        const allBox = h('input', {type: 'checkbox', 'aria-label': 'Select everything shown'});
        const boxes = () => [...list.querySelectorAll('input.pick')];
        const setPick = (box, on) => {
            box.checked = on;
            box.closest('.list-item')?.classList.toggle('picked', on);
            if (on) picked.set(box.dataset.key, shown.get(box.dataset.key));
            else picked.delete(box.dataset.key);
        };
        const afterBulk = () => {
            picked.clear();
            lastBox = null;
            drawBulk();
            paged?.reload();
            loadCounts();
        };
        const bulkTriage = async (keys, dismissed) => {
            try {
                const {changed} = await api('POST', '/triage/bulk', {keys, dismissed});
                toast(dismissed ? `Dismissed ${changed}: out of the queue.` : `Put ${changed} back in the queue.`);
                afterBulk();
            } catch (e) {
                toast(e.message, 'error');
            }
        };
        const drawBulk = () => {
            const items = [...picked.values()];
            const rendered = boxes();
            allBox.checked = rendered.length > 0 && rendered.every((b) => b.checked);
            allBox.indeterminate = !allBox.checked && rendered.some((b) => b.checked);
            bulk.classList.toggle('hidden', !items.length);
            if (!items.length) return replace(bulk);
            const reports = items.filter((i) => i.ref.type === 'report').length;
            const sessions = items.length - reports;
            const what = [reports && plural(reports, 'report'), sessions && plural(sessions, 'flagged session')].filter(Boolean);
            // The player most of them are about names the case.
            const tally = new Map();
            for (const i of items) tally.set(nameKey(i.player), [i.player, (tally.get(nameKey(i.player))?.[1] || 0) + 1]);
            const [player] = [...tally.values()].sort((a, b) => b[1] - a[1])[0];
            const titles = new Set(items.map((i) => i.title));
            const refs = items.map((i) => i.ref);
            const tooMany = items.length > 100;
            const dismissable = items.filter((i) => !i.inCase && !i.dismissed).map((i) => i.key);
            const restorable = items.filter((i) => i.dismissed).map((i) => i.key);
            const caseArgs = {player, refs, what, title: titles.size === 1 ? items[0].title : ''};
            replace(bulk,
                h('b', {class: 'num'}, `${what.join(' and ')} selected`),
                tally.size > 1 && h('span', {class: 'muted'}, `about ${plural(tally.size, 'player')}`),
                tooMany && h('span', {class: 'badge warn'}, 'At most 100 at once'),
                h('span', {class: 'toolbar-gap'}),
                button('Open case', {
                    small: true,
                    icon: 'briefcase',
                    disabled: tooMany,
                    title: tally.size > 1 ? `On ${player}, who most of them are about` : null,
                    on: {click: () => newCaseDialog({...caseArgs, priority: items.some((i) => i.high) ? 'high' : 'normal'}, afterBulk)}
                }),
                button('Add to case', {small: true, icon: 'plus', disabled: tooMany, on: {click: () => addToCaseDialog(caseArgs, afterBulk)}}),
                !isTrial() && dismissable.length > 0 && button(`Dismiss${dismissable.length < items.length ? ` ${dismissable.length}` : ''}`, {
                    small: true,
                    icon: 'x',
                    disabled: dismissable.length > 100,
                    title: dismissable.length < items.length ? 'Those in a case or dismissed already are left as they are' : 'Looked at, nothing to do: takes them out of the queue',
                    on: {click: () => bulkTriage(dismissable, true)}
                }),
                restorable.length > 0 && button(`Restore${restorable.length < items.length ? ` ${restorable.length}` : ''}`, {
                    small: true,
                    icon: 'undo',
                    disabled: restorable.length > 100,
                    on: {click: () => bulkTriage(restorable, false)}
                }),
                button('Clear', {
                    small: true, kind: 'ghost', on: {
                        click: () => {
                            for (const b of boxes()) setPick(b, false);
                            picked.clear();
                            drawBulk();
                        }
                    }
                }));
        };
        allBox.addEventListener('change', () => {
            for (const b of boxes()) setPick(b, allBox.checked);
            drawBulk();
        });
        // A row with a box to pick it; shift picks the run from the last one.
        const selectable = (info, el) => {
            shown.set(info.key, info);
            const box = h('input', {
                type: 'checkbox',
                class: 'pick',
                'aria-label': 'Select',
                checked: picked.has(info.key),
                dataset: {key: info.key}
            });
            box.addEventListener('click', (e) => {
                const all = boxes();
                const from = e.shiftKey && lastBox ? all.indexOf(lastBox) : -1;
                const to = all.indexOf(box);
                const run = from >= 0 ? all.slice(Math.min(from, to), Math.max(from, to) + 1) : [box];
                for (const b of run) setPick(b, box.checked);
                lastBox = box;
                drawBulk();
            });
            el.prepend(h('label', {class: 'pick-box'}, box));
            el.classList.toggle('picked', box.checked);
            return el;
        };
        const reportInfo = (r) => ({
            key: `report:${r.id}`, ref: {type: 'report', id: r.id}, player: r.offender,
            title: REPORT_RULES[r.reason] || 'Abuse report', inCase: r.cases?.length > 0, dismissed: !!r.dismissed,
        });
        const sessionInfo = (r) => ({
            key: `replay:${r.id}`, ref: {type: 'replay', id: r.id}, player: r.player,
            title: `Flagged session: ${FLAGS[r.flags[0]]?.[0] || 'anti-cheat'}`, high: r.score >= 100,
            inCase: r.cases?.length > 0, dismissed: !!r.dismissed,
        });

        const load = () => {
            picked.clear();
            shown.clear();
            lastBox = null;
            drawBulk();
            const name = nameInput.value.trim();
            if (name && !validName(name)) return replace(list, h('div', {class: 'empty'}, 'That isn\'t a valid player name.'));
            const q = query({tab: 'triage', kind: kind === 'sessions' ? 'sessions' : null, all: all ? 1 : null, name});
            history.replaceState(null, '', `/moderation?${q}`);
            nameInput.placeholder = kind === 'reports' ? 'Against (optional)' : 'Player (optional)';
            note.textContent = kind === 'reports'
                ? 'Abuse reports from the in-game Report Abuse screen. Each carries the minute before it: watch the offender\'s session, or the reporter\'s view.'
                : 'Sessions the anti-cheat checks scored as suspicious. They flag for review only; nobody is banned automatically.';
            if (!all) note.textContent += ' Open a case, add to one, or dismiss what needs nothing, to empty the queue.';
            note.textContent += ' Tick several (shift-click for a run) to deal with them together.';
            const onDone = () => {
                paged.reload();
                loadCounts();
            };
            paged = kind === 'reports'
                ? pagedList((before) => api('GET', `/lookup/reports?${query({
                        name,
                        before,
                        untriaged: all ? null : true
                    })}`),
                    (r) => selectable(reportInfo(r), reportItem(r, onDone)), all ? 'No reports.' : 'Nothing to triage.')
                : pagedList((before) => api('GET', `/replays?${query({
                        name,
                        flagged: true,
                        before,
                        untriaged: all ? null : true
                    })}`),
                    (r) => selectable(sessionInfo(r), flaggedItem(r, onDone)), all ? 'No flagged sessions.' : 'Nothing to triage.', (r) => r.started_ms);
            replace(list, paged.el);
        };
        const chip = (label, pressed, onClick, iconName) => h('button', {
            type: 'button',
            class: 'chip',
            'aria-pressed': String(pressed),
            on: {click: onClick}
        }, iconName && icon(iconName), label);
        const drawChips = () => {
            replace(kinds,
                chip('Reports', kind === 'reports', () => {
                    kind = 'reports';
                    drawChips();
                    load();
                }, 'flag'),
                chip('Flagged sessions', kind === 'sessions', () => {
                    kind = 'sessions';
                    drawChips();
                    load();
                }, 'alert'));
            replace(show,
                chip('Needs triage', !all, () => {
                    all = false;
                    drawChips();
                    load();
                }),
                chip('Everything', all, () => {
                    all = true;
                    drawChips();
                    load();
                }));
        };
        nameInput.addEventListener('input', debounce(load, 400));
        drawChips();
        load();
        return {
            load: () => paged?.reload(), el: [
                h('div', {class: 'toolbar'}, h('div', {class: 'search'}, icon('search'), nameInput), kinds, h('span', {class: 'toolbar-gap'}), show),
                note,
                h('section', {class: 'card'},
                    h('div', {class: 'card-head bulk-head'}, h('label', {class: 'check-row'}, allBox, h('span', {}, 'Select everything shown'))),
                    bulk,
                    list)]
        };
    };

    const casesTab = () => {
        const SHOWS = [['open', 'Open'], ['mine', 'Yours'], ['unread', 'Changed'], ['unassigned', 'Nobody on it'], ['detected', 'Detected'], ['waiting', 'Waiting'], ['closed', 'Closed'], ['all', 'All']];
        let show = SHOWS.some(([k]) => k === params.get('show')) ? params.get('show') : 'open';
        const search = h('input', {
            class: 'input',
            type: 'search',
            placeholder: 'Title, player or #',
            'aria-label': 'Search cases',
            value: params.get('q') || ''
        });
        const chips = h('div', {class: 'chips'});
        const list = spinnerList();
        const count = h('span', {class: 'muted'});
        const markRead = button('Mark all read', {
            small: true,
            kind: 'ghost',
            icon: 'check',
            on: {
                click: async () => {
                    try {
                        state.caseSummary = await api('POST', '/cases/read');
                        updateNavCounts();
                        for (const fn of caseListeners) fn(state.caseSummary);
                        load();
                    } catch (e) {
                        toast(e.message, 'error');
                    }
                }
            }
        });
        markRead.classList.add('hidden');
        let seq = 0;
        const load = async () => {
            const mine = ++seq;
            const q = search.value.trim();
            history.replaceState(null, '', `/moderation?${query({
                tab: 'cases',
                show: show === 'open' ? null : show,
                q
            })}`);
            const filter = {
                mine: {mine: true},
                unread: {unread: true, status: 'all'},
                unassigned: {unassigned: true},
                detected: {detected: true},
                waiting: {status: 'waiting'},
                closed: {status: 'closed'},
                all: {status: 'all'}
            }[show] || {};
            try {
                const rows = await api('GET', `/cases?${query({...filter, q})}`);
                if (mine !== seq) return;
                count.textContent = plural(rows.length, 'case');
                markRead.classList.toggle('hidden', !rows.some((c) => c.unread));
                replace(list, rows.length ? rows.map((c) => caseItem(c)) : h('div', {class: 'empty'}, icon('briefcase'), q ? 'No case matches.' : 'No cases here.'));
            } catch (e) {
                if (mine === seq) failed(list, e);
            }
        };
        const drawChips = () => replace(chips, SHOWS.map(([k, label]) =>
            h('button', {
                type: 'button', class: 'chip', 'aria-pressed': String(show === k), on: {
                    click: () => {
                        show = k;
                        drawChips();
                        load();
                    }
                }
            }, label, k === 'unread' && state.caseSummary?.unread ? h('span', {class: 'chip-count num'}, state.caseSummary.unread) : null,
                k === 'detected' && state.caseSummary?.detected ? h('span', {class: 'chip-count alert num', title: 'Nobody on these yet'}, state.caseSummary.detected) : null)));
        search.addEventListener('input', debounce(load, 250));
        drawChips();
        load();
        return {
            load, el: [
                h('div', {class: 'toolbar'}, h('div', {class: 'search'}, icon('search'), search), chips),
                h('section', {class: 'card'}, h('div', {class: 'card-head'}, h('h2', {}, 'Cases'), h('div', {class: 'actions'}, markRead, count)), scrollWindow(list))]
        };
    };

    const sanctionItem = (s, reload) => h('div', {class: 'list-item'},
        h('div', {class: `list-icon ${s.banned_until_ms ? 'danger' : 'warn'}`}, icon(s.banned_until_ms ? 'ban' : 'mute')),
        h('div', {class: 'list-main'},
            h('div', {class: 'list-title'}, h('b', {}, playerLink(s.name)), ' ',
                s.banned_until_ms && h('span', {class: 'badge danger'}, `Banned · ${until(s.banned_until_ms)}`), ' ',
                s.muted_until_ms && h('span', {class: 'badge warn'}, `Muted · ${until(s.muted_until_ms)}`)),
            h('div', {class: 'list-sub'}, s.moderator
                ? `${s.last_action === 'ban' ? 'Banned' : 'Muted'} by ${s.moderator} ${ago(s.at_ms)}`
                : 'No moderator on record (abuse report or older)')),
        h('div', {class: 'chips'},
            !isTrial() && s.muted_until_ms && button('Unmute', {
                small: true,
                icon: 'undo',
                on: {click: () => moderateDialog(s.name, 'unmute', reload)}
            }),
            !isTrial() && s.banned_until_ms && button('Unban', {
                small: true,
                icon: 'undo',
                on: {click: () => moderateDialog(s.name, 'unban', reload)}
            })));

    const sanctionsTab = () => {
        const list = spinnerList();
        const count = h('span', {class: 'muted'});
        const load = async () => {
            try {
                const rows = await api('GET', '/lookup/sanctions');
                sanctions = rows;
                drawStats();
                count.textContent = plural(rows.length, 'player');
                replace(list, rows.length ? rows.map((s) => sanctionItem(s, load)) : h('div', {class: 'empty'}, icon('shield'), 'Nobody is muted or banned.'));
            } catch (e) {
                failed(list, e);
            }
        };
        load();
        return {
            load,
            el: h('section', {class: 'card'}, h('div', {class: 'card-head'}, h('h2', {}, 'In force now'), h('div', {class: 'actions'}, count)), scrollWindow(list))
        };
    };

    const historyTab = () => {
        const paged = pagedList((before) => api('GET', `/lookup/moderation?${query({before})}`), moderationItem, 'No moderation on record.');
        return {
            load: paged.reload,
            el: h('section', {class: 'card'}, h('div', {class: 'card-head'}, h('h2', {}, 'Every action, in game or here, newest first')), paged.el)
        };
    };

    let current = null;
    const draw = () => {
        replace(tabs, centerTabs().map(([k, label]) =>
            h('button', {type: 'button', 'aria-pressed': String(tab === k), on: {click: () => go(k)}}, label)));
        current = {
            overview,
            triage: triageTab,
            cases: casesTab,
            sanctions: sanctionsTab,
            history: historyTab,
            templates: templatesTab,
            staff: () => staffActivityTab(params, go),
        }[tab]();
        replace(body, current.el);
    };
    const reload = () => current?.load();

    drawStats();
    loadCaseSummary();
    if (tab !== 'overview') loadCounts().catch(() => {
    });
    ctx.cleanup(onCases(drawStats));
    // Kept fresh while it's open, except under a dialog or on paged lists.
    const timer = setInterval(() => {
        if (document.hidden || document.querySelector('dialog[open]')) return;
        if (tab === 'overview') reload();
        else loadCounts().catch(() => {
        });
    }, 30000);
    ctx.cleanup(() => clearInterval(timer));
    draw();

    return h('div', {class: 'page'},
        pageHead('Moderation center', 'Cases staff work on together, the reports and flagged sessions waiting to be looked at, and every action taken.',
            button('Moderate by name', {icon: 'shield', on: {click: () => moderateDialog('', 'mute', reload)}}),
            button('Open a case', {icon: 'briefcase', kind: 'primary', on: {click: () => newCaseDialog({}, reload)}})),
        stats,
        tabs,
        body);
}

// Staff activity: what each staff member did, how their sanctions fare on
// appeal and how quickly they get to cases. For coaching, admins only.

const ACTIVITY_WINDOWS = [[7, '7 days'], [30, '30 days'], [90, '90 days']];
// An overturn rate worth a look, once enough of their sanctions were appealed.
const OVERTURN_WATCH = {appealed: 3, rate: 0.3};

function staffActivityTab(params, go) {
    const days = ACTIVITY_WINDOWS.some(([d]) => String(d) === params.get('days')) ? Number(params.get('days')) : 30;
    let trialsOnly = params.get('trials') === '1';
    const body = h('div', {class: 'table-wrap'}, spinnerList());
    const chips = h('div', {class: 'chips'});
    const note = h('p', {class: 'muted'},
        'Sanctions come from the audit log (those that took), cases and dismissals from the case log. Overturned counts appeals against their mutes and bans that were decided in the window. Pickup is how long a case waited, from opening, for their first change on it.');
    let rows = null;
    const took = (ms) => (ms == null ? '-' : span(ms / 1000));
    const draw = () => {
        replace(chips,
            ACTIVITY_WINDOWS.map(([d, label]) => h('button', {
                type: 'button',
                class: 'chip',
                'aria-pressed': String(days === d),
                on: {click: () => go('staff', {days: d, trials: trialsOnly ? 1 : null})}
            }, label)),
            h('span', {class: 'toolbar-gap'}),
            h('button', {
                type: 'button',
                class: 'chip',
                'aria-pressed': String(trialsOnly),
                on: {
                    click: () => {
                        trialsOnly = !trialsOnly;
                        go('staff', {days, trials: trialsOnly ? 1 : null});
                    }
                }
            }, 'Trial moderators only'));
        if (!rows) return;
        const shown = rows.filter((a) => !trialsOnly || a.role === 'trial');
        if (!shown.length) {
            replace(body, h('div', {class: 'empty'}, icon('users'), trialsOnly ? 'No trial moderators.' : 'Nobody did anything in this window.'));
            return;
        }
        replace(body, h('table', {class: 'table staff-activity'},
            h('thead', {}, h('tr', {},
                h('th', {}, 'Staff'),
                h('th', {class: 'num', title: 'Kicks / mutes / bans / lifted'}, 'Kick · mute · ban · lift'),
                h('th', {class: 'num', title: 'Cases opened / closed, and comments'}, 'Cases'),
                h('th', {class: 'num', title: 'Reports and sessions set aside from triage'}, 'Dismissed'),
                h('th', {class: 'num'}, 'Appeals decided'),
                h('th', {class: 'num', title: 'Appeals against their mutes and bans that were overturned / decided'}, 'Overturned'),
                h('th', {class: 'num', title: 'Median time from a case opening to their first change on it'}, 'Pickup'),
                h('th', {class: 'num', title: 'Median time from a case opening to their closing it'}, 'To close'),
                h('th', {class: 'num', title: 'Median time from an appeal coming in to their decision'}, 'To decide'),
                h('th', {}, 'Last active'))),
            h('tbody', {}, shown.map((a) => {
                const rate = a.appealed ? a.overturned / a.appealed : 0;
                const watch = a.appealed >= OVERTURN_WATCH.appealed && rate >= OVERTURN_WATCH.rate;
                return h('tr', {},
                    h('td', {}, h('div', {class: 'name-cell'}, a.role && isMaintainer() ? staffLink(a.name) : playerLink(a.name),
                        a.role ? h('span', {class: `badge ${a.role === 'trial' ? 'warn' : ''}`}, ROLE_NAMES[a.role] || a.role)
                            : h('span', {class: 'badge', title: 'No console account: their in-game sanctions were appealed'}, 'In game'))),
                    h('td', {class: 'num'}, `${a.kicks} · ${a.mutes} · ${a.bans} · ${a.lifts}`),
                    h('td', {class: 'num', title: `${a.cases_opened} opened, ${a.cases_closed} closed, ${plural(a.comments, 'comment')}`},
                        `${a.cases_opened} / ${a.cases_closed}`, h('span', {class: 'muted'}, ` · ${a.comments}`)),
                    h('td', {class: 'num'}, a.dismissed),
                    h('td', {class: 'num'}, a.appeals_decided),
                    h('td', {class: `num ${watch ? 'tone-warn' : ''}`},
                        a.appealed ? `${a.overturned} / ${a.appealed}` : h('span', {class: 'muted'}, '-'),
                        a.appealed > 0 && h('span', {class: 'muted'}, ` (${Math.round(rate * 100)}%)`),
                        a.overturned_cases.length > 0 && h('div', {class: 'chips nowrap case-refs'}, caseLinks(a.overturned_cases))),
                    h('td', {class: 'num', title: a.pickups ? `Over ${plural(a.pickups, 'case')}` : ''}, took(a.pickup_ms)),
                    h('td', {class: 'num'}, took(a.close_ms)),
                    h('td', {class: 'num'}, took(a.decide_ms)),
                    h('td', {class: 'muted', title: a.last_active_ms ? fullTime(a.last_active_ms) : ''}, a.last_active_ms ? ago(a.last_active_ms) : 'Not in this window'));
            }))));
    };
    draw();
    api('GET', `/staff/activity?${query({days})}`).then((found) => {
        rows = found;
        draw();
    }).catch((e) => failed(body, e));

    return {
        load: () => {
        },
        el: h('div', {class: 'column'},
            h('div', {class: 'toolbar'}, chips),
            note,
            h('section', {class: 'card'}, body))
    };
}

// Saved closing notes and appeal decisions (edited in the Templates tab),
// filled in for the case they're used on.
const CANNED_USES = [['close', 'Closing note'], ['upheld', 'Appeal upheld'], ['overturned', 'Appeal overturned']];
const CANNED_HINT = '{player}, {sanction} and {case} are filled in for the case it\'s used on.';

async function loadCanned() {
    state.canned = (await api('GET', '/canned')).canned;
    return state.canned;
}

const fillCanned = (text, c) => text.replaceAll('{player}', c.player).replaceAll('{case}', `#${c.id}`)
    .replaceAll('{sanction}', c.appeal?.sanction || 'sanction');

// Chips that put a saved text for `use` into `textarea`: in place of what's
// there if that's nothing or another saved text left as it was, else after
// it. `onPick` hears which.
function cannedChips(use, c, textarea, onPick) {
    const el = h('div', {class: 'chips canned-chips'});
    let inserted = null;
    const draw = (all) => {
        const mine = all.filter((t) => t.use === use);
        replace(el, mine.length > 0 && [h('span', {class: 'hint'}, 'Saved:'), mine.map((t) => h('button', {
            type: 'button',
            class: 'chip',
            title: t.text,
            on: {
                click: () => {
                    const text = fillCanned(t.text, c);
                    const now = textarea.value.trim();
                    textarea.value = !now || now === inserted ? text : `${textarea.value.trimEnd()}\n\n${text}`;
                    inserted = textarea.value.trim();
                    textarea.focus();
                    onPick?.(t);
                }
            }
        }, t.name))]);
    };
    if (state.canned) draw(state.canned);
    loadCanned().then(draw).catch(() => {
    });
    return el;
}

// ---------------------------------------------------------------------------
// A case: its evidence, who's on it, and a timeline of everything done,
// comments included. It refreshes every few seconds, so staff on it together
// see each other's changes, and who else has it open.

function evidenceLabel(e) {
    if (e.type === 'report') return `${e.reporter}'s report of ${e.offender} (${REPORT_RULES[e.reason] || 'unknown rule'})`;
    if (e.type === 'replay') return `${e.player}'s session of ${fullTime(e.started_ms)}`;
    if (e.type === 'clip') return `a clip of ${e.player}'s session (${clipSpan(e.from_ms, e.to_ms)})${e.label ? `: "${clip(e.label, 60)}"` : ''}`;
    if (e.type === 'moderation') return `${e.moderator}'s ${e.action} of ${e.target}`;
    if (e.type === 'evasion') return `${e.player}'s logins ${e.via === 'address' ? 'from the address' : 'with the browser'} of banned ${e.banned}`;
    return `${e.from}'s message "${clip(e.message, 60)}"`;
}

function evidenceItem(e, onRemove) {
    const remove = onRemove && button(null, {
        small: true,
        kind: 'ghost',
        icon: 'x',
        'aria-label': 'Remove from the case',
        title: 'Remove from the case',
        on: {click: onRemove}
    });
    const row = (iconName, tone, title, sub, side) => h('div', {class: 'list-item'},
        h('div', {class: `list-icon ${tone}`}, icon(iconName)),
        h('div', {class: 'list-main'}, h('div', {class: 'list-title'}, title), h('div', {class: 'list-sub'}, sub)),
        h('div', {class: 'list-side', title: fullTime(side)}, ago(side)),
        remove);
    if (e.type === 'report') {
        return row('flag', 'warn',
            [h('b', {}, playerLink(e.reporter)), ' reported ', h('b', {}, playerLink(e.offender)), ' ', h('span', {class: 'badge'}, ruleName(e.reason))],
            [`${worldOf(e.node)} · ${where(e.x, e.z, e.level)} · `, watchLink(e.offender, e.at_ms, 'Their session'), ' ', watchLink(e.reporter, e.at_ms, 'Reporter\'s view')],
            e.at_ms);
    }
    if (e.type === 'replay') {
        return row(e.score > 0 ? 'alert' : 'play', e.score > 0 ? 'danger' : '',
            [link(`/replay?${query({id: e.id})}`, {class: 'name-link'}, h('b', {}, `${e.player}'s session`)), ' ', e.flags.map(flagBadge)],
            `${worldOf(e.node)} · ${fullTime(e.started_ms)} · ${clock(e.duration_ms / 1000)}${e.score > 0 ? ` · score ${e.score}` : ''}`,
            e.started_ms);
    }
    if (e.type === 'clip') {
        return row('scissors', 'accent',
            [link(clipHref(e), {class: 'name-link'}, h('b', {}, e.label || `${e.player}'s clip`)), ' ', e.flags.map(flagBadge)],
            [e.label && [playerLink(e.player), ' · '], `${clipSpan(e.from_ms, e.to_ms)} (${clock((e.to_ms - e.from_ms) / 1000)}) into the session of ${fullTime(e.started_ms)} · ${worldOf(e.node)}`],
            e.started_ms + e.from_ms);
    }
    if (e.type === 'moderation') {
        return row(e.action === 'unmute' ? 'unlock' : 'undo', 'danger',
            [h('b', {}, playerLink(e.moderator)), ` lifted ${e.action === 'unmute' ? 'the mute' : 'the ban'} on `, h('b', {}, playerLink(e.target))],
            joined([`${e.target} ${linkedHow(e.via)} ${e.moderator}`, e.reason && `Reason: ${e.reason}`, fullTime(e.at_ms)]),
            e.at_ms);
    }
    if (e.type === 'evasion') {
        const address = e.via === 'address';
        return row(address ? 'globe' : 'monitor', address ? 'warn' : 'danger',
            [h('b', {}, playerLink(e.player)), address ? ' logged in from the address of banned ' : ' logged in with the browser of banned ', h('b', {}, playerLink(e.banned))],
            joined([`${plural(e.logins, 'login')} since the ban`, `first ${ago(e.first_ms)}`, h('span', {title: fullTime(e.banned_ms)}, `banned ${ago(e.banned_ms)}`)]),
            e.last_ms);
    }
    return row('chat', '',
        [h('b', {}, playerLink(e.from)), e.to && [h('span', {class: 'muted'}, ' to '), h('b', {}, playerLink(e.to))]],
        [h('span', {class: `chat-text ${e.to ? 'private' : ''}`}, e.message), h('br'), `${worldOf(e.node)} · ${fullTime(e.at_ms)}`],
        e.at_ms);
}

// One timeline entry, in words: [what, icon, tone, body].
function describeEntry(e, timeline) {
    const b = (text) => h('b', {}, text);
    const text = (t) => t && richText(e.html, t);
    switch (e.kind) {
        case 'opened':
            return [['opened the case on ', b(e.player)], 'briefcase', 'accent'];
        case 'comment':
            return ['', 'chat', '', text(e.text)];
        case 'assigned':
            return [e.name === e.by ? 'joined the case' : ['put ', b(e.name), ' on the case'], 'plus', 'accent'];
        case 'unassigned':
            return [e.name === e.by ? 'left the case' : ['took ', b(e.name), ' off the case'], 'logout', ''];
        case 'status':
            return [['set the status to ', b(CASE_STATUS[e.status]?.[0] || e.status)], 'activity', ''];
        case 'priority':
            return [['set the priority to ', b(PRIORITIES.find(([k]) => k === e.priority)?.[1] || e.priority)], 'alert', PRIORITY_TONE[e.priority] || ''];
        case 'retitled':
            return [['renamed it ', b(`"${e.title}"`)], 'edit', ''];
        case 'evidence_added':
            return [['added ', b(evidenceLabel(e.evidence))], 'clip', ''];
        case 'evidence_removed': {
            const was = timeline.find((x) => x.kind === 'evidence_added' && evidenceKey(x.evidence) === e.key);
            return [['removed ', b(was ? evidenceLabel(was.evidence) : e.key)], 'trash', ''];
        }
        case 'action': {
            const [verb, iconName, tone] = MOD_VERBS[e.action] || [e.action, 'shield', ''];
            const what = e.ok
                ? [`${verb} `, playerLink(e.name), e.hours ? ` for ${hoursLabel(e.hours)}` : '']
                : [`tried to ${e.action} `, playerLink(e.name), ' ', h('span', {class: 'badge danger'}, 'Failed')];
            // The reason starts with the case it came from, which is this one.
            const why = (e.reason || '').replace(/^Case #\d+:?\s*/, '');
            return [what, iconName, e.ok ? tone : 'danger', h('div', {class: 'list-sub'}, e.message, why && ` Reason: ${why}`)];
        }
        case 'closed':
            return [['closed it: ', b(RESOLUTIONS[e.resolution] || e.resolution)], 'check', 'ok', text(e.note)];
        case 'reopened':
            return ['reopened the case', 'undo', 'accent'];
        case 'appeal_filed':
            return [['received an appeal against a ', b(e.appeal.sanction)], 'flag', 'warn',
                h('div', {class: 'appeal-statement'}, e.appeal.statement)];
        case 'appeal_decided':
            return [['decided the appeal: ', b(e.decision === 'overturned' ? 'Overturned' : 'Upheld')],
                e.decision === 'overturned' ? 'undo' : 'check', e.decision === 'overturned' ? 'accent' : 'ok',
                e.note && h('div', {class: 'list-sub'}, 'Told the player: ', e.note)];
        default:
            return [e.kind.replace(/_/g, ' '), 'activity', ''];
    }
}

function caseEntry(e, timeline) {
    const [what, iconName, tone, body] = describeEntry(e, timeline);
    const comment = e.kind === 'comment';
    return h('div', {class: `list-item case-entry ${comment ? 'comment' : ''}`},
        comment ? h('div', {class: 'avatar'}, initial(e.by)) : h('div', {class: `list-icon ${tone}`}, icon(iconName)),
        h('div', {class: 'list-main'}, h('div', {class: 'list-title'}, h('b', {}, e.by), ' ', what), body),
        h('div', {class: 'list-side', title: fullTime(e.at_ms)}, ago(e.at_ms)));
}

// Evidence to add to case `c`: reports against a player (the case's, or
// anyone's, such as the other side of a scam), their sessions, or chat.
function evidenceDialog(c, onAdded) {
    let kind = 'reports';
    const have = new Set(c.evidence.map(evidenceKey));
    const nameInput = h('input', {
        class: 'input',
        type: 'search',
        value: c.player,
        maxlength: '12',
        placeholder: 'Player name',
        'aria-label': 'Player'
    });
    nameSuggest(nameInput);
    const kinds = h('div', {class: 'segmented'});
    const body = h('div', {class: 'pick-list'});
    const add = (item) => {
        const key = `${item.type}:${item.id}`;
        const done = have.has(key);
        const btn = button(done ? 'Added' : 'Add', {small: true, icon: done ? 'check' : 'plus', disabled: done});
        btn.addEventListener('click', async () => {
            btn.disabled = true;
            try {
                const detail = await api('POST', `/cases/${c.id}`, {op: 'add_evidence', item});
                have.add(key);
                replace(btn, icon('check'), 'Added');
                onAdded(detail);
            } catch (e) {
                btn.disabled = false;
                toast(e.message, 'error');
            }
        });
        return btn;
    };
    const pick = (iconName, title, sub, action) => h('div', {class: 'list-item'},
        h('div', {class: 'list-icon'}, icon(iconName)),
        h('div', {class: 'list-main'}, h('div', {class: 'list-title'}, title), h('div', {class: 'list-sub'}, sub)),
        action);
    const load = () => {
        const who = nameInput.value.trim();
        if (!validName(who)) return replace(body, h('div', {class: 'empty'}, 'Enter a player name.'));
        const specs = {
            reports: [(before) => api('GET', `/lookup/reports?${query({name: who, before})}`),
                (r) => pick('flag', [h('b', {}, r.reporter), ' reported ', h('b', {}, r.offender), ' ', h('span', {class: 'badge'}, ruleName(r.reason))],
                    `${worldOf(r.node)} · ${fullTime(r.at_ms)}`, add({type: 'report', id: r.id})),
                `No reports against ${who}.`, (r) => r.id],
            sessions: [(before) => api('GET', `/replays?${query({name: who, before})}`),
                (r) => pick(r.score > 0 ? 'alert' : 'play', [h('b', {}, `${r.player}'s session`), ' ', r.flags.map(flagBadge)],
                    `${worldOf(r.node)} · ${fullTime(r.started_ms)} · ${clock(r.duration_ms / 1000)}${r.score > 0 ? ` · score ${r.score}` : ''}`, add({
                        type: 'replay',
                        id: r.id
                    })),
                `No recorded sessions of ${who}.`, (r) => r.started_ms],
            chat: [(before) => api('GET', `/lookup/chat?${query({name: who, before})}`),
                (m) => pick('chat', [h('b', {}, m.from), m.to && [' to ', h('b', {}, m.to)], ': ', h('span', {class: `chat-text ${m.to ? 'private' : ''}`}, m.message)],
                    `${worldOf(m.node)} · ${fullTime(m.at_ms)}`, add({type: 'chat', id: m.id})),
                `No chat from ${who}.`, (m) => m.id],
        };
        const [fetchPage, item, empty, cursor] = specs[kind];
        replace(body, pagedList(fetchPage, item, empty, cursor).el);
    };
    const drawKinds = () => replace(kinds, [['reports', 'Reports against'], ['sessions', 'Sessions'], ['chat', 'Chat']].map(([k, label]) =>
        h('button', {
            type: 'button', 'aria-pressed': String(kind === k), on: {
                click: () => {
                    kind = k;
                    drawKinds();
                    load();
                }
            }
        }, label)));
    nameInput.addEventListener('input', debounce(load, 400));
    drawKinds();
    load();
    openDialog({
        title: 'Add evidence',
        subtitle: `To case #${c.id}. Looking at chat here is recorded in the audit log.`,
        width: '680px',
        body: [h('div', {class: 'evidence-tools'}, h('div', {class: 'search'}, icon('search'), nameInput), kinds), body],
        actions: [{label: 'Done'}],
    });
}

function casePage(ctx) {
    const id = Number(ctx.params.get('id'));
    const page = h('div', {class: 'page'});
    const head = h('div', {}, pageHead(`Case #${ctx.params.get('id') || ''}`, 'Loading...'));
    append(page, head);
    if (!Number.isInteger(id) || id < 1) {
        replace(head, pageHead('Case', 'No such case.'));
        return page;
    }

    const presence = h('div', {class: 'presence hidden', role: 'status'});
    const evidenceList = h('div', {class: 'list'});
    const evidenceCount = h('span', {class: 'muted'});
    const timeline = h('div', {class: 'list case-timeline'});
    const playerBox = h('div', {class: 'card-body'}, h('div', {class: 'spinner'}));
    const caseBox = h('div', {class: 'card-body'});
    const others = spinnerList();
    const reportsBox = h('div', {}, spinnerList());
    const pastActions = spinnerList();
    const layout = h('div', {class: 'case-layout hidden'});
    let c = null;
    let record;

    const draftKey = `caseDraft:${id}`;
    const composer = h('textarea', {
        class: 'input composer-text',
        maxlength: '4000',
        placeholder: 'Write a comment for everyone on the case. Markdown works; paste or drop in images, GIFs and videos.',
        'aria-label': 'Comment',
        value: prefs.get(draftKey, '')
    });
    const composerMedia = mediaUploader(composer, caseUpload, 'Attach');
    const send = button('Comment', {kind: 'primary', icon: 'chat'});
    const previewBox = h('div', {class: 'composer-preview hidden'});
    const previewButton = button('Preview', {small: true, kind: 'ghost', icon: 'eye', 'aria-pressed': 'false'});
    let previewing = false;
    const setPreview = async (on) => {
        previewing = on;
        previewButton.setAttribute('aria-pressed', String(on));
        composer.classList.toggle('hidden', on);
        previewBox.classList.toggle('hidden', !on);
        if (!on) return composer.focus();
        replace(previewBox, h('div', {class: 'spinner'}));
        try {
            const {html} = await api('POST', '/cases/preview', {markdown: composer.value});
            replace(previewBox, composer.value.trim() ? richText(html, composer.value) : h('p', {class: 'muted'}, 'Nothing to preview yet.'));
        } catch (e) {
            replace(previewBox, h('p', {class: 'error-text'}, e.message));
        }
    };
    previewButton.addEventListener('click', () => setPreview(!previewing));
    const addEvidence = button('Add evidence', {small: true, icon: 'plus'});

    const op = async (body) => {
        const detail = await api('POST', `/cases/${id}`, body);
        show(detail, true);
        loadCaseSummary();
        return detail;
    };
    const run = (body, done) => async () => {
        try {
            await op(body);
            if (done) toast(done);
        } catch (e) {
            toast(e.message, 'error');
        }
    };
    const open = () => c.status !== 'closed';
    const afterAction = () => {
        refresh();
        loadPlayer();
        loadHistory();
    };

    const retitleDialog = () => {
        const input = h('input', {
            class: 'input',
            value: c.title,
            maxlength: '100',
            required: true,
            autofocus: true,
            autocomplete: 'off'
        });
        openDialog({
            title: 'Rename case',
            subtitle: `#${c.id}`,
            body: [h('label', {class: 'field'}, h('span', {}, 'Title'), input)],
            actions: [{label: 'Cancel'}, {
                label: 'Rename',
                kind: 'primary',
                run: () => op({op: 'retitle', title: input.value.trim()})
            }],
        });
    };

    const closeDialog = () => {
        let resolution = c.actions > 0 ? 'actioned' : 'no_action';
        const seg = h('div', {class: 'segmented', role: 'group', 'aria-label': 'Outcome'});
        const hint = h('p', {class: 'hint'});
        const note = h('textarea', {
            class: 'input',
            maxlength: '4000',
            placeholder: 'What was found and what was done. Optional, but the next person to look will thank you.'
        });
        const noteMedia = mediaUploader(note, caseUpload, 'Attach');
        // A saved note picks its outcome too.
        const saved = cannedChips('close', c, note, (t) => {
            if (!t.resolution) return;
            resolution = t.resolution;
            draw();
        });
        const draw = () => {
            replace(seg, Object.entries(RESOLUTIONS).map(([k, label]) =>
                h('button', {
                    type: 'button', 'aria-pressed': String(resolution === k), on: {
                        click: () => {
                            resolution = k;
                            draw();
                        }
                    }
                }, label)));
            hint.textContent = {
                actioned: c.actions ? `${plural(c.actions, 'action')} sent from this case.` : 'No moderation was sent from this case: say in the note what was done.',
                no_action: 'Looked into: nothing to do.',
                duplicate: 'Another case covers it: say which in the note.',
            }[resolution];
        };
        draw();
        openDialog({
            title: `Close case #${c.id}`,
            subtitle: c.title,
            body: [
                h('div', {class: 'field'}, h('span', {}, 'Outcome'), seg, hint),
                h('div', {class: 'field'}, h('span', {}, 'Note'), saved, note,
                    h('div', {class: 'news-tools'}, noteMedia.button, noteMedia.status, noteMedia.input), h('span', {class: 'hint'}, MARKDOWN_HINT)),
                c.assignees.length > 1 && h('div', {class: 'callout'}, icon('users'), h('div', {}, `Closes it for everyone on it: ${listNames(c.assignees)}.`)),
            ],
            actions: [{label: 'Cancel'}, {
                label: 'Close case', kind: 'primary', icon: 'check', run: async () => {
                    if (noteMedia.busy()) throw new Error('Wait for the upload to finish.');
                    await op({op: 'close', resolution, note: note.value});
                    toast(`Closed case #${c.id}.`);
                }
            }],
        });
    };

    // An appeal's sanction was issued by this staff member: they can't decide it.
    const issuedByMe = () => !!c.appeal?.issued_by && isMe(c.appeal.issued_by);
    const decideDialog = (decision) => {
        const a = c.appeal;
        const overturn = decision === 'overturned';
        const note = h('textarea', {
            class: 'input', maxlength: '2000', placeholder: overturn
                ? 'Optional. e.g. "We looked again and agree the ban was a mistake."'
                : 'Optional. e.g. "Our logs show the account was botting."', autofocus: true
        });
        const saved = cannedChips(decision, c, note);
        openDialog({
            title: overturn ? `Overturn the ${a.sanction}` : `Uphold the ${a.sanction}`,
            subtitle: `${c.player}'s appeal #${a.id}`,
            body: [
                h('div', {class: `callout ${overturn ? '' : 'warn'}`}, icon(overturn ? 'undo' : 'check'), h('div', {}, overturn
                    ? `${c.player}'s ${a.sanction} is lifted now, they're told it was overturned, and the case closes.`
                    : `The ${a.sanction} stands. ${c.player} is told it was upheld, and the case closes. They can't appeal this ${a.sanction} again.`)),
                h('div', {class: 'field'}, h('span', {}, 'Note to the player'), saved, note,
                    h('span', {class: 'hint'}, 'Shown to them on the website\'s appeal status page, as written. Plain text.')),
            ],
            actions: [{label: 'Cancel'}, {
                label: overturn ? 'Overturn' : 'Uphold',
                kind: overturn ? 'primary' : 'danger',
                icon: overturn ? 'undo' : 'check',
                run: async () => {
                    await op({op: 'decide_appeal', decision, note: note.value});
                    toast(overturn ? `Overturned: ${c.player}'s ${a.sanction} is lifted.` : 'Upheld.');
                    afterAction();
                },
            }],
        });
    };
    const appealBox = h('div', {});
    const drawAppeal = () => {
        const a = c.appeal;
        if (!a) return replace(appealBox);
        const facts = [
            ['Appealing', `${a.sanction === 'ban' ? 'Ban' : 'Mute'} until ${fullTime(a.until_ms)}`],
            ['Issued by', a.issued_by || 'Nobody (an abuse report\'s mute, or the anti-cheat)'],
            a.reason && ['Their reason', a.reason],
            ['Sent', `${fullTime(a.filed_ms)} (${ago(a.filed_ms)})`],
            a.decision && ['Decision', `${a.decision === 'overturned' ? 'Overturned' : 'Upheld'} by ${a.decided_by}`],
            a.note && ['Told them', a.note],
        ].filter(Boolean);
        const blocked = issuedByMe();
        replace(appealBox, h('section', {class: 'card'},
            h('div', {class: 'card-head'}, h('h2', {}, `Appeal #${a.id}`), h('div', {class: 'actions'}, appealBadge(c))),
            h('div', {class: 'card-body'},
                h('div', {class: 'appeal-statement'}, a.statement),
                h('dl', {class: 'facts'}, facts.map(([k, v]) => [h('dt', {}, k), h('dd', {}, v)])),
                open() && (blocked || isTrial()
                    ? h('div', {class: 'callout'}, icon('shield'), h('div', {}, blocked
                        ? 'You issued this sanction, so someone else must decide the appeal.'
                        : 'Trial moderators don\'t decide appeals: comment with what you found, and a moderator decides.'))
                    : h('div', {class: 'chips'},
                        button('Uphold', {icon: 'check', on: {click: () => decideDialog('upheld')}}),
                        button('Overturn', {
                            icon: 'undo',
                            kind: 'primary',
                            on: {click: () => decideDialog('overturned')}
                        }))))));
    };

    const assignDialog = async () => {
        const list = h('div', {class: 'chips'}, h('div', {class: 'spinner'}));
        const {close} = openDialog({
            title: 'Put staff on the case',
            subtitle: 'It shows under Your cases for them.',
            body: [list],
            actions: [{label: 'Done'}]
        });
        try {
            const staff = await api('GET', '/cases/staff');
            const free = staff.filter((s) => !c.assignees.some((a) => nameKey(a) === nameKey(s.name)));
            replace(list, free.length ? free.map((s) => button(s.name, {
                small: true, icon: 'plus', title: s.role, on: {
                    click: async () => {
                        try {
                            await op({op: 'assign', name: s.name});
                            toast(`${s.name} is on the case.`);
                            close();
                        } catch (e) {
                            toast(e.message, 'error');
                        }
                    }
                }
            })) : h('p', {class: 'muted'}, 'Everyone is on it already.'));
        } catch (e) {
            replace(list, h('p', {class: 'error-text'}, e.message));
        }
    };

    const drawHead = () => {
        const on = c.assignees.some(isMe);
        replace(head, pageHead(
            h('span', {}, h('span', {class: 'case-id num'}, `#${c.id}`), c.title, ' ', statusBadge(c), ' ', priorityBadge(c.priority)),
            h('span', {}, 'On ', playerLink(c.player), ` · opened by ${c.created_by} ${ago(c.created_ms)}`,
                c.closed_ms ? ` · closed by ${c.closed_by} ${ago(c.closed_ms)}` : ''),
            open() && button(null, {icon: 'edit', 'aria-label': 'Rename', title: 'Rename', on: {click: retitleDialog}}),
            open() && button(on ? 'Leave' : 'Join', {
                icon: on ? 'logout' : 'plus',
                on: {click: run({op: on ? 'leave' : 'join'}, on ? 'You left the case.' : 'You\'re on the case.')}
            }),
            open() && button('Moderate', {
                icon: 'shield',
                on: {click: () => moderateDialog(c.player, 'mute', afterAction, c.id)}
            }),
            !open()
                ? button('Reopen', {icon: 'undo', on: {click: run({op: 'reopen'}, 'Reopened.')}})
                : c.appeal && !c.appeal.decision
                    ? !issuedByMe() && button('Decide appeal', {
                    icon: 'flag',
                    kind: 'primary',
                    on: {click: () => appealBox.scrollIntoView({block: 'center'})}
                })
                    : button('Close case', {icon: 'check', kind: 'primary', on: {click: closeDialog}})));
    };

    const drawPresence = () => {
        presence.classList.toggle('hidden', !c.viewing.length);
        replace(presence, h('span', {class: 'dot ok'}), `${listNames(c.viewing)} ${c.viewing.length === 1 ? 'has' : 'have'} this case open too.`);
    };

    const drawEvidence = () => {
        evidenceCount.textContent = c.evidence.length ? plural(c.evidence.length, 'item') : '';
        addEvidence.disabled = !open();
        const remove = (e) => open() && (() => confirmDialog({
            title: 'Remove evidence',
            subtitle: evidenceLabel(e),
            body: [h('p', {class: 'muted'}, 'It comes out of the case. The timeline keeps a note that it was there.')],
            label: 'Remove',
            kind: 'danger',
            run: () => op({op: 'remove_evidence', key: evidenceKey(e)}),
        }));
        replace(evidenceList, c.evidence.length
            ? c.evidence.map((e) => evidenceItem(e, remove(e)))
            : h('div', {class: 'empty'}, icon('clip'), 'No evidence yet: add reports, sessions or chat.'));
    };

    const drawTimeline = () => {
        const shown = timeline.children.length;
        const atEnd = timeline.getBoundingClientRect().bottom <= window.innerHeight + 40;
        replace(timeline, c.timeline.map((e) => caseEntry(e, c.timeline)));
        // New entries come into view if you were reading the end already.
        if (shown && shown < c.timeline.length && atEnd) timeline.lastElementChild?.scrollIntoView({block: 'nearest'});
    };

    const drawCase = () => {
        const seg = (options, current, opName) => h('div', {class: 'segmented small'}, options.map(([k, label]) =>
            h('button', {
                type: 'button',
                'aria-pressed': String(current === k),
                disabled: !open(),
                on: {click: () => current !== k && run({op: opName, [opName]: k})()}
            }, label)));
        const people = h('div', {class: 'assignees'},
            c.assignees.map((n) => h('span', {class: 'assignee'}, h('span', {class: 'avatar'}, initial(n)), h('span', {}, n),
                open() && button(null, {
                    small: true,
                    kind: 'ghost',
                    icon: 'x',
                    'aria-label': `Take ${n} off`,
                    title: isMe(n) ? 'Leave' : `Take ${n} off`,
                    on: {click: run({op: 'unassign', name: n})}
                }))),
            !c.assignees.length && h('span', {class: 'muted'}, 'Nobody yet.'),
            open() && button('Add', {small: true, icon: 'plus', on: {click: assignDialog}}));
        const facts = [
            ['Opened', `${fullTime(c.created_ms)} by ${c.created_by}`],
            ['Last change', ago(c.updated_ms)],
            c.closed_ms && ['Closed', `${fullTime(c.closed_ms)} by ${c.closed_by}`],
            ['Moderation sent', String(c.actions)],
            ['Comments', String(c.comments)],
        ].filter(Boolean);
        replace(caseBox,
            h('div', {class: 'field'}, h('span', {}, 'Status'), c.status === 'closed'
                ? h('div', {}, statusBadge(c))
                : seg([['open', 'Open'], ['investigating', 'Investigating'], ['waiting', 'Waiting']], c.status, 'status')),
            h('div', {class: 'field'}, h('span', {}, 'Priority'), seg(PRIORITIES, c.priority, 'priority')),
            h('div', {class: 'field'}, h('span', {}, 'On it'), people),
            h('dl', {class: 'facts'}, facts.map(([k, v]) => [h('dt', {}, k), h('dd', {}, v)])));
    };

    const drawPlayer = () => {
        const act = (action) => () => moderateDialog(c.player, action, afterAction, open() ? c.id : undefined);
        const links = h('div', {class: 'chips'},
            link(`/player?${query({name: c.player})}`, {class: 'btn small ghost'}, icon('user'), 'Profile'),
            link(`/chat?${query({name: c.player})}`, {class: 'btn small ghost'}, icon('chat'), 'Chat'),
            link(`/replays?${query({name: c.player})}`, {class: 'btn small ghost'}, icon('play'), 'Sessions'),
            link(`/moderation?${query({
                tab: 'triage',
                all: 1,
                name: c.player
            })}`, {class: 'btn small ghost'}, icon('flag'), 'Reports'));
        if (!record) {
            replace(playerBox, h('p', {class: 'muted'}, `No registered player is called ${c.player}. Reports name whoever the reporter typed.`), links);
            return;
        }
        const muted = record.muted_until_ms != null;
        const banned = record.banned_until_ms != null;
        const online = record.online_node != null;
        replace(playerBox,
            h('div', {class: 'list-title'}, h('b', {}, playerLink(record.name)), ' ', playerBadges({staff_level: record.staff_level}),
                online ? h('span', {class: 'badge ok'}, `Online · ${worldOf(record.online_node)}`) : h('span', {class: 'badge'}, 'Offline'),
                muted && h('span', {class: 'badge warn'}, `Muted · ${until(record.muted_until_ms)}`),
                banned && h('span', {class: 'badge danger'}, `Banned · ${until(record.banned_until_ms)}`)),
            h('dl', {class: 'facts'},
                h('dt', {}, 'Registered'), h('dd', {}, dateOnly(record.registered_ms)),
                h('dt', {}, 'Last seen'), h('dd', {}, online ? 'Now' : record.last_seen_ms ? ago(record.last_seen_ms) : 'Never')),
            h('div', {class: 'chips'},
                online && button('Kick', {small: true, icon: 'kick', on: {click: act('kick')}}),
                !(muted && isTrial()) && button(muted ? 'Unmute' : 'Mute', {
                    small: true,
                    icon: muted ? 'undo' : 'mute',
                    on: {click: act(muted ? 'unmute' : 'mute')}
                }),
                !isTrial() && button(banned ? 'Unban' : 'Ban', {
                    small: true,
                    icon: banned ? 'undo' : 'ban',
                    kind: banned ? undefined : 'danger',
                    on: {click: act(banned ? 'unban' : 'ban')}
                })),
            links);
    };

    const loadPlayer = async () => {
        try {
            record = await api('GET', `/lookup/player?${query({name: c.player})}`);
            drawPlayer();
        } catch (e) {
            replace(playerBox, h('p', {class: 'error-text'}, e.message));
        }
    };
    // Every report against them, in this case or not: what isn't may still
    // say something about them.
    let reportsShown = null;
    const loadReports = () => {
        const inCase = new Set(c.evidence.map(evidenceKey));
        reportsShown = `${c.player}|${open()}|${[...inCase].filter((k) => k.startsWith('report:')).sort()}`;
        const item = (r) => {
            const here = inCase.has(`report:${r.id}`);
            const elsewhere = (r.cases || []).filter((x) => x !== id);
            const row = {...r, cases: elsewhere};
            return reportItem(row, null, h('div', {class: 'chips item-actions'},
                here
                    ? h('span', {class: 'badge ok'}, icon('check'), 'In this case')
                    : open() && button('Add to case', {
                    small: true,
                    icon: 'plus',
                    on: {click: run({op: 'add_evidence', item: {type: 'report', id: r.id}}, 'Added to the case.')}
                })));
        };
        replace(reportsBox, pagedList((before) => api('GET', `/lookup/reports?${query({
            name: c.player,
            before
        })}`), item, 'No abuse reports against them.').el);
    };
    const loadOthers = () => api('GET', `/cases?${query({player: c.player, status: 'all'})}`).then((rows) => {
        const rest = rows.filter((x) => x.id !== id);
        replace(others, rest.length ? rest.map((x) => caseItem(x)) : h('div', {class: 'empty'}, 'No other cases on them.'));
    }).catch((e) => failed(others, e));
    const loadHistory = () => api('GET', `/lookup/moderation?${query({name: c.player})}`).then((rows) => {
        replace(pastActions, rows.length ? rows.slice(0, 6).map(moderationItem) : h('div', {class: 'empty'}, 'No moderation on record.'));
    }).catch((e) => failed(pastActions, e));

    // Redraws what changed; `force` after your own change.
    const show = (detail, force) => {
        const first = !c;
        const changed = force || first || detail.updated_ms !== c.updated_ms || detail.timeline.length !== c.timeline.length;
        const playerChanged = first || detail.player !== c.player;
        c = detail;
        drawPresence();
        if (!changed) return;
        // Looking counts as reading it: the nav's dot may go.
        if (state.caseSummary?.unread) loadCaseSummary();
        document.title = `#${c.id} ${c.title} · RS Admin`;
        layout.classList.remove('hidden');
        drawHead();
        drawAppeal();
        drawEvidence();
        drawCase();
        drawTimeline();
        if (playerChanged) {
            loadPlayer();
            loadOthers();
            loadHistory();
        }
        const reportKeys = c.evidence.map(evidenceKey).filter((k) => k.startsWith('report:')).sort();
        if (reportsShown !== `${c.player}|${open()}|${reportKeys}`) loadReports();
    };
    const refresh = async () => {
        try {
            show(await api('GET', `/cases/${id}`));
        } catch (e) {
            if (!c) replace(head, pageHead(`Case #${id}`, e.message));
        }
    };

    const sendComment = async () => {
        const text = composer.value.trim();
        if (composerMedia.busy()) return toast('Wait for the upload to finish.', 'error');
        if (!text) return setPreview(false);
        send.disabled = true;
        try {
            await op({op: 'comment', text});
            composer.value = '';
            prefs.set(draftKey, '');
            if (previewing) setPreview(false);
            timeline.lastElementChild?.scrollIntoView({block: 'nearest'});
        } catch (e) {
            toast(e.message, 'error');
        } finally {
            send.disabled = false;
        }
    };
    send.addEventListener('click', sendComment);
    composer.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            sendComment();
        }
    });
    // An unsent comment survives leaving the page, in this browser.
    composer.addEventListener('input', debounce(() => prefs.set(draftKey, composer.value), 300));
    addEvidence.addEventListener('click', () => evidenceDialog(c, (detail) => show(detail, true)));

    refresh();
    const timer = setInterval(() => {
        if (!document.hidden) refresh();
    }, 5000);
    ctx.cleanup(() => clearInterval(timer));
    // Evidence names worlds by node until the world list first arrives.
    let named = state.worldsLoaded;
    ctx.cleanup(onWorlds(() => {
        if (named || !c) return;
        named = true;
        drawEvidence();
    }));

    append(layout,
        h('div', {class: 'column'},
            appealBox,
            card('Evidence', [evidenceCount, addEvidence], evidenceList),
            card('Reports against them', h('span', {class: 'muted'}, 'All of them, in this case or not'), reportsBox),
            h('section', {class: 'card'},
                h('div', {class: 'card-head'}, h('h2', {}, 'Timeline')),
                timeline,
                h('div', {class: 'composer'}, composer, previewBox,
                    h('div', {class: 'composer-foot'},
                        h('div', {class: 'news-tools'}, composerMedia.button, composerMedia.status, composerMedia.input),
                        h('div', {class: 'composer-send'}, h('span', {class: 'hint'}, 'Ctrl+Enter to send. All staff can read it.'), previewButton, send))))),
        h('div', {class: 'column'},
            card('Player', null, playerBox),
            card('Case', null, caseBox),
            card('Other cases on them', null, others),
            card('Moderation history', null, pastActions)));
    append(page, presence, layout);
    return page;
}

// ---------------------------------------------------------------------------
// Chat logs

// New chat comes in this often while the feed is live.
const CHAT_LIVE_MS = 2000;
// The most messages a live feed keeps on the page.
const CHAT_LIVE_KEEP = 500;

function chatPage(ctx) {
    let node = ctx.params.get('node') ? Number(ctx.params.get('node')) : null;
    let live = ctx.params.get('live') === '1';
    // The newest message shown, which the live feed carries on from.
    let newest = null;
    let feed = null;
    let liveTimer = null;
    const liveBtn = button('Live', {
        icon: 'radio',
        'aria-pressed': String(live),
        title: 'Show new messages as they come in, newest at the top'
    });
    const nameInput = h('input', {
        class: 'input',
        type: 'search',
        placeholder: 'Player name (optional)',
        'aria-label': 'Player',
        value: ctx.params.get('name') || '',
        maxlength: '12'
    });
    nameSuggest(nameInput);
    const worlds = h('div', {class: 'chips'});
    const body = h('div', {class: 'chat-list'});
    const note = h('p', {class: 'muted'});

    const load = () => {
        const name = nameInput.value.trim();
        if (name && !validName(name)) {
            replace(body, h('div', {class: 'empty'}, 'That isn\'t a valid player name.'));
            return;
        }
        const q = query({name, node, live: live ? 1 : null});
        history.replaceState(null, '', `/chat${q ? `?${q}` : ''}`);
        saved.draw();
        note.textContent = `${name ? `${name}'s` : 'Everyone\'s'} public chat and private messages${node ? ` on world ${state.worlds.find((w) => w.node === node)?.id ?? node}` : ', on every world'}, newest first${live ? ', as they come in' : ''}. Viewing them is recorded in the audit log.`;
        newest = null;
        const paged = pagedList((before) => api('GET', `/lookup/chat?${query({name, node, before})}`).then((rows) => {
            if (!before) newest = rows.length ? rows[0].id : 0;
            return rows;
        }), chatItem, 'No chat matches.');
        feed = {list: paged.list, name, node};
        replace(body, paged.el);
    };
    // The live feed: what's come since the newest shown, at the top.
    const poll = async () => {
        clearTimeout(liveTimer);
        if (!live) return;
        const current = feed;
        if (current && newest !== null && !document.hidden) {
            try {
                const rows = await api('GET', `/lookup/chat?${query({
                    name: current.name,
                    node: current.node,
                    after: newest
                })}`);
                if (current === feed && rows.length) {
                    newest = rows[rows.length - 1].id;
                    current.list.querySelector(':scope > .empty')?.remove();
                    for (const row of rows) {
                        const el = chatItem(row);
                        el.classList.add('fresh');
                        current.list.prepend(el);
                    }
                    while (current.list.children.length > CHAT_LIVE_KEEP) current.list.lastElementChild.remove();
                }
            } catch { /* the next poll tries again */
            }
        }
        if (live) liveTimer = setTimeout(poll, CHAT_LIVE_MS);
    };
    liveBtn.addEventListener('click', () => {
        live = !live;
        liveBtn.setAttribute('aria-pressed', String(live));
        load();
        poll();
    });
    ctx.cleanup(() => {
        live = false;
        clearTimeout(liveTimer);
    });
    const drawWorlds = () => replace(worlds,
        h('button', {
            type: 'button', class: 'chip', 'aria-pressed': String(node === null), on: {
                click: () => {
                    node = null;
                    drawWorlds();
                    load();
                }
            }
        }, 'All worlds'),
        state.worlds.map((w) => h('button', {
            type: 'button',
            class: 'chip',
            'aria-pressed': String(node === w.node),
            on: {
                click: () => {
                    node = w.node;
                    drawWorlds();
                    load();
                }
            }
        }, worldName(w))));
    const filter = () => ({name: nameInput.value.trim(), node: node === null ? '' : String(node)});
    const saved = savedFilters('chat', filter, (params) => {
        nameInput.value = params.name || '';
        node = params.node ? Number(params.node) : null;
        drawWorlds();
        load();
    });
    nameInput.addEventListener('input', debounce(load, 400));
    drawWorlds();
    ctx.cleanup(onWorlds(() => {
        if (!worlds.children.length || worlds.children.length !== state.worlds.length + 1) drawWorlds();
    }));
    load();
    poll();

    return h('div', {class: 'page'},
        pageHead('Chat logs', null, liveBtn, button('Refresh', {icon: 'refresh', on: {click: load}}),
            exportButton(() => `/export/chat?${query(filter())}`, 'Download the messages this shows (up to 5,000) as a CSV file. Recorded in the audit log.')),
        h('div', {class: 'toolbar'}, h('div', {class: 'search'}, icon('search'), nameInput), worlds),
        saved.el,
        note,
        h('section', {class: 'card'}, body));
}

// ---------------------------------------------------------------------------
// Session replays: every session is recorded, and the anti-cheat checks
// score it. A replay plays in the world's own client (its /replay page),
// framed here and fed the recording by postMessage.

const FLAGS = {
    modified_client: ['Modified client', 'Sent anti-cheat packets with the wrong contents. The real client never does.'],
    no_heartbeat: ['No game loop', 'Acted without sending any of the anti-cheat packets the client\'s game loop sends. Usually a packet bot.'],
    missing_op_checks: ['Missing op checks', 'Left out the anti-cheat packets the real client sends with some actions once it has done them enough.'],
    actions_without_clicks: ['Actions without clicks', 'Many actions came without the click the real client reports with each one.'],
    no_mouse_movement: ['No mouse', 'Played without the client ever reporting the mouse.'],
    cursor_jumps: ['Cursor jumps', 'Most clicks landed far from where the cursor last was: it jumps rather than moves. A touch screen does this too (a tap puts the cursor where it lands), so check the replay: an iPad or a touchscreen laptop passes as a desktop.'],
    robotic_click_timing: ['Clockwork clicks', 'Clicks came at very even intervals.'],
    automated_browser: ['Automated browser', 'The web client ran in a browser that says it\'s headless (Puppeteer, Selenium, Playwright), or sent no user agent at all.'],
};

const flagBadge = (flag) => h('span', {
    class: 'badge danger',
    title: FLAGS[flag]?.[1] || flag
}, FLAGS[flag]?.[0] || flag);

function kilobytes(n) {
    if (n < 1024 * 1024) return `${Math.max(1, Math.round(n / 1024))} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
    return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

const replayHref = (r) => `/replay?${query({id: r.id})}`;

function replayItem(r, extra) {
    const flagged = r.score > 0;
    return h('div', {class: 'list-item'},
        h('div', {class: `list-icon ${flagged ? 'danger' : r.live ? 'ok' : ''}`}, icon(flagged ? 'alert' : 'play')),
        h('div', {class: 'list-main'},
            h('div', {class: 'list-title'},
                link(replayHref(r), {class: 'name-link'}, r.player), ' ',
                r.live && link(liveHref(r.id), {class: 'badge ok', title: 'Watch it live'}, icon('radio'), 'Live'), ' ',
                !r.complete && h('span', {
                    class: 'badge warn',
                    title: 'Part of it was lost: it stops early.'
                }, 'Incomplete'), ' ',
                r.flags.map(flagBadge), caseLinks(r.cases), dismissedBadge(r.dismissed)),
            h('div', {class: 'list-sub'},
                `${r.world ? `World ${r.world}` : `Node ${r.node}`} · ${clock(r.duration_ms / 1000)} · ${kilobytes(r.bytes)}${flagged ? ` · score ${r.score}` : ''}`),
            extra),
        h('div', {class: 'list-side', title: fullTime(r.started_ms)}, ago(r.started_ms)));
}

function replaysPage(ctx) {
    let flagged = ctx.params.get('flagged') === '1';
    const nameInput = h('input', {
        class: 'input',
        type: 'search',
        placeholder: 'Player name (optional)',
        'aria-label': 'Player',
        value: ctx.params.get('name') || '',
        maxlength: '12'
    });
    nameSuggest(nameInput);
    const filter = h('div', {class: 'chips'});
    const body = h('div', {});
    const stats = h('div', {class: 'stats'});
    const stat = (iconName, label, value, sub) => h('div', {class: 'card stat'},
        h('div', {class: 'label'}, icon(iconName), label), h('div', {class: 'value num'}, value), h('div', {class: 'sub'}, sub));

    // How much they take up: left out when no world can say.
    const loadStorage = () => api('GET', '/replays/storage').then((s) => replace(stats,
        stat('download', 'Storage', kilobytes(s.bytes), `${kilobytes(s.disk_bytes)} on disk, indexes and overhead included`),
        stat('play', 'Recordings', s.recordings.toLocaleString(), s.oldest_ms ? `The oldest started ${ago(s.oldest_ms)}` : 'None kept'),
        stat('clock', 'Last 24 hours', kilobytes(s.day_bytes), plural(s.day_recordings, 'session')),
        stat('activity', 'Per hour played', s.play_ms ? kilobytes(s.bytes / s.play_ms * 3_600_000) : '-',
            `Over ${plural(Math.round(s.play_ms / 3_600_000), 'hour')} recorded`)))
        .catch(() => replace(stats));

    const load = () => {
        const name = nameInput.value.trim();
        if (name && !validName(name)) {
            replace(body, h('div', {class: 'empty'}, 'That isn\'t a valid player name.'));
            return;
        }
        const q = query({name, flagged: flagged ? 1 : null});
        history.replaceState(null, '', `/replays${q ? `?${q}` : ''}`);
        const paged = pagedList(
            // The client for these starts loading now (see dockWarm).
            (before) => api('GET', `/replays?${query({name, flagged: flagged || null, before})}`).then((rows) => {
                if (!before) dockWarm(rows);
                return rows;
            }),
            replayItem,
            flagged ? 'No flagged sessions.' : 'No recorded sessions.',
            (r) => r.started_ms);
        replace(body, paged.el);
    };
    const drawFilter = () => replace(filter,
        h('button', {
            type: 'button', class: 'chip', 'aria-pressed': String(!flagged), on: {
                click: () => {
                    flagged = false;
                    drawFilter();
                    load();
                }
            }
        }, 'All sessions'),
        h('button', {
            type: 'button', class: 'chip', 'aria-pressed': String(flagged), on: {
                click: () => {
                    flagged = true;
                    drawFilter();
                    load();
                }
            }
        }, icon('alert'), 'Flagged by the checks'));
    nameInput.addEventListener('input', debounce(load, 400));
    drawFilter();
    loadStorage();
    load();

    return h('div', {class: 'page'},
        pageHead('Replays', 'Every session is recorded and kept for a while. Watch one to see the game as the player did, with their mouse.',
            button('Refresh', {
                icon: 'refresh', on: {
                    click: () => {
                        loadStorage();
                        load();
                    }
                }
            })),
        stats,
        h('div', {class: 'toolbar'}, h('div', {class: 'search'}, icon('search'), nameInput), filter),
        h('section', {class: 'card'}, body));
}

// What a recorded session's client said it ran on, as the player page's
// devices name it: "Chrome on Windows (en-GB)".
function replayDevice(s) {
    const tapNote = s.touch ? ', a phone or tablet (taps may jump the cursor)' : '';
    if (s.web === undefined) return s.touch === undefined ? null : `Not recorded${tapNote}`;
    if (!s.web) return 'Desktop client';
    const what = s.userAgent ? device(s.userAgent) : 'Web client that sent no user agent';
    const platform = s.platform && !what.includes(s.platform) ? `, says ${s.platform}` : '';
    const language = s.language ? ` (${s.language})` : '';
    return h('span', {title: s.userAgent || ''}, `${what}${platform}${language}${tapNote}`);
}

// The checks' measurements, in words.
function replayStats(r) {
    let s = {};
    try {
        s = JSON.parse(r.stats || '{}');
    } catch { /* none */
    }
    const sum = (a) => (a || []).reduce((x, y) => x + y, 0);
    const rows = [
        ['Device', replayDevice(s)],
        ['Automated browser', s.automated ? 'Yes' : null],
        ['Actions', s.ops],
        ['Actions without a click', s.opsUnclicked],
        ['Clicks', s.clicks],
        ['Clicks far from the cursor', s.cursorJumps],
        ['Click rhythm', s.clickRhythm == null ? null : `${s.clickRhythm} ${s.clickRhythm < 0.12 ? '(even)' : '(varied)'}`],
        ['Mouse reports', s.inputReports],
        ['Window lost focus', s.focusLost],
        ['Game loop heartbeats', s.heartbeats ? sum(s.heartbeats) : null],
        ['Packets with wrong contents', s.badHeartbeats ? sum(s.badHeartbeats) + sum(s.badOpChecks) : null],
        ['Op checks sent / expected', s.opChecksExpected ? `${sum(s.opChecksReceived)} / ${sum(s.opChecksExpected)}` : null],
    ].filter(([, v]) => v !== undefined && v !== null);
    return h('dl', {class: 'replay-stats'}, rows.map(([k, v]) => [h('dt', {}, k), h('dd', {class: 'num'}, v instanceof Node ? v : String(v))]));
}

const SPEEDS = [0.5, 1, 2, 4, 8];

// Finished recordings already downloaded this visit, by id: reopening one
// starts at once. Up to REPLAY_CACHE_BYTES, the oldest dropped first.
const replayCache = new Map();
const REPLAY_CACHE_BYTES = 256 * 1024 * 1024;

function cacheReplay(id, file) {
    replayCache.delete(id);
    replayCache.set(id, file);
    let total = 0;
    for (const f of replayCache.values()) total += f.byteLength;
    for (const [key, f] of replayCache) {
        if (total <= REPLAY_CACHE_BYTES) break;
        replayCache.delete(key);
        total -= f.byteLength;
    }
}

// Finished recordings are also kept in this browser between visits (Cache
// Storage, which only this origin reads), so going back to one doesn't
// download it again. They hold private messages, so they're wiped when
// staff sign out or their session ends, expire after REPLAY_STORE_DAYS, and
// the least recently watched go first past REPLAY_STORE_BYTES. Watching a
// kept copy is still audited. Cache Storage needs a secure origin (https, or
// localhost); without one only the in-memory copies above are kept.
const REPLAY_STORE = 'rs-admin-replays-v1';
const REPLAY_STORE_BYTES = 1024 * 1024 * 1024;
const REPLAY_STORE_DAYS = 14;
const replayStoreKey = (id) => `/replay-store/${id}`;

// What's kept, by id: { bytes, usedMs }. In localStorage, so trimming
// doesn't have to open every copy.
const replayIndex = {
    read() {
        try {
            return JSON.parse(prefs.get('replayStore', '{}')) || {};
        } catch {
            return {};
        }
    },
    write(index) {
        prefs.set('replayStore', JSON.stringify(index));
    },
};

async function openReplayStore() {
    try {
        return self.caches ? await caches.open(REPLAY_STORE) : null;
    } catch {
        return null;
    }
}

// A kept copy of recording `id`, or null.
async function storedReplay(id) {
    const store = await openReplayStore();
    if (!store) return null;
    try {
        const res = await store.match(replayStoreKey(id));
        if (!res) return null;
        const file = await res.arrayBuffer();
        const index = replayIndex.read();
        index[id] = {bytes: file.byteLength, usedMs: Date.now()};
        replayIndex.write(index);
        return file;
    } catch {
        return null;
    }
}

async function storeReplay(id, file) {
    const store = await openReplayStore();
    if (!store || file.byteLength > REPLAY_STORE_BYTES) return;
    try {
        await store.put(replayStoreKey(id), new Response(file, {headers: {'Content-Type': 'application/octet-stream'}}));
        const index = replayIndex.read();
        index[id] = {bytes: file.byteLength, usedMs: Date.now()};
        replayIndex.write(index);
        await trimReplayStore();
    } catch { /* out of space, most likely: it just isn't kept */
    }
}

// Drops expired copies, then the least recently watched past the limit, and
// anything the index and the store disagree on.
async function trimReplayStore() {
    const store = await openReplayStore();
    if (!store) return;
    const index = replayIndex.read();
    const oldest = Date.now() - REPLAY_STORE_DAYS * 86400 * 1000;
    const kept = new Set((await store.keys()).map((req) => new URL(req.url).pathname.split('/').pop()));
    let total = 0;
    const live = Object.entries(index).filter(([id, e]) => kept.has(id) && e.usedMs >= oldest)
        .sort(([, a], [, b]) => b.usedMs - a.usedMs)
        .filter(([, e]) => (total += e.bytes) <= REPLAY_STORE_BYTES);
    const keep = Object.fromEntries(live);
    for (const id of kept) if (!keep[id]) await store.delete(replayStoreKey(id));
    replayIndex.write(keep);
}

// Wipes every kept copy, and the ones in memory.
async function clearReplayStore() {
    replayCache.clear();
    replayIndex.write({});
    try {
        if (self.caches) await caches.delete(REPLAY_STORE);
    } catch { /* nothing kept */
    }
}

// ---------------------------------------------------------------------------
// The replay dock: a loaded game client (and its scout) kept between replays.
// Loading the client takes seconds; it's started ahead of time on the pages
// that lead to a replay, and a replay only logs it in to its recording.
// Moving a frame in the page reloads it, so the dock sits apart, fixed on
// top of the page and laid over a replay page's stage. Away from one it
// waits out of sight (but in view: Chrome slows a frame out of view to a
// timer a second), and is let go after a while.

const DOCK_KEEP_MS = 5 * 60 * 1000;
const dock = {
    url: null, el: null, main: null, scout: null,
    ready: {main: false, scout: false}, size: null,
    page: null, box: null, raf: 0, letGo: null, warmedFor: null,
};

window.addEventListener('message', (e) => {
    if (!dock.el || e.data?.source !== 'rs-replay' || e.origin !== new URL(dock.url).origin) return;
    const which = e.source === dock.main?.contentWindow ? 'main' : e.source === dock.scout?.contentWindow ? 'scout' : null;
    if (!which) return;
    if (e.data.type === 'ready') {
        dock.ready[which] = true;
        if (which === 'main') dock.size = [e.data.width, e.data.height];
    }
    dock.page?.onMessage(which, e.data);
});

const dockPost = (which, msg, transfer) => dock[which]?.contentWindow?.postMessage(msg, new URL(dock.url).origin, transfer);

function dockFrame(url, scout) {
    const src = new URL(url);
    if (scout) src.searchParams.set('scout', '1');
    return h('iframe', scout
        ? {
            class: 'replay-scout',
            src: src.href,
            tabindex: '-1',
            'aria-hidden': 'true',
            title: 'Replay previews',
            referrerpolicy: 'no-referrer'
        }
        : {class: 'replay-frame', src: src.href, title: 'Replay', referrerpolicy: 'no-referrer'});
}

// A client loaded for `url` (a world's replay page), and its scout if asked.
function dockEnsure(url, withScout) {
    if (dock.url !== url) dockLetGo();
    if (!dock.el) {
        dock.url = url;
        dock.el = h('div', {class: 'replay-dock parked'});
        dock.main = dockFrame(url, false);
        dock.el.append(dock.main);
        dock.ready = {main: false, scout: false};
        document.body.append(dock.el);
    }
    if (withScout && !dock.scout) {
        dock.scout = dockFrame(url, true);
        // Under the replay, covered by it.
        dock.el.prepend(dock.scout);
    }
    clearTimeout(dock.letGo);
    dock.letGo = null;
}

function dockLetGo() {
    cancelAnimationFrame(dock.raf);
    clearTimeout(dock.letGo);
    dock.el?.remove();
    Object.assign(dock, {
        url: null,
        el: null,
        main: null,
        scout: null,
        ready: {main: false, scout: false},
        size: null,
        page: null,
        box: null,
        raf: 0,
        letGo: null
    });
}

// Laid over `page.stage` until detached, kept there as the page scrolls or
// its layout changes. `dock.box` is where in the stage the game goes.
function dockAttach(page) {
    dock.page = page;
    dock.el.classList.remove('parked');
    const place = () => {
        cancelAnimationFrame(dock.raf);
        dock.raf = requestAnimationFrame(place);
        const s = dock.el.style;
        if (!dock.box) {
            s.visibility = 'hidden';
            return;
        }
        const r = page.stage.getBoundingClientRect();
        s.visibility = '';
        s.left = `${r.left + dock.box.left}px`;
        s.top = `${r.top}px`;
        s.width = `${dock.box.width}px`;
        s.height = `${dock.box.height}px`;
    };
    dock.place = place;
    place();
}

// With the page as it scrolls, not a frame behind.
window.addEventListener('scroll', () => dock.page && dock.place?.(), {passive: true, capture: true});

// Done with it: the replay logs out, and the client waits for the next.
function dockDetach(page) {
    if (!dock.el || dock.page !== page) return;
    cancelAnimationFrame(dock.raf);
    dock.page = null;
    dock.box = null;
    dockPost('main', {type: 'unload'});
    dockPost('scout', {type: 'unload'});
    dock.el.classList.add('parked');
    dock.el.removeAttribute('style');
    dock.letGo = setTimeout(dockLetGo, DOCK_KEEP_MS);
}

// Ahead of a replay: the client for the first of a list page's `replays`
// starts loading, both copies, while nothing's being watched.
async function dockWarm(replays) {
    const first = replays?.[0];
    if (!first || dock.page || dock.warmedFor === first.id) return;
    dock.warmedFor = first.id;
    try {
        const detail = await api('GET', `/replays/${first.id}`);
        if (detail.player_url && !dock.page) {
            dockEnsure(detail.player_url, true);
            dock.letGo = setTimeout(dockLetGo, DOCK_KEEP_MS);
        }
    } catch { /* only a head start */
    }
}

// A replay file's JSON header.
function fileHeader(file) {
    const view = new DataView(file);
    return JSON.parse(new TextDecoder().decode(new Uint8Array(file, 9, view.getUint32(5))));
}

// A live session is asked for what's new this often.
const LIVE_POLL_MS = 2000;

function replayPage(ctx) {
    const id = ctx.params.get('id') || '';
    const page = h('div', {class: 'page replay-page'});
    const head = h('div', {}, pageHead('Replay', 'Loading...'));
    append(page, head);
    if (!/^\d{1,20}$/.test(id)) {
        replace(head, pageHead('Replay', 'No such recording.'));
        return page;
    }

    let frame = null;
    let origin = null;
    let file = null;
    let detail = null;
    // Watching live (`live=1`, for a session still going on): the world's
    // token for what comes next, how far behind its end to play, and what has
    // come since the first file (sent again if the page has to load again).
    let liveMode = ctx.params.get('live') === '1';
    let liveToken = null;
    let liveDelay = 0;
    let liveTimer = null;
    let liveEnded = false;
    const appended = [];
    let notes = [];
    let total = 1;
    // Whether the first catch-up after loading has been done (see onMain).
    let caughtUp = false;
    let wantSpeed = 1;
    let wantPaused = false;
    let freeLook = false;
    let dragging = false;
    // Where it is, and where it's going while it seeks.
    let atMs = 0;
    let seekingTo = null;
    // When the last seek was sent, and whether the page has said it's on it
    // (states sent before it got the seek still say where it was).
    let seekSentAt = 0;
    let seekHeard = false;
    // While the client starts it can't seek (the page ignores it): the
    // controls that would wait.
    let starting = true;
    // The game's own size: 789x532 on 225, 765x503 after.
    let gameW = 765;
    let gameH = 503;
    let sizing = prefs.get('replaySize', 'sharp');
    // A clip: the stretch marked (or opened by a clip's link, `from` to `to`
    // ms in), and whether playing pauses at its end.
    const msParam = (k) => (/^\d{1,10}$/.test(ctx.params.get(k) || '') ? Number(ctx.params.get(k)) : null);
    let clipFrom = msParam('from');
    let clipTo = msParam('to');
    if (clipFrom === null || (clipTo !== null && clipTo < clipFrom + 1000)) clipTo = null;
    const fromLink = clipFrom !== null && clipTo !== null;
    let clipStop = fromLink;
    const clipLabel = h('input', {
        class: 'input',
        maxlength: '200',
        placeholder: 'What happens in it (optional)',
        'aria-label': 'Clip label',
        value: ctx.params.get('label') || ''
    });
    const clipMark = h('span', {});
    const clipRange = h('div', {class: 'replay-clip hidden', 'aria-hidden': 'true'}, clipMark);
    const clipBar = h('div', {class: 'replay-clipbar'});
    const clipsList = h('div', {class: 'list replay-notes'});

    const stage = h('div', {class: 'replay-stage'}, h('div', {class: 'empty'}, h('div', {class: 'spinner'})));
    const status = h('span', {class: 'muted replay-status'}, 'Loading the recording...');
    const time = h('span', {class: 'num replay-time'}, '0:00 / 0:00');
    const scrub = h('input', {
        type: 'range',
        class: 'replay-scrub',
        min: '0',
        max: '1',
        step: '100',
        value: '0',
        'aria-label': 'Position'
    });
    const marks = h('div', {class: 'replay-marks', 'aria-hidden': 'true'});
    // What's buffered: has frames to preview or jump to straight away.
    const bufferBar = h('div', {class: 'replay-buffer', 'aria-hidden': 'true'});
    const playBtn = button(null, {icon: 'pause', 'aria-label': 'Pause', title: 'Play / pause (space)', disabled: true});
    const backBtn = button('10', {
        icon: 'back',
        small: true,
        'aria-label': 'Back 10 seconds',
        title: 'Back 10 seconds'
    });
    const forwardBtn = button('10', {
        icon: 'forward',
        small: true,
        'aria-label': 'Forward 10 seconds',
        title: 'Forward 10 seconds'
    });
    const edgeBtn = button('Now', {icon: 'radio', small: true, title: 'Back to as live as it gets'});
    edgeBtn.classList.add('hidden');
    const lookBtn = button('Free look', {
        icon: 'eye',
        small: true,
        'aria-pressed': 'false',
        title: 'Pause and use the client yourself: your mouse and arrow keys work its menus, tabs and camera. Nothing reaches the game, and turning it off puts the client back as it was.'
    });
    const footHint = h('span', {class: 'muted'}, 'Watching is recorded in the audit log.');
    const speeds = h('div', {class: 'segmented replay-speeds', role: 'group', 'aria-label': 'Speed'});
    const timeline = h('div', {class: 'list replay-notes'});
    const checks = h('div', {});
    const seekText = h('span', {});
    const seekBar = h('span', {class: 'replay-seek-bar'});
    const cover = h('div', {class: 'replay-cover hidden'}, h('div', {}, h('div', {class: 'spinner'}), seekText, h('span', {class: 'replay-seek-track'}, seekBar)));
    const sizes = h('div', {class: 'segmented replay-sizes', role: 'group', 'aria-label': 'Size'});

    const send = (msg, transfer) => {
        if (dock.page === pageCtl) dockPost('main', msg, transfer);
    };

    // While it seeks, the game is covered: it plays the way there, fast.
    // Over a frame already watched (`peek`), only a corner badge: the frame
    // shows while the client catches up behind it.
    const showSeek = (to, at, peek) => {
        cover.classList.toggle('hidden', to === null);
        cover.classList.toggle('peek', !!peek);
        if (to !== null) {
            seekText.textContent = `Seeking to ${clock(to / 1000)}`;
            seekBar.style.width = `${Math.min(100, Math.round(((at || 0) / Math.max(1, to)) * 100))}%`;
        }
    };

    const drawSpeeds = () => replace(speeds, SPEEDS.map((s) => h('button', {
        type: 'button', 'aria-pressed': String(wantSpeed === s),
        on: {
            click: () => {
                wantSpeed = s;
                send({type: 'speed', value: s});
                drawSpeeds();
            }
        },
    }, s === 0.5 ? '½×' : `${s}×`)));

    const drawPlay = () => {
        replace(playBtn, icon(wantPaused ? 'play' : 'pause'));
        playBtn.setAttribute('aria-label', wantPaused ? 'Play' : 'Pause');
    };

    const togglePlay = () => {
        wantPaused = !wantPaused;
        // Played again at the end: the page starts over.
        if (!wantPaused && status.dataset.ended === '1') {
            status.dataset.ended = '';
            status.textContent = '';
        }
        send({type: wantPaused ? 'pause' : 'play'});
        drawPlay();
        // Playing ends free look (the page puts the client back).
        if (!wantPaused && freeLook) {
            freeLook = false;
            drawLook();
        }
    };

    const seek = (ms) => {
        if (starting) return;
        clipStop = false;
        seekingTo = ms;
        seekSentAt = performance.now();
        seekHeard = false;
        // The slider goes straight there, not through where the replay passes
        // on its way (a seek back starts it from earlier).
        if (!dragging) scrub.value = String(Math.round(ms));
        time.textContent = `${clock(ms / 1000)} / ${clock(Number(scrub.max) / 1000)}`;
        // The page answers at once with how it's seeking (over a watched frame
        // or not), which sets the cover.
        send({type: 'seek', ms});
        // Seeking ends free look too.
        if (freeLook) {
            freeLook = false;
            drawLook();
        }
        status.dataset.ended = '';
        status.textContent = '';
    };

    // Sizes the game to the stage. Sharp: the largest whole number of screen
    // pixels per game pixel that fits, crisp as in game. Fit: the whole width,
    // smoothed.
    const fit = () => {
        if (!frame) return;
        const room = stage.clientWidth;
        const ratio = window.devicePixelRatio || 1;
        const whole = Math.floor((room * ratio) / gameW);
        const width = sizing === 'sharp' && whole >= 1 ? (gameW * whole) / ratio : room;
        const height = (width * gameH) / gameW;
        if (dock.page === pageCtl) dock.box = {left: (room - width) / 2, width, height};
        stage.style.height = `${height}px`;
    };
    const drawSizes = () => replace(sizes, [['sharp', 'Sharp'], ['fit', 'Fit']].map(([key, label]) => h('button', {
        type: 'button',
        'aria-pressed': String(sizing === key),
        title: key === 'sharp' ? 'Whole screen pixels per game pixel' : 'Fill the width, smoothed',
        on: {
            click: () => {
                sizing = key;
                prefs.set('replaySize', key);
                drawSizes();
                fit();
            }
        },
    }, label)));
    drawSizes();
    const onDpr = () => fit();
    window.addEventListener('resize', onDpr);
    ctx.cleanup(() => window.removeEventListener('resize', onDpr));
    const resize = new ResizeObserver(fit);
    resize.observe(stage);
    ctx.cleanup(() => resize.disconnect());

    // The dock's client, laid over the stage. `seekTo`: where to start.
    const pageCtl = {stage, onMessage: (which, msg) => (which === 'main' ? onMain(msg) : onScout(msg))};
    const mount = (seekTo) => {
        setStarting(true);
        dockEnsure(detail.player_url, false);
        frame = dock.main;
        scout = dock.scout;
        loadSeek = seekTo || 0;
        mainLoaded = false;
        scoutLoaded = false;
        dockAttach(pageCtl);
        dock.el.append(cover);
        showSeek(seekTo ? seekTo : null, 0);
        // Already loaded (it was waiting): as when it says it's ready.
        if (dock.ready.main) onMain({type: 'ready', width: dock.size?.[0], height: dock.size?.[1]});
        if (scout && dock.ready.scout) onScout({type: 'ready'});
        fit();
    };
    ctx.cleanup(() => {
        dockDetach(pageCtl);
        cover.remove();
    });

    const setStarting = (on) => {
        starting = on;
        for (const el of [scrub, backBtn, forwardBtn, edgeBtn]) el.disabled = on;
    };

    // From where it's headed, if it's on the way somewhere.
    const skip = (by) => {
        const total = Number(scrub.max) || 0;
        seek(Math.max(0, Math.min(total, (seekingTo ?? atMs) + by)));
    };

    const drawLook = () => {
        lookBtn.setAttribute('aria-pressed', String(freeLook));
        footHint.textContent = freeLook
            ? "Free look: your mouse and arrow keys work the client; the player's cursor is still drawn. Nothing reaches the game."
            : 'Watching is recorded in the audit log.';
    };

    // The recording goes to the page once both are there: the page (and its
    // client, which takes seconds to load) starts while the file downloads.
    let frameReady = false;
    let mainLoaded = false;
    let loadSeek = 0;
    const sendLoad = () => {
        if (!frame || !frameReady || !file || mainLoaded) return;
        mainLoaded = true;
        const copy = file.slice(0);
        send({type: 'load', file: copy, seek: loadSeek, speed: wantSpeed, paused: wantPaused, freeLook}, [copy]);
        for (const more of appended) {
            const again = more.slice(0);
            send({type: 'append', file: again}, [again]);
        }
        status.textContent = 'Starting the client...';
    };

    const onMain = (msg) => {
        if (msg.type === 'ready') {
            gameW = msg.width || gameW;
            gameH = msg.height || gameH;
            fit();
            frameReady = true;
            sendLoad();
        } else if (msg.type === 'state') {
            // The scout starts as soon as the replay plays (not with it: the two
            // share a thread, and loading both at once slows both), so the first
            // preview of somewhere unwatched doesn't wait for a client to load.
            if (msg.live && !scout && !liveMode) requestShot(1000);
            // Hovering somewhere still waiting for a frame: ask again, in case
            // the first ask came before the page (or the scout) could answer.
            const waiting = hoverMs !== null && !(previewFrame !== null && Math.abs(previewFrame - hoverMs) <= previewWithin());
            if (waiting && performance.now() - previewAsked > 400) {
                previewAsked = performance.now();
                send({type: 'preview', ms: hoverMs});
            }
            const total = msg.durationMs || detail.replay.duration_ms;
            atMs = msg.ms;
            if (msg.seeking) seekHeard = true;
            // Watching live, loading the client took a while, which it's now that
            // far further behind: catch up, once.
            if (liveMode && !caughtUp && msg.live && !msg.seeking && !msg.loading && seekingTo === null) {
                caughtUp = true;
                if (total - msg.ms > liveDelay + 4000) seek(Math.max(0, total - liveDelay));
            } else if (seekingTo !== null && (seekHeard || performance.now() - seekSentAt > 1000)) seekingTo = null;
            // Seeking: shown at where it's going.
            const shownMs = seekingTo ?? (msg.seeking ? msg.seekTo : msg.ms);
            if (Array.isArray(msg.buffered) && total > 0) {
                replace(bufferBar, msg.buffered.map(([from, to]) => h('span', {
                    style: {
                        left: `${Math.min(100, (from / total) * 100)}%`,
                        width: `${Math.max(0.2, Math.min(100, ((Math.min(to, total) - from) / total) * 100))}%`,
                    }
                })));
            }
            scrub.max = String(total);
            if (!dragging) scrub.value = String(Math.round(shownMs));
            time.textContent = `${clock(shownMs / 1000)} / ${clock(total / 1000)}`;
            playBtn.disabled = msg.loading;
            // (A world from before `starting` only says it's loading.)
            const nowStarting = msg.starting ?? msg.loading;
            if (nowStarting !== starting) setStarting(nowStarting);
            if (msg.seeking) {
                showSeek(msg.seekTo, msg.ms, msg.preview);
            } else if (!msg.loading || !loadSeek) {
                showSeek(null);
            }
            if (status.dataset.ended !== '1') {
                const behind = liveMode && !liveEnded && msg.live
                    ? (msg.starved ? 'Live: waiting for more from the world...' : `Live, ${clock(Math.max(0, total - msg.ms) / 1000)} behind`)
                    : '';
                status.textContent = msg.seeking ? '' : msg.loading ? 'Starting the client...' : msg.live ? behind : 'Reconnecting...';
            }
            for (const el of timeline.querySelectorAll('[data-ms]')) el.classList.toggle('past', Number(el.dataset.ms) <= shownMs);
            // Playing a clip: it pauses at the clip's end.
            if (clipStop && clipTo !== null && !msg.seeking && !msg.loading && seekingTo === null && msg.ms >= clipTo) {
                clipStop = false;
                if (!wantPaused) togglePlay();
                status.textContent = 'End of the clip.';
            }
        } else if (msg.type === 'ended') {
            wantPaused = true;
            drawPlay();
            status.dataset.ended = '1';
            status.textContent = liveMode ? 'The session has ended.' : 'End of the recording.';
        } else if (msg.type === 'reload') {
            // A file from an older world: a fresh client, from there.
            status.dataset.ended = '';
            dockLetGo();
            mount(msg.ms);
        } else if (msg.type === 'preview') {
            drawPreview(msg);
        } else if (msg.type === 'error') {
            status.textContent = msg.message;
        }
    };

    const onKey = (e) => {
        if (e.target.closest('input, textarea, select') || e.ctrlKey || e.metaKey || e.altKey) return;
        // [ and ]: the clip starts or ends where it is.
        if ((e.key === '[' || e.key === ']') && detail) {
            e.preventDefault();
            (e.key === '[' ? markStart : markEnd)();
            return;
        }
        if (e.code !== 'Space' || e.target.closest('button') || playBtn.disabled) return;
        e.preventDefault();
        togglePlay();
    };
    document.addEventListener('keydown', onKey);
    ctx.cleanup(() => document.removeEventListener('keydown', onKey));

    playBtn.addEventListener('click', togglePlay);
    edgeBtn.addEventListener('click', () => seek(Math.max(0, total - liveDelay)));
    backBtn.addEventListener('click', () => skip(-10_000));
    forwardBtn.addEventListener('click', () => skip(10_000));
    lookBtn.addEventListener('click', () => {
        freeLook = !freeLook;
        send({type: 'freeLook', on: freeLook});
        drawLook();
        // It pauses.
        if (freeLook && !wantPaused) {
            wantPaused = true;
            drawPlay();
        }
        // Straight into the game, to use the arrow keys.
        if (freeLook) dock.main?.focus();
    });
    // Hovering the timeline: the time there, and the frame if it was watched.
    const previewImg = h('img', {alt: '', class: 'hidden'});
    const previewTime = h('span', {class: 'num'});
    const previewWaitText = h('span', {}, 'Loading previews...');
    const previewSpin = h('div', {class: 'replay-preview-wait hidden'}, h('div', {class: 'spinner'}), previewWaitText);
    const preview = h('div', {
        class: 'replay-preview hidden',
        'aria-hidden': 'true'
    }, previewImg, previewSpin, previewTime);
    let previewUrl = null;
    let previewFrame = null;
    let previewAsked = 0;
    let hoverMs = null;
    const showPreviewImage = (frameMs, blob) => {
        if (frameMs !== previewFrame) {
            if (previewUrl) URL.revokeObjectURL(previewUrl);
            previewUrl = blob ? URL.createObjectURL(blob) : null;
            previewFrame = frameMs;
            if (previewUrl) previewImg.src = previewUrl;
            previewImg.dataset.ms = frameMs ?? '';
        }
        previewImg.classList.toggle('hidden', !previewUrl);
        previewSpin.classList.toggle('hidden', !!previewUrl);
    };
    // The main replay's answer: a frame already watched, or none, when the
    // scout goes and gets one.
    const drawPreview = (msg) => {
        if (hoverMs === null || Math.abs(msg.ms - hoverMs) > 2000) return;
        if (msg.blob) {
            clearTimeout(shotTimer);
            shotTimer = null;
            showPreviewImage(msg.frameMs, msg.blob);
            return;
        }
        // The scout's frame for here already shows (this answer was sent
        // before the main replay had it).
        if (previewFrame !== null && Math.abs(previewFrame - hoverMs) <= previewWithin()) return;
        showPreviewImage(null, null);
        // Only once the mouse rests: passing over the timeline fetches nothing.
        // A hand's twitch (a pixel is a fraction of a second) doesn't count as
        // moving on.
        if (shotTimer !== null && Math.abs(shotWant - hoverMs) <= 1000) return;
        clearTimeout(shotTimer);
        shotWant = hoverMs;
        shotTimer = setTimeout(() => {
            shotTimer = null;
            if (hoverMs !== null && Math.abs(hoverMs - shotWant) <= 1500) requestShot(hoverMs);
        }, 200);
    };
    let shotTimer = null;
    let shotWant = null;
    ctx.cleanup(() => clearTimeout(shotTimer));
    ctx.cleanup(() => {
        if (previewUrl) URL.revokeObjectURL(previewUrl);
    });

    // The scout: a second, hidden, silent copy of the replay that fetches
    // frames of what hasn't been watched. Made on the first hover that needs
    // it. Its frames go to the main replay too, so seeking there shows them.
    let scout = null;
    let scoutReady = false;
    let scoutLoaded = false;
    let scoutWant = null;
    // How far apart the scout's sweep frames are: one stands in for anywhere
    // between them.
    let scoutStep = 0;
    const previewWithin = () => Math.max(2500, scoutStep * 0.6);
    let scoutAsked = null;
    let scoutAskedAt = 0;
    const requestShot = (ms) => {
        // Live, the scout would only have what came first.
        if (!detail?.player_url || !file || liveMode) return;
        const at = Math.max(1, Math.round(ms / 1000) * 1000);
        // Once per second of the timeline, unless that was a while ago.
        if (at === scoutAsked && performance.now() - scoutAskedAt < 3000) return;
        scoutAsked = at;
        scoutAskedAt = performance.now();
        if (dock.page !== pageCtl) return;
        if (!scout) {
            scoutWant = at;
            dockEnsure(detail.player_url, true);
            scout = dock.scout;
            if (dock.ready.scout) onScout({type: 'ready'});
        } else if (scoutLoaded) {
            dockPost('scout', {type: 'shoot', ms: at});
        } else {
            scoutWant = at;
        }
    };
    const onScout = (msg) => {
        if (msg.type === 'ready') {
            scoutReady = true;
            if (!scoutLoaded && file) {
                scoutLoaded = true;
                const copy = file.slice(0);
                dockPost('scout', {type: 'load', file: copy, seek: scoutWant || 1, speed: 1, paused: true}, [copy]);
            }
        } else if (msg.type === 'shot' && msg.blob) {
            // Its first frame: it has loaded.
            previewWaitText.remove();
            send({type: 'addShot', ms: msg.ms, blob: msg.blob, step: msg.step});
            scoutStep = Math.max(scoutStep, Number(msg.step) || 0);
            if (hoverMs !== null && Math.abs(msg.ms - hoverMs) <= previewWithin()) showPreviewImage(msg.ms, msg.blob);
        }
    };
    const onHover = (e) => {
        const rect = scrub.getBoundingClientRect();
        // The thumb's travel: the track less its width at each end.
        const f = Math.min(1, Math.max(0, (e.clientX - rect.left - 7) / Math.max(1, rect.width - 14)));
        hoverMs = Math.round(f * (Number(scrub.max) || 0));
        previewTime.textContent = clock(hoverMs / 1000);
        preview.classList.remove('hidden');
        preview.style.left = `${Math.min(rect.width - 80, Math.max(80, e.clientX - rect.left))}px`;
        // At most one question every 60ms, and always one for where it stops.
        const now = performance.now();
        clearTimeout(askTimer);
        if (now - previewAsked > 60) {
            previewAsked = now;
            send({type: 'preview', ms: hoverMs});
        } else {
            askTimer = setTimeout(() => {
                if (hoverMs === null) return;
                previewAsked = performance.now();
                send({type: 'preview', ms: hoverMs});
            }, 60);
        }
    };
    let askTimer = null;
    ctx.cleanup(() => clearTimeout(askTimer));
    scrub.addEventListener('pointermove', onHover);
    scrub.addEventListener('pointerleave', () => {
        hoverMs = null;
        preview.classList.add('hidden');
    });
    scrub.addEventListener('input', () => {
        dragging = true;
        time.textContent = `${clock(Number(scrub.value) / 1000)} / ${clock(Number(scrub.max) / 1000)}`;
    });
    scrub.addEventListener('change', () => {
        dragging = false;
        seek(Number(scrub.value));
    });
    drawSpeeds();
    drawPlay();

    const load = async () => {
        try {
            detail = await api('GET', `/replays/${id}`);
        } catch (e) {
            replace(head, pageHead('Replay', e.message));
            replace(stage, h('div', {class: 'empty'}, e.message));
            status.textContent = '';
            return;
        }
        const r = detail.replay;
        if (liveMode && !r.live) {
            liveMode = false;
            toast('That session has ended; playing its recording.');
        }
        const refs = [{type: 'replay', id: r.id}];
        const caseTitle = r.flags.length ? `Flagged session: ${FLAGS[r.flags[0]]?.[0] || r.flags[0]}` : '';
        replace(head, pageHead(
            h('span', {}, playerLink(r.player), fromLink ? '\'s session: a clip' : '\'s session', ' ', caseLinks(r.cases),
                detail.kept && h('span', {
                    class: 'badge',
                    title: 'The worlds have pruned it: this is the copy the console kept for a clip in a case.'
                }, 'Kept for a case')),
            [fromLink && `${clipSpan(clipFrom, clipTo)}${clipLabel.value ? `: ${clipLabel.value}` : ''} · `,
                `${r.world ? `World ${r.world}` : `Node ${r.node}`} · ${fullTime(r.started_ms)} · ${clock(r.duration_ms / 1000)}${r.live ? ' so far (still playing)' : ''} · revision ${r.revision}`],
            r.live && !liveMode && link(liveHref(r.id), {class: 'btn'}, icon('radio'), 'Watch live'),
            link(`/replays?${query({name: r.player})}`, {class: 'btn'}, 'Their sessions'),
            button('Add to case', {
                icon: 'plus',
                on: {click: () => addToCaseDialog({refs, player: r.player, what: ['this session'], title: caseTitle})}
            }),
            button('Open case', {
                icon: 'briefcase',
                on: {
                    click: () => newCaseDialog({
                        player: r.player,
                        refs,
                        what: ['this session'],
                        title: caseTitle,
                        priority: r.score >= 100 ? 'high' : 'normal'
                    })
                }
            })));

        replace(checks,
            r.score > 0
                ? h('div', {class: 'callout danger'}, icon('alert'), h('div', {},
                    h('b', {}, `The checks scored this session ${r.score}.`),
                    h('ul', {class: 'flag-list'}, r.flags.map((f) => h('li', {}, h('b', {}, FLAGS[f]?.[0] || f), ': ', FLAGS[f]?.[1] || '')))))
                : h('div', {class: 'callout'}, icon('check'), h('div', {}, 'Nothing suspicious found by the checks.')),
            replayStats(r));
        // Its length until the file says.
        total = r.duration_ms || 1;
        drawClips();
        drawClip();

        if (!detail.player_url) {
            replace(stage, h('div', {class: 'empty'}, icon('alert'), detail.why));
            status.textContent = '';
            return;
        }
        origin = new URL(detail.player_url).origin;
        // `t`: where to start, as from a report's "Their session"; a clip's
        // link starts at the clip.
        const start = fromLink ? clipFrom : Number(ctx.params.get('t')) || 0;
        mount(Math.min(Math.max(0, start), r.duration_ms));
        // A finished recording never changes; a live one has grown since.
        const finished = !r.live && r.ended_ms != null;
        const kept = !finished ? null : replayCache.get(id) || await storedReplay(id);
        if (liveMode) {
            try {
                file = await fetchLive();
            } catch (e) {
                dockDetach(pageCtl);
                cover.remove();
                replace(stage, h('div', {class: 'empty'}, e.message));
                status.textContent = '';
                return;
            }
        } else if (kept) {
            file = kept;
            cacheReplay(id, file);
            // Not downloaded, so not audited by the download: say it was watched.
            api('POST', `/replays/${id}/viewed`, {player: r.player}).catch(() => {
            });
        } else {
            try {
                const res = await fetch(`/api/replays/${id}/file?${query({
                    revision: r.revision,
                    player: r.player
                })}`, {credentials: 'same-origin'});
                if (!res.ok) {
                    let message = `Request failed (${res.status})`;
                    try {
                        message = (await res.json()).error || message;
                    } catch { /* not JSON */
                    }
                    throw new Error(message);
                }
                file = await res.arrayBuffer();
            } catch (e) {
                dockDetach(pageCtl);
                cover.remove();
                replace(stage, h('div', {class: 'empty'}, e.message));
                status.textContent = '';
                return;
            }
            if (finished) {
                cacheReplay(id, file);
                storeReplay(id, file);
            }
        }

        // The timeline: what they did, from the recording.
        const header = fileHeader(file);
        total = header.durationMs || r.duration_ms || 1;
        notes = header.notes;
        drawTimeline();
        if (liveMode) {
            liveToken = header.live.token;
            liveDelay = header.live.delayMs || 0;
            liveEnded = !!header.live.ended;
            edgeBtn.classList.remove('hidden');
            // From a little behind the end: what the player does comes in batches.
            loadSeek = Math.max(0, total - liveDelay);
            showSeek(loadSeek || null, 0);
            liveTimer = setTimeout(pollLive, LIVE_POLL_MS);
        }
        sendLoad();
        if (scoutReady && !scoutLoaded) onScout({type: 'ready'});
    };
    // The timeline's marks and notes, from what's come so far.
    const drawTimeline = () => {
        replace(marks, notes.map(([ms]) => h('span', {style: {left: `${Math.min(100, (ms / total) * 100)}%`}})));
        replace(timeline, notes.length
            ? notes.map(([ms, text]) => h('button', {
                    type: 'button',
                    class: 'list-item replay-note',
                    dataset: {ms},
                    on: {click: () => seek(ms)}
                },
                h('span', {class: 'num muted'}, clock(ms / 1000)), h('span', {}, text)))
            : h('div', {class: 'empty'}, 'Nothing noted.'));
        drawClip();
    };

    // Clips: marking one, playing it, and handing it on.
    const here = () => Math.round((seekingTo ?? atMs) / 100) * 100;
    const clipRef = () => ({type: 'clip', replay: id, from_ms: clipFrom, to_ms: clipTo, label: clipLabel.value.trim()});
    const markStart = () => {
        clipFrom = here();
        if (clipTo !== null && clipTo < clipFrom + 1000) clipTo = null;
        drawClip();
    };
    const markEnd = () => {
        const to = here();
        if (clipFrom === null || to < clipFrom + 1000) {
            toast('Mark where the clip starts first, at least a second before its end.', 'error');
            return;
        }
        clipTo = to;
        drawClip();
    };
    const playClip = (from = clipFrom, to = clipTo) => {
        if (from === null || to === null || starting) return;
        seek(from);
        clipTo = to;
        clipStop = true;
        if (wantPaused) togglePlay();
    };
    const copyClip = async () => {
        const url = `${location.origin}${clipHref(clipRef())}`;
        try {
            await navigator.clipboard.writeText(url);
            toast('Link copied: it opens this clip for anyone on the console.');
        } catch {
            toast(`Couldn't copy it. The link is ${url}`, 'error');
        }
    };
    // The bar under the controls, the range on the track, and the address
    // (so the address bar is the clip's link too).
    const drawClip = () => {
        const both = clipFrom !== null && clipTo !== null;
        clipRange.classList.toggle('hidden', clipFrom === null);
        clipRange.classList.toggle('open', clipFrom !== null && clipTo === null);
        if (clipFrom !== null) {
            const left = Math.min(100, (clipFrom / total) * 100);
            clipMark.style.left = `${left}%`;
            clipMark.style.width = clipTo === null ? '' : `${Math.min(100 - left, ((clipTo - clipFrom) / total) * 100)}%`;
        }
        const params = new URLSearchParams(location.search);
        for (const [k, v] of [['from', clipFrom], ['to', clipTo], ['label', both ? clipLabel.value.trim() : null]]) {
            if (both && v !== null && v !== '') params.set(k, String(v));
            else params.delete(k);
        }
        if (location.pathname === '/replay') history.replaceState(history.state, '', `/replay?${params}`);
        const r = detail?.replay;
        const what = both ? [`this clip (${clipSpan(clipFrom, clipTo)})`] : [];
        const caseTitle = clipLabel.value.trim() || (r ? `Clip of ${r.player}'s session` : '');
        replace(clipBar,
            h('span', {class: 'replay-clip-icon'}, icon('scissors')),
            button('Start', {small: true, title: 'The clip starts here ([)', on: {click: markStart}}),
            button('End', {small: true, title: 'The clip ends here (])', on: {click: markEnd}}),
            clipFrom === null
                ? h('span', {class: 'muted'}, 'Mark a stretch with [ and ] to share it or add it to a case.')
                : h('span', {class: 'num'}, both
                    ? `${clipSpan(clipFrom, clipTo)} (${clock((clipTo - clipFrom) / 1000)})`
                    : `From ${clock(clipFrom / 1000)}: mark where it ends`),
            both && [
                clipLabel,
                button('Play clip', {small: true, icon: 'play', on: {click: () => playClip()}}),
                button('Copy link', {small: true, icon: 'link', on: {click: copyClip}}),
                r && button('Add to case', {
                    small: true,
                    icon: 'plus',
                    on: {click: () => addToCaseDialog({refs: [clipRef()], player: r.player, what, title: caseTitle})}
                }),
                r && button('Open case', {
                    small: true,
                    icon: 'briefcase',
                    on: {click: () => newCaseDialog({player: r.player, refs: [clipRef()], what, title: caseTitle})}
                }),
            ],
            clipFrom !== null && button(null, {
                small: true,
                kind: 'ghost',
                icon: 'x',
                'aria-label': 'Clear the clip',
                title: 'Clear the clip',
                on: {
                    click: () => {
                        clipFrom = clipTo = null;
                        clipStop = false;
                        drawClip();
                    }
                }
            }));
    };
    clipLabel.addEventListener('input', debounce(() => clipFrom !== null && clipTo !== null && drawClip(), 300));
    // The clips of it in cases, to play.
    const drawClips = () => {
        replace(clipsList, detail.clips.length
            ? detail.clips.map((c) => h('div', {class: 'list-item replay-note'},
                h('button', {
                    type: 'button',
                    class: 'link-button',
                    title: 'Play it',
                    on: {
                        click: () => {
                            clipFrom = c.from_ms;
                            clipLabel.value = c.label;
                            playClip(c.from_ms, c.to_ms);
                            drawClip();
                        }
                    }
                }, h('span', {class: 'num'}, clipSpan(c.from_ms, c.to_ms))),
                h('span', {class: 'grow'}, c.label || h('span', {class: 'muted'}, 'No label')),
                caseLinks([c.case])))
            : h('div', {class: 'empty'}, 'No clips of it in cases yet.'));
    };

    // The live session from its world: everything so far, then (with the
    // token) what's come since.
    const fetchLive = async () => {
        const r = detail.replay;
        const res = await fetch(`/api/replays/${id}/live?${query({
            node: r.node,
            player: r.player,
            token: liveToken
        })}`, {credentials: 'same-origin'});
        if (!res.ok) {
            let message = `Request failed (${res.status})`;
            try {
                message = (await res.json()).error || message;
            } catch { /* not JSON */
            }
            throw new Error(message);
        }
        return res.arrayBuffer();
    };
    const pollLive = async () => {
        if (liveEnded || !liveToken) return;
        try {
            const more = await fetchLive();
            const header = fileHeader(more);
            appended.push(more);
            if (mainLoaded) {
                const copy = more.slice(0);
                send({type: 'append', file: copy}, [copy]);
            }
            total = Math.max(total, header.durationMs || 0);
            if (header.notes.length) {
                notes = notes.concat(header.notes);
                drawTimeline();
            } else {
                // The marks are placed by the length so far.
                replace(marks, notes.map(([ms]) => h('span', {style: {left: `${Math.min(100, (ms / total) * 100)}%`}})));
                drawClip();
            }
            if (header.live?.ended) {
                liveEnded = true;
                toast('The session has ended: the rest of it plays to its end.');
            }
        } catch (e) {
            liveEnded = true;
            status.dataset.ended = '1';
            status.textContent = `Stopped watching live: ${e.message}`;
        }
        if (!liveEnded) liveTimer = setTimeout(pollLive, LIVE_POLL_MS);
    };
    ctx.cleanup(() => {
        liveEnded = true;
        clearTimeout(liveTimer);
    });

    load();

    append(page,
        h('div', {class: 'replay-layout'},
            h('section', {class: 'card replay-card'},
                stage,
                h('div', {class: 'replay-controls'},
                    backBtn,
                    playBtn,
                    forwardBtn,
                    h('div', {class: 'replay-track'}, bufferBar, clipRange, scrub, marks, preview),
                    time,
                    speeds,
                    sizes,
                    edgeBtn,
                    lookBtn),
                clipBar,
                h('div', {class: 'replay-foot'}, status, footHint)),
            h('div', {class: 'replay-side'},
                h('section', {class: 'card'}, h('div', {class: 'card-head'}, h('h2', {}, 'Anti-cheat checks')), h('div', {class: 'card-body'}, checks)),
                h('section', {class: 'card'}, h('div', {class: 'card-head'}, h('h2', {}, 'Clips in cases')), clipsList),
                h('section', {class: 'card'}, h('div', {class: 'card-head'}, h('h2', {}, 'What they did')), timeline))));
    return page;
}

// ---------------------------------------------------------------------------
// Investigate: watching everyone at once. A live map of where players are
// (the world's own map viewer, framed, with the dots sent to it), the
// players the anti-cheat checks flag most, and groups of accounts that look
// like one bot farm.

const INVESTIGATE_TABS = [['map', 'Live map'], ['anticheat', 'Anti-cheat'], ['farms', 'Bot farms'], ['evasion', 'Ban evasion']];
const EVASION_WINDOWS = [[7, '7 days'], [14, '14 days'], [30, '30 days']];
const WINDOWS = [['24h', 'Last 24 hours'], ['7d', 'Last 7 days'], ['30d', 'Last 30 days']];

// The live recording of `name`'s session, watched live.
async function watchLive(name) {
    try {
        const [r] = await api('GET', `/replays?${query({name})}`);
        if (!r || !r.live) {
            toast(`${name} has no session going on now.`, 'error');
            return;
        }
        navigate(`/replay?${query({id: r.id, live: 1})}`);
    } catch (e) {
        toast(e.message, 'error');
    }
}

const liveHref = (id) => `/replay?${query({id, live: 1})}`;

// A small menu by the pointer: closed by Escape, a click elsewhere, or
// picking from it.
function popover(x, y, ...children) {
    document.querySelector('.popover')?.remove();
    const el = h('div', {class: 'popover', role: 'menu'}, children);
    document.body.append(el);
    const r = el.getBoundingClientRect();
    el.style.left = `${Math.max(8, Math.min(window.innerWidth - r.width - 8, x))}px`;
    el.style.top = `${Math.max(8, Math.min(window.innerHeight - r.height - 8, y))}px`;
    const close = () => {
        el.remove();
        document.removeEventListener('pointerdown', outside, true);
        document.removeEventListener('keydown', onKey, true);
    };
    const outside = (e) => {
        if (!el.contains(e.target)) close();
    };
    const onKey = (e) => {
        if (e.key === 'Escape') close();
    };
    setTimeout(() => {
        document.addEventListener('pointerdown', outside, true);
        document.addEventListener('keydown', onKey, true);
    }, 0);
    el.addEventListener('click', (e) => {
        if (e.target.closest('a, button')) close();
    });
    return close;
}

// What can be done to an online player from the map.
function playerActions(p, refresh) {
    const act = (action) => () => moderateDialog(p.name, action, refresh);
    return h('div', {class: 'pop-row'},
        h('div', {class: 'pop-main'},
            h('b', {}, playerLink(p.name)), ' ', playerBadges(p),
            p.flagged > 0 && h('span', {
                class: 'badge danger',
                title: `Flagged by the checks in the last day: highest score ${p.flagged}`
            }, icon('alert'), p.flagged),
            h('div', {class: 'list-sub num'}, `World ${p.world} · ${where(p.x, p.z, p.level)}`)),
        h('div', {class: 'chips'},
            p.replay && link(liveHref(p.replay), {
                class: 'btn small',
                title: 'Watch their session live'
            }, icon('play'), 'Live'),
            button(null, {
                small: true,
                icon: 'kick',
                'aria-label': `Kick ${p.name}`,
                title: 'Kick',
                on: {click: act('kick')}
            }),
            button(null, {
                small: true,
                icon: 'mute',
                'aria-label': `Mute ${p.name}`,
                title: 'Mute',
                on: {click: act('mute')}
            }),
            !isTrial() && button(null, {
                small: true,
                icon: 'ban',
                'aria-label': `Ban ${p.name}`,
                title: 'Ban',
                on: {click: act('ban')}
            })));
}

// Where players stand close together: `size` or more within `tiles` of the
// first, on the same floor, on any world. Biggest first.
function hotspots(players, size, tiles = 4) {
    const cells = new Map();
    const cellOf = (x, z, level) => `${level}:${Math.floor(x / tiles)}:${Math.floor(z / tiles)}`;
    for (const p of players) {
        const key = cellOf(p.x, p.z, p.level);
        if (!cells.has(key)) cells.set(key, []);
        cells.get(key).push(p);
    }
    const taken = new Set();
    const spots = [];
    const byCount = [...players].sort((a, b) => (cells.get(cellOf(b.x, b.z, b.level)).length - cells.get(cellOf(a.x, a.z, a.level)).length));
    for (const p of byCount) {
        if (taken.has(p)) continue;
        const near = [];
        const cx = Math.floor(p.x / tiles);
        const cz = Math.floor(p.z / tiles);
        for (let dx = -1; dx <= 1; dx++) {
            for (let dz = -1; dz <= 1; dz++) {
                for (const q of cells.get(`${p.level}:${cx + dx}:${cz + dz}`) || []) {
                    if (!taken.has(q) && Math.abs(q.x - p.x) <= tiles && Math.abs(q.z - p.z) <= tiles) near.push(q);
                }
            }
        }
        if (near.length >= size) {
            near.forEach((q) => taken.add(q));
            const x = Math.round(near.reduce((s, q) => s + q.x, 0) / near.length);
            const z = Math.round(near.reduce((s, q) => s + q.z, 0) / near.length);
            spots.push({x, z, level: p.level, players: near});
        }
    }
    return spots.sort((a, b) => b.players.length - a.players.length);
}

// The surface the world map shows, in tiles.
const onSurface = (p) => p.x >= 2048 && p.x < 3648 && p.z >= 2624 && p.z < 4032;

function liveMapTab(ctx, params) {
    const host = state.worlds.find((w) => w.online && w.client);
    if (!host) {
        return h('section', {class: 'card'}, h('div', {class: 'empty'}, icon('alert'),
            'No online world has a public address (client) in the console\'s worlds file to show the map from.'));
    }
    const url = `${host.client.replace(/\/+$/, '')}/livemap`;
    const origin = new URL(url).origin;
    const frame = h('iframe', {class: 'livemap-frame', src: url, title: 'Live map', referrerpolicy: 'no-referrer'});
    const stage = h('div', {class: 'livemap-stage'}, frame);
    const summary = h('div', {class: 'livemap-summary muted'}, 'Loading players...');
    const searchInput = h('input', {
        class: 'input',
        type: 'search',
        placeholder: 'Find an online player',
        'aria-label': 'Find a player',
        maxlength: '12'
    });
    const found = h('div', {class: 'list'});
    const spotsList = h('div', {class: 'list'});
    const worldChips = h('div', {class: 'chips'});
    const sizeChips = h('div', {class: 'segmented livemap-sizes', role: 'group', 'aria-label': 'Hotspot size'});
    let node = null;
    let spotSize = 4;
    let players = [];
    let flagged = new Map();
    let ready = false;
    let selected = (params.get('select') || '').split(',').map((n) => n.trim()).filter(Boolean);

    const shown = () => players.filter((p) => node === null || p.node === node);
    const tone = (p) => (flagged.has(nameKey(p.name)) ? 'flagged' : p.staff_level > 0 ? 'staff' : p.muted ? 'muted' : 'normal');
    const post = (msg) => frame.contentWindow?.postMessage(msg, origin);
    const sendPlayers = () => {
        if (!ready) return;
        post({
            type: 'players',
            players: shown().filter(onSurface).map((p) => ({
                name: p.name,
                x: p.x,
                z: p.z,
                level: p.level,
                tone: tone(p)
            }))
        });
    };
    const decorate = (p) => ({...p, flagged: flagged.get(nameKey(p.name)) || 0});
    const focusOn = (p, names) => {
        selected = names || [p.name];
        post({type: 'select', names: selected});
        if (onSurface(p)) post({type: 'focus', x: p.x, z: p.z});
        else toast(`${p.name ?? 'They'} are off the surface map (${where(p.x, p.z, p.level)}).`, 'error');
    };

    const drawWorlds = () => replace(worldChips,
        h('button', {
            type: 'button', class: 'chip', 'aria-pressed': String(node === null), on: {
                click: () => {
                    node = null;
                    drawWorlds();
                    draw();
                }
            }
        }, 'All worlds'),
        state.worlds.filter((w) => w.online).map((w) => h('button', {
            type: 'button',
            class: 'chip',
            'aria-pressed': String(node === w.node),
            on: {
                click: () => {
                    node = w.node;
                    drawWorlds();
                    draw();
                }
            }
        }, worldName(w))));
    const drawSizes = () => replace(sizeChips, [3, 4, 6, 10].map((n) => h('button', {
        type: 'button', 'aria-pressed': String(spotSize === n), title: `${n} or more players within 4 tiles`,
        on: {
            click: () => {
                spotSize = n;
                drawSizes();
                draw();
            }
        },
    }, `${n}+`)));

    const drawFound = () => {
        const q = nameKey(searchInput.value);
        if (!q) {
            replace(found);
            return;
        }
        const hits = shown().filter((p) => nameKey(p.name).includes(q)).slice(0, 8);
        replace(found, hits.length
            ? hits.map((p) => h('button', {type: 'button', class: 'list-item spot-item', on: {click: () => focusOn(p)}},
                h('div', {class: 'list-main'}, h('div', {class: 'list-title'}, h('b', {}, p.name), ' ', playerBadges(p)),
                    h('div', {class: 'list-sub num'}, `World ${p.world} · ${where(p.x, p.z, p.level)}`))))
            : h('div', {class: 'empty'}, 'Nobody online matches.'));
    };

    const draw = () => {
        const list = shown();
        const surface = list.filter(onSurface);
        const flaggedOnline = list.filter((p) => flagged.has(nameKey(p.name))).length;
        replace(summary,
            h('span', {}, h('b', {class: 'num'}, list.length.toLocaleString()), ' online'),
            h('span', {}, h('b', {class: 'num'}, (list.length - surface.length).toLocaleString()), ' underground or elsewhere'),
            h('span', {class: flaggedOnline ? 'tone-danger' : ''}, h('b', {class: 'num'}, flaggedOnline), ' flagged in the last day'));
        const spots = hotspots(list, spotSize);
        replace(spotsList, spots.length
            ? spots.slice(0, 30).map((s) => {
                const bad = s.players.filter((p) => flagged.has(nameKey(p.name))).length;
                const worlds = [...new Set(s.players.map((p) => p.world))];
                return h('button', {
                        type: 'button',
                        class: 'list-item spot-item',
                        on: {click: () => focusOn({x: s.x, z: s.z, level: s.level}, s.players.map((p) => p.name))}
                    },
                    h('div', {class: `list-icon ${bad ? 'danger' : ''}`}, h('b', {class: 'num'}, s.players.length)),
                    h('div', {class: 'list-main'},
                        h('div', {class: 'list-title'}, clip(s.players.map((p) => p.name).join(', '), 70)),
                        h('div', {class: 'list-sub num'}, `${where(s.x, s.z, s.level)} · ${worlds.length === 1 ? `World ${worlds[0]}` : plural(worlds.length, 'world')}${bad ? ` · ${bad} flagged` : ''}`)));
            })
            : h('div', {class: 'empty'}, `No ${spotSize} or more players stand together.`));
        drawFound();
        sendPlayers();
    };

    const load = async () => {
        try {
            const data = await api('GET', '/live/players');
            players = data.players.map(decorate);
            if (data.unreachable.length) toast(`${plural(data.unreachable.length, 'world')} didn't answer.`, 'error');
            draw();
        } catch (e) {
            summary.textContent = e.message;
        }
    };
    const loadFlagged = async () => {
        try {
            const board = await api('GET', '/live/anticheat?window=24h');
            flagged = new Map(board.leaders.map((l) => [nameKey(l.name), l.max_score]));
            players = players.map(decorate);
            draw();
        } catch { /* the map works without */
        }
    };

    const onMessage = (e) => {
        if (e.source !== frame.contentWindow || e.origin !== origin || e.data?.source !== 'rs-livemap') return;
        const msg = e.data;
        if (msg.type === 'ready') {
            ready = true;
            sendPlayers();
            if (selected.length) {
                const first = players.find((p) => selected.includes(p.name));
                post({type: 'select', names: selected});
                if (first && onSurface(first)) post({type: 'focus', x: first.x, z: first.z});
            }
        } else if (msg.type === 'pick' && Array.isArray(msg.names)) {
            const picked = msg.names.map((n) => players.find((p) => p.name === n)).filter(Boolean);
            if (!picked.length) return;
            const r = frame.getBoundingClientRect();
            popover(r.left + msg.x + 8, r.top + msg.y + 8,
                picked.length > 1 && h('div', {class: 'pop-head'}, `${picked.length} players here`),
                h('div', {class: 'pop-list'}, picked.slice(0, 40).map((p) => playerActions(p, load))),
                picked.length > 40 && h('div', {class: 'pop-head muted'}, `and ${picked.length - 40} more`));
        }
    };
    window.addEventListener('message', onMessage);
    ctx.cleanup(() => window.removeEventListener('message', onMessage));
    searchInput.addEventListener('input', drawFound);
    searchInput.addEventListener('keydown', (e) => {
        if (e.key !== 'Enter') return;
        const q = nameKey(searchInput.value);
        const p = shown().find((x) => nameKey(x.name) === q) || shown().find((x) => nameKey(x.name).includes(q));
        if (p) focusOn(p);
    });
    drawWorlds();
    drawSizes();
    load();
    loadFlagged();
    const timer = setInterval(() => {
        if (!document.hidden) load();
    }, 3000);
    const flaggedTimer = setInterval(() => {
        if (!document.hidden) loadFlagged();
    }, 5 * 60 * 1000);
    ctx.cleanup(() => {
        clearInterval(timer);
        clearInterval(flaggedTimer);
        document.querySelector('.popover')?.remove();
    });

    const legend = h('div', {class: 'livemap-legend'},
        [['normal', 'Player'], ['flagged', 'Flagged in the last day'], ['staff', 'Staff'], ['muted', 'Muted']]
            .map(([t, label]) => h('span', {}, h('i', {class: `map-dot ${t}`}), label)),
        h('span', {class: 'muted'}, 'Click a dot for what you can do. Drag to move, 1-4 to zoom.'));
    return h('div', {class: 'livemap-layout'},
        h('section', {class: 'card livemap-card'}, stage, h('div', {class: 'livemap-foot'}, summary, legend)),
        h('div', {class: 'column'},
            h('section', {class: 'card'},
                h('div', {class: 'livemap-tools'}, h('div', {class: 'search'}, icon('search'), searchInput), worldChips),
                found),
            h('section', {class: 'card'},
                h('div', {class: 'card-head'}, h('h2', {}, 'Hotspots'), h('div', {class: 'actions'}, sizeChips)),
                h('p', {class: 'card-note muted'}, 'Players standing within 4 tiles of each other. Bot farms bunch up; so do banks.'),
                h('div', {class: 'spot-list'}, spotsList))));
}

// A leader's highest score in each stretch of the window, as bars.
function trendBars(trend, sinceMs) {
    const svg = document.createElementNS(SVG, 'svg');
    const w = 120;
    const hgt = 28;
    const n = trend.length || 1;
    const gap = n > 24 ? 1 : 2;
    const bw = Math.max(1, (w - gap * (n - 1)) / n);
    const top = Math.max(100, ...trend);
    const span = Date.now() - sinceMs;
    svg.setAttribute('viewBox', `0 0 ${w} ${hgt}`);
    svg.setAttribute('class', 'trend');
    svg.setAttribute('role', 'img');
    svg.setAttribute('aria-label', `Highest score per stretch, oldest first: ${trend.join(', ')}`);
    trend.forEach((v, i) => {
        const x = i * (bw + gap);
        const hit = document.createElementNS(SVG, 'rect');
        hit.setAttribute('x', x);
        hit.setAttribute('y', 0);
        hit.setAttribute('width', bw + gap);
        hit.setAttribute('height', hgt);
        hit.setAttribute('class', 'trend-hit');
        const title = document.createElementNS(SVG, 'title');
        const from = sinceMs + (span * i) / n;
        title.textContent = `${shortTime(from)}: ${v ? `highest score ${v}` : 'nothing flagged'}`;
        hit.append(title);
        const bar = document.createElementNS(SVG, 'rect');
        const bh = v ? Math.max(2, (v / top) * (hgt - 2)) : 1;
        bar.setAttribute('x', x);
        bar.setAttribute('y', hgt - bh);
        bar.setAttribute('width', bw);
        bar.setAttribute('height', bh);
        bar.setAttribute('rx', Math.min(1.5, bw / 2));
        bar.setAttribute('class', v ? 'trend-bar' : 'trend-zero');
        svg.append(bar, hit);
    });
    return svg;
}

// Whether a leader is getting worse: more flagged in the latest third of the
// window than in the rest, by stretch.
function trendWord(trend) {
    const n = trend.length;
    if (n < 3) return null;
    const cut = Math.floor((n * 2) / 3);
    const rate = (a) => a.filter((v) => v > 0).length / Math.max(1, a.length);
    const early = rate(trend.slice(0, cut));
    const late = rate(trend.slice(cut));
    if (late > early * 1.5 && late > 0) return h('span', {
        class: 'badge danger',
        title: 'Flagged more often lately than earlier in the window'
    }, 'Rising');
    if (late === 0 && early > 0) return h('span', {class: 'badge', title: 'Nothing flagged lately'}, 'Quiet lately');
    return null;
}

function anticheatTab(ctx, params, go) {
    const win = WINDOWS.some(([k]) => k === params.get('window')) ? params.get('window') : '24h';
    const chips = h('div', {class: 'chips'}, WINDOWS.map(([k, label]) =>
        h('button', {
            type: 'button',
            class: 'chip',
            'aria-pressed': String(win === k),
            on: {click: () => go('anticheat', {window: k})}
        }, label)));
    const body = tableWindow(spinnerList());
    const note = h('p', {class: 'muted'});

    api('GET', `/live/anticheat?${query({window: win})}`).then((board) => {
        note.textContent = `Players with sessions flagged by the anti-cheat checks since ${fullTime(board.since_ms)}, most flagged first. The bars are each stretch's highest score.`;
        if (!board.leaders.length) {
            replace(body, h('div', {class: 'empty'}, icon('check'), 'No sessions were flagged in this window.'));
            return;
        }
        replace(body, h('table', {class: 'table leaders'},
            h('thead', {}, h('tr', {},
                h('th', {class: 'num'}, '#'), h('th', {}, 'Player'), h('th', {
                    class: 'num',
                    title: 'Flagged sessions / all sessions'
                }, 'Flagged'),
                h('th', {class: 'num'}, 'Top'), h('th', {
                    class: 'num',
                    title: 'Every flagged session\'s score, added up'
                }, 'Total'),
                h('th', {}, 'Over the window'), h('th', {}, 'Checks failed'), h('th', {}, 'Last flagged'), h('th', {}))),
            h('tbody', {}, board.leaders.map((l, i) => h('tr', {},
                h('td', {class: 'num muted'}, i + 1),
                h('td', {}, h('div', {class: 'name-cell'}, playerLink(l.name),
                    l.online_world && h('span', {class: 'badge ok', title: 'Online now'}, `W${l.online_world}`),
                    l.banned_until_ms && h('span', {class: 'badge danger'}, 'Banned'),
                    l.muted_until_ms && h('span', {class: 'badge warn'}, 'Muted'))),
                h('td', {class: 'num'}, `${l.flagged} / ${l.sessions}`),
                h('td', {class: `num ${l.max_score >= 100 ? 'tone-danger' : ''}`}, l.max_score),
                h('td', {class: 'num'}, l.total_score.toLocaleString()),
                h('td', {}, h('div', {class: 'trend-cell'}, trendBars(l.trend, board.since_ms), trendWord(l.trend))),
                h('td', {}, h('div', {class: 'chips'}, l.flags.map(flagBadge))),
                h('td', {class: 'num muted', title: fullTime(l.last_flagged_ms)}, ago(l.last_flagged_ms)),
                h('td', {}, h('div', {class: 'chips nowrap'},
                    link(`/replays?${query({name: l.name, flagged: 1})}`, {class: 'btn small ghost'}, 'Sessions'),
                    l.online_world && button('Live', {
                        small: true,
                        icon: 'play',
                        title: 'Watch their session live',
                        on: {click: () => watchLive(l.name)}
                    }),
                    button(null, {
                        small: true,
                        icon: 'briefcase',
                        'aria-label': `Open a case on ${l.name}`,
                        title: 'Open case',
                        on: {
                            click: () => newCaseDialog({
                                player: l.name,
                                title: `Flagged by the checks: ${FLAGS[l.flags[0]]?.[0] || 'anti-cheat'}`,
                                priority: l.max_score >= 100 ? 'high' : 'normal'
                            })
                        }
                    }),
                    !isTrial() && button(null, {
                        small: true,
                        icon: 'ban',
                        'aria-label': `Ban ${l.name}`,
                        title: 'Ban',
                        on: {click: () => moderateDialog(l.name, 'ban')}
                    }))))))));
    }).catch((e) => failed(body, e));

    return h('div', {class: 'column'},
        h('div', {class: 'toolbar'}, chips),
        note,
        h('section', {class: 'card'}, body));
}

const REASONS = {
    registered: (r) => ['Registered from one address', r.ip],
    logged_in: (r) => ['Logged in from one address', r.ip],
    together: () => ['Registered within 15 minutes', null],
    names: () => ['Names alike', null],
    rhythm: () => ['The same even click rhythm', null],
    spot: () => ['Saved at the same spot', null],
    hours: () => ['Play the same hours', null],
};

// Which hours of the day (UTC) an account played, as a strip of 24 cells.
function hourStrip(hours) {
    const top = Math.max(1, ...hours);
    return h('div', {class: 'hours', role: 'img', 'aria-label': `Sessions by hour of day (UTC): ${hours.join(', ')}`},
        hours.map((n, i) => h('span', {
            class: n ? 'on' : '',
            title: `${String(i).padStart(2, '0')}:00 UTC: ${plural(n, 'session')}`,
            style: n ? {opacity: String(0.35 + (0.65 * n) / top)} : null,
        })));
}

function farmCard(f) {
    const names = f.members.map((m) => m.name);
    const online = f.members.filter((m) => m.online_world);
    const banned = f.members.filter((m) => m.banned_until_ms).length;
    const shared = f.shared.map((s) => {
        const [label, ip] = (REASONS[s.kind] || (() => [s.kind, null]))(s);
        return h('span', {
                class: `badge ${s.kind === 'registered' || s.kind === 'rhythm' ? 'danger' : ''}`,
                title: `${s.accounts} of the ${f.members.length} accounts`
            },
            label, ip && h('span', {class: 'mono'}, ` ${ip}`), h('span', {class: 'num muted'}, ` ×${s.accounts}`));
    });
    return h('section', {class: 'card farm'},
        h('div', {class: 'card-head'},
            h('h2', {}, `${f.members.length} accounts`),
            h('span', {
                class: 'badge',
                title: 'Bigger groups, more in common and flagged sessions score higher'
            }, `score ${f.score}`),
            banned > 0 && h('span', {class: 'badge danger'}, `${banned} banned`),
            online.length > 0 && h('span', {class: 'badge ok'}, `${online.length} online`),
            h('div', {class: 'actions'},
                online.length > 0 && link(`/investigate?${query({
                    tab: 'map',
                    select: online.map((m) => m.name).join(',')
                })}`, {class: 'btn small ghost'}, icon('pin'), 'On the map'),
                button('Open case', {
                    small: true,
                    icon: 'briefcase',
                    on: {
                        click: () => newCaseDialog({
                            player: names[0],
                            title: `Possible bot farm: ${clip(listNames(names), 80)}`,
                            priority: f.score >= 100 ? 'high' : 'normal'
                        })
                    }
                }))),
        h('div', {class: 'farm-shared'}, shared),
        scrollWindow(h('div', {class: 'list'}, f.members.map((m) => h('div', {class: 'list-item farm-member'},
            h('div', {class: 'list-main'},
                h('div', {class: 'list-title'}, h('b', {}, playerLink(m.name)), ' ',
                    m.online_world && h('span', {class: 'badge ok'}, `W${m.online_world}`),
                    m.banned_until_ms && h('span', {class: 'badge danger'}, 'Banned'),
                    m.muted_until_ms && h('span', {class: 'badge warn'}, 'Muted'),
                    m.flagged > 0 && h('span', {
                        class: 'badge danger',
                        title: `Flagged sessions; highest score ${m.max_score}`
                    }, icon('alert'), `${m.flagged} flagged`)),
                h('div', {class: 'list-sub'}, joined([
                    `registered ${ago(m.registered_ms)}`,
                    `${plural(m.sessions, 'session')}, ${span(m.play_ms / 1000)}`,
                    m.rhythm != null && `rhythm ${m.rhythm.toFixed(3)}`,
                    m.x != null && h('span', {class: 'num'}, where(m.x, m.z, m.level)),
                    m.registration_ip && h('span', {class: 'mono', title: 'Registered from'}, m.registration_ip),
                ]))),
            hourStrip(m.hours),
            h('div', {class: 'chips nowrap'},
                m.online_world && button(null, {
                    small: true,
                    icon: 'play',
                    'aria-label': `Watch ${m.name} live`,
                    title: 'Watch live',
                    on: {click: () => watchLive(m.name)}
                }),
                !isTrial() && button(null, {
                    small: true,
                    icon: 'ban',
                    'aria-label': `Ban ${m.name}`,
                    title: 'Ban',
                    on: {click: () => moderateDialog(m.name, 'ban')}
                })))))));
}

function farmsTab(ctx, params, go) {
    const win = WINDOWS.some(([k]) => k === params.get('window')) ? params.get('window') : '7d';
    const chips = h('div', {class: 'chips'}, WINDOWS.map(([k, label]) =>
        h('button', {
            type: 'button',
            class: 'chip',
            'aria-pressed': String(win === k),
            on: {click: () => go('farms', {window: k})}
        }, label)));
    const body = h('div', {class: 'column'}, h('section', {class: 'card'}, spinnerList()));
    const note = h('p', {class: 'muted'}, 'Weighing accounts...');

    api('GET', `/live/farms?${query({window: win})}`).then((r) => {
        note.textContent = `${plural(r.accounts, 'account')} active since ${fullTime(r.since_ms)}${r.capped ? ' (the most recently active; there are more)' : ''}, grouped where they share an address, or several of: registering together, alike names, the same even click rhythm, the same spot and the same hours.${r.addresses_shown ? ' Viewing addresses is recorded in the audit log.' : ' Addresses show as labels: the same label is the same address.'}`;
        replace(body, r.farms.length
            ? r.farms.map(farmCard)
            : h('section', {class: 'card'}, h('div', {class: 'empty'}, icon('check'), 'No accounts look grouped in this window.')));
    }).catch((e) => replace(body, h('section', {class: 'card'}, h('div', {class: 'empty'}, e.message))));

    return h('div', {class: 'column'}, h('div', {class: 'toolbar'}, chips), note, body);
}

// Accounts that logged in with a banned account's browser, or from its
// address, after its ban.
function evasionTab(ctx, params, go) {
    const days = EVASION_WINDOWS.some(([d]) => String(d) === params.get('days')) ? Number(params.get('days')) : 14;
    const chips = h('div', {class: 'chips'}, EVASION_WINDOWS.map(([d, label]) =>
        h('button', {
            type: 'button',
            class: 'chip',
            'aria-pressed': String(days === d),
            on: {click: () => go('evasion', {days: d})}
        }, label)));
    const list = spinnerList();
    api('GET', `/lookup/evasion?${query({days})}`).then((rows) => replace(list, rows.length
        ? rows.map((r) => h('div', {class: 'list-item'},
            h('div', {class: `list-icon ${r.via === 'address' ? 'warn' : 'danger'}`}, icon(r.via === 'address' ? 'globe' : 'monitor')),
            h('div', {class: 'list-main'},
                h('div', {class: 'list-title'}, h('b', {}, playerLink(r.name)), ' ', playerBadges({staff_level: r.staff_level}),
                    r.via === 'address' ? ' logged in from the address of banned ' : ' logged in with the browser of banned ', playerLink(r.banned),
                    ' ', caseLinks(r.cases)),
                h('div', {class: 'list-sub'}, joined([
                    `${plural(r.logins, 'login')} since the ban`,
                    `first ${ago(r.first_ms)}`,
                    h('span', {title: fullTime(r.banned_ms)}, `banned ${ago(r.banned_ms)}`),
                ]))),
            h('div', {class: 'list-side', title: fullTime(r.last_ms)}, ago(r.last_ms))))
        : h('div', {class: 'empty'}, icon('check'), 'Nobody logged in with the browser or from the address of a banned account in this window.')))
        .catch((e) => failed(list, e));
    return h('div', {class: 'column'},
        h('div', {class: 'toolbar'}, chips),
        h('p', {class: 'muted'}, 'Accounts not banned themselves that logged in with the same browser (or desktop client install) as a banned account, or from one of its addresses, after its ban. A shared browser is the stronger sign; an address can be a household. Addresses shared by more than 25 accounts are skipped. Each one opens a case on its own. Shared family computers show up here too: check the player page before acting.'),
        h('section', {class: 'card'}, scrollWindow(list)));
}

function investigatePage(ctx) {
    const tab = INVESTIGATE_TABS.some(([k]) => k === ctx.params.get('tab')) ? ctx.params.get('tab') : 'map';
    const go = (next, extra = {}) => navigate(`/investigate?${query({tab: next, ...extra})}`);
    const tabs = h('div', {
        class: 'segmented tabs',
        role: 'group',
        'aria-label': 'View'
    }, INVESTIGATE_TABS.map(([k, label]) =>
        h('button', {
            type: 'button', 'aria-pressed': String(tab === k), on: {
                click: () => {
                    if (k !== tab) go(k);
                }
            }
        }, label)));
    const body = tab === 'map'
        ? (state.worldsLoaded ? liveMapTab(ctx, ctx.params) : h('section', {class: 'card'}, spinnerList()))
        : tab === 'anticheat' ? anticheatTab(ctx, ctx.params, go)
            : tab === 'evasion' ? evasionTab(ctx, ctx.params, go) : farmsTab(ctx, ctx.params, go);
    if (tab === 'map' && !state.worldsLoaded) {
        // The map is framed from an online world, known once the worlds are in.
        const off = onWorlds(() => {
            off();
            if (location.pathname === '/investigate') renderPage();
        });
        ctx.cleanup(off);
    }
    return h('div', {class: 'page'},
        pageHead('Investigate', 'Everyone at once: where they are, who the checks flag most, accounts that look like one bot farm, and banned players coming back.'),
        tabs,
        body);
}

// ---------------------------------------------------------------------------
// Economy: what's in circulation, who holds an item, where a player's item
// came from and who trades with whom; rolling a dupe ring back; keeping
// accounts off the hiscores.

const ECONOMY_TABS = [
    ['circulation', 'Circulation'],
    ['holders', 'Who owns'],
    ['trace', 'Item trace'],
    ['trades', 'Trade graph'],
    ['hiscores', 'Hiscores'],
];
// The wealth events the engine logs itself as items pass between players
// (rs_db::transfer).
const TRANSFER = {trade: 100, stake: 101, give: 102, drop: 103, take: 104};
// A handover as the player it went to sees it.
const RECEIVED_TYPES = {100: 'Received in a trade', 101: 'Won as a duel stake', 102: 'Handed to them'};
// The website's hiscore tables: 0 is Overall, any other a stat id + 1.
const HISCORE_TABLES = [
    [0, 'Overall'], [1, 'Attack'], [2, 'Defence'], [3, 'Strength'], [4, 'Hitpoints'], [5, 'Ranged'],
    [6, 'Prayer'], [7, 'Magic'], [8, 'Cooking'], [9, 'Woodcutting'], [10, 'Fletching'], [11, 'Fishing'],
    [12, 'Firemaking'], [13, 'Crafting'], [14, 'Smithing'], [15, 'Mining'], [16, 'Herblore'],
    [17, 'Agility'], [18, 'Thieving'], [21, 'Runecrafting'],
];
const tableName = (id) => HISCORE_TABLES.find(([t]) => t === id)?.[1] || `Table ${id}`;
const economyHref = (tab, params = {}) => `/economy?${query({tab, ...params})}`;

// How a holder came by some of an item: [what happened, icon].
function sourceLabel(s) {
    switch (s.event_type) {
        case TRANSFER.trade:
            return ['Traded to them by', 'users'];
        case TRANSFER.stake:
            return ['Won as a duel stake from', 'flag'];
        case TRANSFER.give:
            return ['Handed to them by', 'users'];
        case TRANSFER.take:
            return s.from ? ['Picked up what was dropped by', 'download'] : ['Picked up from the floor', 'download'];
        default:
            return [WEALTH_TYPES[s.event_type] || `Wealth event ${s.event_type}`, 'zap'];
    }
}

function economyPage(ctx) {
    const tab = ECONOMY_TABS.some(([k]) => k === ctx.params.get('tab')) ? ctx.params.get('tab') : 'circulation';
    const go = (next, extra = {}) => navigate(economyHref(next, extra));
    const tabs = h('div', {class: 'segmented tabs', role: 'group', 'aria-label': 'View'},
        ECONOMY_TABS.map(([k, label]) => h('button', {
            type: 'button',
            'aria-pressed': String(tab === k),
            on: {click: () => k !== tab && go(k)},
        }, label)));
    const body = {
        circulation: circulationTab,
        holders: holdersTab,
        trace: traceTab,
        trades: tradesTab,
        hiscores: hiscoresTab,
    }[tab](ctx, ctx.params, go);
    return h('div', {class: 'page'},
        pageHead('Economy', 'What\'s in circulation, who holds it, where it came from and who hands items to whom.'),
        tabs,
        body);
}

// A count of every tracked item rose more than its threshold.
function alertItem(a, onDone) {
    const rise = a.to - a.from;
    const pct = a.from ? `+${((rise / a.from) * 100).toFixed(1)}%` : 'from none';
    return h('div', {class: 'list-item'},
        h('div', {class: `list-icon ${a.acknowledged ? '' : 'danger'}`}, icon('alert')),
        h('div', {class: 'list-main'},
            h('div', {class: 'list-title'}, h('b', {}, a.name), `: ${a.from.toLocaleString()} to ${a.to.toLocaleString()} `,
                h('span', {class: `badge ${a.acknowledged ? '' : 'danger'}`}, pct)),
            h('div', {class: 'list-sub'}, joined([
                `between ${shortTime(a.since_ms)} and ${shortTime(a.at_ms)}`,
                a.acknowledged && `seen by ${a.acknowledged.by} ${ago(a.acknowledged.at_ms)}`,
            ]))),
        h('div', {class: 'chips nowrap'},
            link(economyHref('holders', {obj: a.obj}), {class: 'btn small ghost'}, icon('users'), 'Who owns it'),
            !a.acknowledged && button('Seen', {
                small: true,
                icon: 'check',
                title: 'Mark it as looked at, for everyone',
                on: {
                    click: async () => {
                        try {
                            await api('POST', `/economy/alerts/${a.id}/acknowledge`);
                            onDone();
                        } catch (e) {
                            toast(e.message, 'error');
                        }
                    },
                },
            })));
}

function trackDialog(tracked, onDone) {
    const picker = tracked ? null : objPicker();
    const pct = h('input', {
        class: 'input',
        type: 'number',
        min: '0.1',
        max: '1000',
        step: '0.1',
        required: true,
        value: String(tracked?.alert_pct ?? 5),
        'data-invalid': 'Alert on a rise of 0.1% to 1000%.',
    });
    openDialog({
        title: tracked ? `Alert threshold: ${tracked.name}` : 'Track an item',
        subtitle: 'Counted across every save every 15 minutes. A count that rises this much over the one before raises an alert.',
        body: [
            picker && h('label', {class: 'field'}, h('span', {}, 'Item'), picker.search, picker.results),
            h('label', {class: 'field'}, h('span', {}, 'Alert on a rise of (%)'), pct,
                h('span', {class: 'hint'}, 'Rares nothing in the game makes more of can be set low: any rise is a dupe or a staff spawn.')),
        ],
        actions: [{label: 'Cancel'}, {
            label: tracked ? 'Save' : 'Track',
            kind: 'primary',
            icon: tracked ? 'check' : 'plus',
            run: async () => {
                const obj = tracked ? tracked.obj : picker.chosen()?.id;
                if (obj == null) throw new Error('Pick an item from the results.');
                await api('POST', '/economy/track', {obj, alert_pct: Number(pct.value)});
                toast(tracked ? 'Threshold saved.' : 'Tracking it from the next count.');
                onDone();
            },
        }],
    });
}

// One tracked item's count over time.
function circulationCard(t, samples, canChange, onDone) {
    const points = samples.flatMap((s) => {
        const item = s.items.find((i) => i.obj === t.obj);
        return item ? [{at_ms: s.at_ms, total: item.total, kind: plural(item.holders, 'holder')}] : [];
    });
    const last = points[points.length - 1];
    const body = points.length >= 2
        ? lineChart(points, 'total', 'In circulation', (n) => compact(n))
        : h('div', {class: 'empty'}, icon('chart'), last
            ? `${last.total.toLocaleString()} in circulation; a chart once it's counted again.`
            : 'Not counted yet.');
    return h('section', {class: 'card'},
        h('div', {class: 'card-head'},
            h('h2', {}, link(economyHref('holders', {obj: t.obj}), {class: 'name-link'}, t.name)),
            h('span', {class: 'badge', title: 'A rise this big between two counts alerts'}, `alerts at +${t.alert_pct}%`),
            canChange && h('div', {class: 'actions'},
                button(null, {
                    small: true,
                    kind: 'ghost',
                    icon: 'edit',
                    'aria-label': `Change the threshold for ${t.name}`,
                    title: 'Change the threshold',
                    on: {click: () => trackDialog(t, onDone)},
                }),
                button(null, {
                    small: true,
                    kind: 'ghost',
                    icon: 'trash',
                    'aria-label': `Stop tracking ${t.name}`,
                    title: 'Stop tracking',
                    on: {
                        click: () => confirmDialog({
                            title: `Stop tracking ${t.name}?`,
                            subtitle: 'Its counts so far are kept; it\'s no longer counted or alerted on.',
                            label: 'Stop tracking',
                            kind: 'danger',
                            run: async () => {
                                await api('POST', '/economy/untrack', {obj: t.obj});
                                onDone();
                            },
                        }),
                    },
                }))),
        h('div', {class: 'card-body'}, body));
}

function circulationTab(ctx, params, go) {
    const days = [7, 30, 90, 365].includes(Number(params.get('days'))) ? Number(params.get('days')) : 30;
    const alerts = h('div');
    const grid = h('div', {class: 'econ-grid'}, h('div', {class: 'empty'}, h('div', {class: 'spinner'})));
    const actions = h('div', {class: 'chips'});
    const load = async () => {
        let d;
        try {
            d = await api('GET', `/economy?${query({days})}`);
        } catch (e) {
            return failed(grid, e);
        }
        const open = d.alerts.filter((a) => !a.acknowledged);
        const seen = d.alerts.filter((a) => a.acknowledged).slice(0, 10);
        replace(alerts,
            open.length > 0 && h('section', {class: 'card danger-card'},
                h('div', {class: 'card-head'}, h('h2', {}, plural(open.length, 'alert')),
                    h('span', {class: 'hint'}, 'A count rose more than its item\'s threshold: often the first sign of a dupe.')),
                h('div', {class: 'list'}, open.map((a) => alertItem(a, load)))),
            seen.length > 0 && h('details', {class: 'card'},
                h('summary', {class: 'card-head'}, h('h2', {}, 'Earlier alerts')),
                h('div', {class: 'list'}, seen.map((a) => alertItem(a, load)))));
        replace(actions,
            [7, 30, 90, 365].map((n) => h('button', {
                type: 'button',
                class: 'chip',
                'aria-pressed': String(n === days),
                on: {click: () => n !== days && go('circulation', {days: n})},
            }, n === 365 ? 'Year' : `${n} days`)),
            d.can_change && button('Count now', {
                small: true,
                icon: 'refresh',
                on: {
                    click: async (e) => {
                        const btn = e.currentTarget;
                        btn.disabled = true;
                        try {
                            await api('POST', '/economy/sample');
                            toast('Counted.');
                            load();
                        } catch (err) {
                            toast(err.message, 'error');
                            btn.disabled = false;
                        }
                    },
                },
            }),
            d.can_change && button('Track an item', {small: true, icon: 'plus', on: {click: () => trackDialog(null, load)}}));
        replace(grid, d.tracked.length
            ? d.tracked.map((t) => circulationCard(t, d.samples, d.can_change, load))
            : h('div', {class: 'empty'}, icon('chart'), 'Nothing is tracked yet. Coins and the holiday rares are added once a world answers.'));
    };
    load();
    return h('div', {class: 'column'},
        alerts,
        h('div', {class: 'toolbar'}, actions,
            h('span', {class: 'hint hide-mobile'}, 'Everything every save holds (bank, worn and noted too), counted every 15 minutes.')),
        grid);
}

const holderBadges = (a) => [
    a.staff_level > 0 && h('span', {class: 'badge', title: 'Game staff level'}, `Staff ${a.staff_level}`),
    a.online_node != null && h('span', {class: 'badge ok'}, worldOf(a.online_node)),
    a.banned_until_ms && h('span', {class: 'badge danger'}, 'Banned'),
];

function holdersTab(ctx, params, go) {
    const obj = params.get('obj');
    const min = Math.max(1, Number(params.get('min')) || 1);
    const minInput = h('input', {class: 'input', type: 'number', min: '1', value: String(min), 'aria-label': 'At least', style: {width: '120px'}});
    const picker = objPicker({
        required: false,
        autofocus: !obj,
        placeholder: 'Find an item: name, config name or id',
        onPick: (o) => go('holders', {obj: o.id, min: minInput.value > 1 ? minInput.value : ''}),
    });
    minInput.addEventListener('change', () => obj && go('holders', {obj, min: minInput.value > 1 ? minInput.value : ''}));
    const result = h('div', {class: 'column'});
    if (obj) {
        replace(result, h('section', {class: 'card'}, spinnerList()));
        api('GET', `/economy/holders?${query({obj, min})}`).then((r) => {
            if (!r) return replace(result, h('div', {class: 'empty'}, 'There\'s no such item.'));
            const rows = r.accounts.map((a) => h('tr', {},
                h('td', {}, h('div', {class: 'name-cell'}, playerLink(a.name), holderBadges(a))),
                h('td', {class: 'num'}, a.total.toLocaleString()),
                h('td', {class: 'muted'}, a.held_in.map((i) => `${invLabel(i.inv)} ${i.count.toLocaleString()}`).join(' · ')),
                h('td', {class: 'muted', title: fullTime(a.saved_ms)}, ago(a.saved_ms)),
                h('td', {class: 'actions'},
                    link(economyHref('trace', {name: a.name, obj: r.item.id}), {class: 'btn small ghost'}, icon('search'), 'Trace'))));
            replace(result, h('section', {class: 'card'},
                h('div', {class: 'card-head'},
                    h('h2', {}, r.item.name),
                    h('span', {class: 'hint'}, `${r.total.toLocaleString()} in circulation, in ${plural(r.holders, 'save')}${r.item.forms.length > 1 ? ', noted ones counted in' : ''}`)),
                rows.length
                    ? tableWindow(h('table', {class: 'table holders'},
                        h('thead', {}, h('tr', {}, h('th', {}, 'Player'), h('th', {}, 'Holds'), h('th', {}, 'Where'), h('th', {}, 'Saved'), h('th', {}))),
                        h('tbody', {}, rows)))
                    : h('div', {class: 'empty'}, min > 1 ? `Nobody holds ${min.toLocaleString()} or more.` : 'Nobody holds any.'),
                h('div', {class: 'card-foot hint'}, 'As of each account\'s last save: what someone online picked up since isn\'t in yet.')));
        }).catch((e) => replace(result, h('div', {class: 'empty'}, e.message)));
    }
    return h('div', {class: 'column'},
        h('section', {class: 'card'}, h('div', {class: 'card-body form-row'},
            h('label', {class: 'field grow'}, h('span', {}, 'Item'), picker.search, picker.results),
            h('label', {class: 'field'}, h('span', {}, 'At least'), minInput))),
        obj ? result : h('div', {class: 'empty'}, icon('users'), 'Pick an item to see every account holding it, the most first.'));
}

// The open cases on any of `names`, to pick the one a ring action is for.
function ringCasePicker(names, caseId) {
    const select = h('select', {class: 'input', required: true, 'aria-label': 'Case'});
    const fill = async (list) => {
        replace(select, h('option', {value: ''}, 'Loading their cases...'));
        try {
            const found = new Map();
            for (const rows of await Promise.all(list.slice(0, 25).map((player) => api('GET', `/cases?${query({player, status: 'open'})}`)))) {
                for (const c of rows) found.set(c.id, c);
            }
            if (caseId && !found.has(caseId)) found.set(caseId, {id: caseId, title: ''});
            const cases = [...found.values()].sort((a, b) => b.id - a.id);
            replace(select, h('option', {value: ''}, cases.length ? 'Pick a case' : 'No open cases on them'),
                cases.map((c) => h('option', {value: String(c.id)}, `#${c.id} ${c.title}`)));
            if (caseId) select.value = String(caseId);
            else if (cases.length === 1) select.value = String(cases[0].id);
        } catch (e) {
            replace(select, h('option', {value: ''}, e.message));
        }
    };
    fill(names);
    return {
        field: h('label', {class: 'field'}, h('span', {}, 'Case'), select,
            h('span', {class: 'hint'}, 'Needed, and each account\'s rollback is noted in its timeline.')),
        value: () => (select.value ? Number(select.value) : null),
        refill: fill,
    };
}

// Rolls every account of a dupe ring back to its newest save from before
// one moment, as one action.
function ringRollbackDialog({names = [], atMs = null, caseId = null} = {}, onDone) {
    const namesInput = h('textarea', {
        class: 'input',
        rows: '2',
        required: true,
        placeholder: 'Account names, separated by commas',
        'aria-label': 'Accounts',
    });
    namesInput.value = names.join(', ');
    const when = h('input', {class: 'input', type: 'datetime-local', required: true, 'aria-label': 'Back to'});
    if (atMs) when.value = localInput(atMs);
    const preview = h('div', {class: 'list ring-preview'}, h('p', {class: 'hint'}, 'Check shows the save each account goes back to.'));
    const why = reasonField(true, 'e.g. Coins duped through the trade screen bug; back to before the first dupe.');
    const picked = ringCasePicker(names, caseId);
    const parse = () => [...new Set(namesInput.value.split(/[,\n]/).map((n) => n.trim()).filter(Boolean))];
    let checkedFor = null;
    let ready = false;
    const check = async () => {
        const list = parse();
        const at = inputTime(when.value);
        if (list.length < 2) throw new Error('A ring is at least 2 accounts.');
        if (Number.isNaN(at)) throw new Error('Pick the moment to go back to.');
        const members = await api('GET', `/economy/ring?${query({names: list.join(','), at_ms: at})}`);
        checkedFor = `${list.join(',')}@${at}`;
        ready = members.every((m) => !m.problem);
        replace(preview, members.map((m) => h('div', {class: 'list-item'},
            h('div', {class: `list-icon ${m.problem ? 'danger' : ''}`}, icon(m.problem ? 'alert' : 'undo')),
            h('div', {class: 'list-main'},
                h('div', {class: 'list-title'}, playerLink(m.name), ' ', m.world && h('span', {class: 'badge ok'}, `W${m.world}`)),
                h('div', {class: 'list-sub'}, m.problem
                    ? h('span', {class: 'error-text'}, m.problem)
                    : `To their save from ${fullTime(m.point.saved_ms)} (${(POINT_KINDS[m.point.kind] || [m.point.kind])[0].toLowerCase()})${m.world ? '; logged out first' : ''}`)))));
        picked.refill(list);
        return ready;
    };
    namesInput.addEventListener('input', () => (ready = false));
    when.addEventListener('input', () => (ready = false));
    openDialog({
        title: 'Roll back a ring',
        subtitle: 'Every account goes back to its newest save from before the same moment, in one action. Nothing is rolled back unless every account has one.',
        width: '640px',
        body: [
            h('label', {class: 'field'}, h('span', {}, 'Accounts'), namesInput),
            h('div', {class: 'form-row'},
                h('label', {class: 'field grow'}, h('span', {}, `Back to (${inUtc() ? 'UTC' : 'your time'})`), when),
                button('Check', {
                    icon: 'search',
                    on: {
                        click: async (e) => {
                            const btn = e.currentTarget;
                            btn.disabled = true;
                            try {
                                await check();
                            } catch (err) {
                                replace(preview, h('p', {class: 'error-text'}, err.message));
                            } finally {
                                btn.disabled = false;
                            }
                        },
                    },
                })),
            preview,
            h('div', {class: 'callout danger'}, icon('undo'), h('div', {},
                'Everything each account did since is undone. Their saves as they are now are kept in the history, so each can be undone on its own.')),
            why.field,
            picked.field,
        ],
        actions: [{label: 'Cancel'}, {
            label: 'Roll back the ring',
            kind: 'danger',
            icon: 'undo',
            run: async () => {
                const list = parse();
                const at = inputTime(when.value);
                if (!ready || checkedFor !== `${list.join(',')}@${at}`) {
                    if (!(await check())) throw new Error('Check the accounts: not every one can be rolled back to then.');
                    throw new Error('Here\'s who goes back to what. Roll back again to go ahead.');
                }
                if (!picked.value()) throw new Error('Pick the case this is for.');
                const results = await api('POST', '/economy/ring-rollback', {
                    names: list,
                    at_ms: at,
                    reason: why.value(),
                    case: picked.value(),
                });
                const failedOnes = results.filter((r) => !r.outcome.ok);
                if (failedOnes.length) {
                    openDialog({
                        title: 'Some rollbacks didn\'t take',
                        body: h('div', {class: 'list'}, results.map((r) => h('div', {class: 'list-item'},
                            h('div', {class: `list-icon ${r.outcome.ok ? 'ok' : 'danger'}`}, icon(r.outcome.ok ? 'check' : 'alert')),
                            h('div', {class: 'list-main'}, h('div', {class: 'list-title'}, r.name), h('div', {class: 'list-sub'}, r.outcome.message))))),
                        actions: [{label: 'Close'}],
                    });
                } else {
                    toast(`Rolled back ${plural(results.length, 'account')}.`);
                }
                onDone?.();
            },
        }],
    });
}

// A trace's steps as a tree: each holder's sources, and under one from
// another player, how that player came by it.
function traceTree(steps, parent) {
    const mine = steps.map((s, i) => [s, i]).filter(([s]) => (s.parent ?? null) === parent);
    if (!mine.length) return null;
    return h('div', {class: parent == null ? 'list trace' : 'trace-children'}, mine.map(([s, i]) => {
        const [what, iconName] = sourceLabel(s);
        const children = traceTree(steps, i);
        const handed = [TRANSFER.trade, TRANSFER.stake, TRANSFER.give].includes(s.event_type);
        return h('div', {class: 'trace-step'},
            h('div', {class: 'list-item'},
                h('div', {class: `list-icon ${handed ? 'warn' : ''}`}, icon(iconName)),
                h('div', {class: 'list-main'},
                    h('div', {class: 'list-title'},
                        s.depth > 0 && h('span', {class: 'muted'}, `${s.holder}: `),
                        what, s.from && [' ', playerLink(s.from)],
                        h('span', {class: 'num'}, ` · ${Number(s.count).toLocaleString()}`),
                        s.detail && h('span', {class: 'muted'}, ` from ${s.detail}`)),
                    h('div', {class: 'list-sub'}, joined([
                        coins(s.value),
                        worldOf(s.node),
                        where(s.x, s.z, s.level),
                    ]))),
                h('div', {class: 'list-side', title: fullTime(s.at_ms)}, shortTime(s.at_ms))),
            children && h('div', {class: 'trace-before hint'}, `Before that, how ${s.from} came by it:`),
            children);
    }));
}

function traceTab(ctx, params, go) {
    const name = params.get('name') || '';
    const obj = params.get('obj');
    const item = params.get('item');
    const depth = [2, 3, 4, 5, 6].includes(Number(params.get('depth'))) ? Number(params.get('depth')) : 4;
    const nameInput = h('input', {class: 'input', required: true, value: name, placeholder: 'Player', 'aria-label': 'Player', autocomplete: 'off', spellcheck: 'false'});
    nameSuggest(nameInput);
    const picker = objPicker({required: false, autofocus: false, placeholder: 'Item: name, config name or id'});
    const depthSelect = h('select', {class: 'input', 'aria-label': 'How far back'},
        [2, 3, 4, 5, 6].map((n) => h('option', {value: String(n), selected: n === depth}, `${n} holders back`)));
    const result = h('div', {class: 'column'});
    const run = () => {
        const picked = picker.chosen()?.id ?? obj;
        if (!validName(nameInput.value.trim())) return toast('Enter a player name.', 'error');
        if (picked == null) return toast('Pick an item from the results.', 'error');
        go('trace', {name: nameInput.value.trim(), obj: picked, depth: depthSelect.value});
    };
    nameInput.addEventListener('keydown', (e) => e.key === 'Enter' && run());
    // From a timeline row: the item by its config name.
    if (item && !obj) {
        api('GET', `/objs?${query({q: item})}`).then((objs) => {
            const found = objs.find((o) => o.debugname === item);
            if (found) {
                history.replaceState(null, '', economyHref('trace', {name, obj: found.id, depth}));
                renderPage();
            } else replace(result, h('div', {class: 'empty'}, `There's no item ${item}.`));
        }).catch((e) => replace(result, h('div', {class: 'empty'}, e.message)));
    }
    if (name && obj) {
        replace(result, h('section', {class: 'card'}, spinnerList()));
        api('GET', `/economy/trace?${query({name, obj, depth})}`).then((t) => {
            if (!t) return replace(result, h('div', {class: 'empty'}, 'There\'s no such item.'));
            picker.search.value = t.item.name;
            const ring = [...new Set([name, ...t.steps.map((s) => s.from).filter(Boolean)])];
            const handovers = t.steps.filter((s) => s.from);
            const earliest = handovers.length ? Math.min(...handovers.map((s) => s.at_ms)) : null;
            replace(result, h('section', {class: 'card'},
                h('div', {class: 'card-head'},
                    h('h2', {}, `Where ${name}'s ${t.item.name} came from`),
                    h('div', {class: 'actions'},
                        link(economyHref('trades', {name}), {class: 'btn small ghost'}, icon('users'), 'Trade graph'),
                        state.canCommand && ring.length > 1 && button('Roll back these accounts', {
                            small: true,
                            icon: 'undo',
                            title: 'Everyone it passed through, back to before the first handover',
                            on: {click: () => ringRollbackDialog({names: ring, atMs: earliest ? earliest - 60_000 : null})},
                        }))),
                t.steps.length
                    ? traceTree(t.steps, null)
                    : h('div', {class: 'empty'}, icon('search'), `Nothing logged shows how ${name} came by ${t.item.name}. Items from before the logs began, and drops no player made (an NPC's, or a spawn), don't show.`),
                h('div', {class: 'card-foot hint'}, 'Newest first at each step. A pickup is matched to the latest drop of the item on that tile; each player is followed back once.')));
        }).catch((e) => replace(result, h('div', {class: 'empty'}, e.message)));
    }
    return h('div', {class: 'column'},
        h('section', {class: 'card'}, h('div', {class: 'card-body form-row'},
            h('label', {class: 'field'}, h('span', {}, 'Player'), nameInput),
            h('label', {class: 'field grow'}, h('span', {}, 'Item'), picker.search, picker.results),
            h('label', {class: 'field'}, h('span', {}, 'How far back'), depthSelect),
            button('Trace', {kind: 'primary', icon: 'search', on: {click: run}}))),
        name && (obj || item) ? result : h('div', {class: 'empty'}, icon('search'),
            'Follow a stack back through the trades, stakes, drops and pickups it came by: the tool to reach for first when a dupe happens.'));
}

// Partners around a player, with arrows for what's handed each way:
// thicker for more value, amber between two partners (a ring, maybe).
function tradeGraphSvg(graph, onPick) {
    const W = 640;
    const H = 420;
    const [centre, ...partners] = graph.accounts;
    const pos = new Map([[centre.name, [W / 2, H / 2]]]);
    const r = Math.min(W, H) / 2 - 48;
    partners.forEach((a, i) => {
        const angle = -Math.PI / 2 + (2 * Math.PI * i) / Math.max(1, partners.length);
        pos.set(a.name, [W / 2 + r * Math.cos(angle) * 1.25, H / 2 + r * Math.sin(angle)]);
    });
    const top = Math.max(1, ...graph.edges.map((e) => e.value));
    const svg = document.createElementNS(SVG, 'svg');
    svg.setAttribute('viewBox', `0 0 ${W} ${H}`);
    svg.setAttribute('class', 'trade-graph');
    svg.setAttribute('role', 'img');
    svg.setAttribute('aria-label', `${centre.name} and ${plural(partners.length, 'trading partner')}`);
    const make = (tag, attrs, parent = svg) => {
        const el = document.createElementNS(SVG, tag);
        for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
        parent.append(el);
        return el;
    };
    const defs = make('defs', {});
    for (const kind of ['main', 'ring']) {
        const marker = make('marker', {
            id: `arrow-${kind}`,
            viewBox: '0 0 10 10',
            refX: '9',
            refY: '5',
            markerUnits: 'userSpaceOnUse',
            markerWidth: '9',
            markerHeight: '9',
            orient: 'auto-start-reverse',
        }, defs);
        make('path', {d: 'M0 0L10 5L0 10z', class: `arrow ${kind}`}, marker);
    }
    const both = new Set(graph.edges.map((e) => `${e.from}\n${e.to}`));
    for (const e of graph.edges) {
        const a = pos.get(e.from);
        const b = pos.get(e.to);
        if (!a || !b || (a[0] === b[0] && a[1] === b[1])) continue;
        const ring = e.from !== centre.name && e.to !== centre.name;
        // Each way bows to its own side when trades go both ways.
        const bow = both.has(`${e.to}\n${e.from}`) ? 18 : 0;
        const [mx, my] = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
        const len = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
        const [nx, ny] = [-(b[1] - a[1]) / len, (b[0] - a[0]) / len];
        const c = [mx + nx * bow, my + ny * bow];
        // Stop short of the node.
        const back = 12 / Math.hypot(b[0] - c[0], b[1] - c[1]);
        const end = [b[0] - (b[0] - c[0]) * back, b[1] - (b[1] - c[1]) * back];
        const path = make('path', {
            d: `M${a[0].toFixed(1)} ${a[1].toFixed(1)} Q${c[0].toFixed(1)} ${c[1].toFixed(1)} ${end[0].toFixed(1)} ${end[1].toFixed(1)}`,
            class: `edge ${ring ? 'ring' : ''}`,
            'stroke-width': (1 + 4 * Math.log1p(e.value) / Math.log1p(top)).toFixed(2),
            'marker-end': `url(#arrow-${ring ? 'ring' : 'main'})`,
        });
        make('title', {}, path).textContent = `${e.from} to ${e.to}: ${coins(e.value)} in ${plural(e.handovers, 'handover')}\n${e.items.map((i) => `${i.count.toLocaleString()} x ${i.item}`).join('\n')}`;
    }
    for (const a of graph.accounts) {
        const [x, y] = pos.get(a.name);
        const g = make('g', {class: `node ${a === centre ? 'centre' : ''} ${a.banned_until_ms ? 'banned' : ''} ${a.staff_level > 0 ? 'staff' : ''}`, tabindex: '0', role: 'button', 'aria-label': a.name});
        make('circle', {cx: x, cy: y, r: a === centre ? 9 : 6}, g);
        // Partners' names on the side away from the centre, clear of the edges.
        const above = a !== centre && y < H / 2 - 1;
        const label = make('text', {x, y: above ? y - 12 : y + (a === centre ? 24 : 19), 'text-anchor': 'middle'}, g);
        label.textContent = a.name;
        if (a !== centre) {
            const go = () => onPick(a.name);
            g.addEventListener('click', go);
            g.addEventListener('keydown', (e) => (e.key === 'Enter' || e.key === ' ') && go());
        }
    }
    return svg;
}

function receiversCard(params, go) {
    const days = [1, 7, 30].includes(Number(params.get('days'))) ? Number(params.get('days')) : 7;
    const givers = [2, 3, 5, 10].includes(Number(params.get('givers'))) ? Number(params.get('givers')) : 3;
    const body = spinnerList();
    api('GET', `/economy/receivers?${query({days, min_givers: givers})}`).then((rows) => {
        replace(body, rows.length
            ? tableWindow(h('table', {class: 'table receivers'},
                h('thead', {}, h('tr', {}, h('th', {}, 'Account'), h('th', {}, 'Givers'), h('th', {}, 'Received'), h('th', {}, 'Gave'), h('th', {}, 'Last'), h('th', {}))),
                h('tbody', {}, rows.map((r) => {
                    const oneWay = r.received > 0 && r.given * 10 < r.received;
                    return h('tr', {},
                        h('td', {}, h('div', {class: 'name-cell'}, playerLink(r.name), holderBadges(r),
                            oneWay && h('span', {class: 'badge warn', title: 'Takes in more than ten times what they give back'}, 'One way'))),
                        h('td', {class: 'num'}, r.givers.toLocaleString()),
                        h('td', {class: 'num'}, coins(r.received)),
                        h('td', {class: 'num muted'}, coins(r.given)),
                        h('td', {class: 'muted', title: fullTime(r.last_ms)}, ago(r.last_ms)),
                        h('td', {class: 'actions'}, link(economyHref('trades', {name: r.name, days: Math.max(days, 7)}), {class: 'btn small ghost'}, icon('users'), 'Graph')));
                }))))
            : h('div', {class: 'empty'}, `Nobody was handed items by ${givers} or more players in this time.`));
    }).catch((e) => failed(body, e));
    const chip = (key, value, current, label) => h('button', {
        type: 'button',
        class: 'chip',
        'aria-pressed': String(value === current),
        on: {click: () => value !== current && go('trades', {days: key === 'days' ? value : days, givers: key === 'givers' ? value : givers})},
    }, label);
    return h('section', {class: 'card'},
        h('div', {class: 'card-head'}, h('h2', {}, 'Accounts many players hand items to'),
            h('span', {class: 'hint hide-mobile'}, 'One account fed by many that gives little back is the usual sign of real-world trading')),
        h('div', {class: 'card-body'}, h('div', {class: 'chips'},
            [1, 7, 30].map((n) => chip('days', n, days, n === 1 ? 'Day' : `${n} days`)),
            h('span', {class: 'sep'}),
            [2, 3, 5, 10].map((n) => chip('givers', n, givers, `${n}+ givers`)))),
        body);
}

function tradesTab(ctx, params, go) {
    const name = params.get('name') || '';
    const days = [7, 30, 90, 365].includes(Number(params.get('days'))) ? Number(params.get('days')) : 30;
    const nameInput = h('input', {class: 'input', value: name, placeholder: 'Player', 'aria-label': 'Player', autocomplete: 'off', spellcheck: 'false'});
    nameSuggest(nameInput, () => show());
    const daysSelect = h('select', {class: 'input', 'aria-label': 'Over'},
        [7, 30, 90, 365].map((n) => h('option', {value: String(n), selected: n === days}, n === 365 ? 'The last year' : `The last ${n} days`)));
    const show = () => {
        const n = nameInput.value.trim();
        if (n && !validName(n)) return toast('That isn\'t a valid player name.', 'error');
        go('trades', {name: n, days: daysSelect.value});
    };
    nameInput.addEventListener('keydown', (e) => e.key === 'Enter' && show());
    daysSelect.addEventListener('change', show);
    const form = h('section', {class: 'card'}, h('div', {class: 'card-body form-row'},
        h('label', {class: 'field grow'}, h('span', {}, 'Player'), nameInput),
        h('label', {class: 'field'}, h('span', {}, 'Over'), daysSelect),
        button('Show', {kind: 'primary', icon: 'users', on: {click: show}})));
    if (!name) return h('div', {class: 'column'}, form, receiversCard(params, go));
    const result = h('section', {class: 'card'}, spinnerList());
    api('GET', `/economy/trades?${query({name, days})}`).then((graph) => {
        if (!graph.accounts.length) return replace(result, h('div', {class: 'empty'}, `No player named ${name} is registered.`));
        if (!graph.edges.length) return replace(result, h('div', {class: 'empty'}, icon('users'), `${name} handed nobody anything, and was handed nothing, in this time.`));
        const centre = graph.accounts[0].name;
        const net = (other) => {
            const to = graph.edges.find((e) => e.from === centre && e.to === other)?.value || 0;
            const from = graph.edges.find((e) => e.from === other && e.to === centre)?.value || 0;
            return from - to;
        };
        const ring = graph.accounts.map((a) => a.name);
        const rows = graph.edges.map((e) => h('tr', {},
            h('td', {}, playerLink(e.from), h('span', {class: 'muted'}, ' to '), playerLink(e.to)),
            h('td', {class: 'num'}, coins(e.value)),
            h('td', {class: 'num'}, e.handovers.toLocaleString()),
            h('td', {class: 'muted'}, e.items.map((i) => `${i.count.toLocaleString()} x ${i.item}`).join(', ')),
            h('td', {class: 'muted', title: fullTime(e.last_ms)}, ago(e.last_ms))));
        replace(result,
            h('div', {class: 'card-head'},
                h('h2', {}, `Who ${centre} trades with`),
                h('span', {class: 'hint hide-mobile'}, 'Click someone to centre on them'),
                h('div', {class: 'actions'},
                    state.canCommand && graph.accounts.length > 1 && button('Roll back a ring', {
                        small: true,
                        icon: 'undo',
                        title: 'Pick which of these to roll back, and to when',
                        on: {click: () => ringRollbackDialog({names: ring})},
                    }))),
            h('div', {class: 'card-body'}, tradeGraphSvg(graph, (other) => go('trades', {name: other, days}))),
            h('div', {class: 'card-body'}, h('div', {class: 'chips'}, graph.accounts.slice(1).map((a) => {
                const n = net(a.name);
                return n ? h('span', {class: `badge ${n > 0 ? 'ok' : 'warn'}`, title: n > 0 ? `${centre} got more from them than they gave` : `${centre} gave them more than they got back`},
                    `${a.name} ${n > 0 ? '+' : '-'}${compact(Math.abs(n))} gp`) : null;
            }))),
            tableWindow(h('table', {class: 'table edges'},
                h('thead', {}, h('tr', {}, h('th', {}, 'Handed'), h('th', {}, 'Worth'), h('th', {}, 'Times'), h('th', {}, 'Items'), h('th', {}, 'Last'))),
                h('tbody', {}, rows))),
            h('div', {class: 'card-foot hint'}, 'Trades, duel stakes and other handovers, worth the items\' values in the cache (rares are worth little there; see the items). Between partners too, which is how rings show.'));
    }).catch((e) => replace(result, h('div', {class: 'empty'}, e.message)));
    return h('div', {class: 'column'}, form, result);
}

function hiscoresChangeDialog(name, standing, mode, onDone) {
    const why = reasonField(true, mode === 'tables' ? 'e.g. Botted fishing; the XP is not earned.' : 'e.g. A test account.');
    const picked = casePicker(name, null, false);
    const boxes = [];
    const tables = mode === 'tables' ? h('div', {class: 'chips wrap'}, HISCORE_TABLES
        .filter(([id]) => standing.rows.some((r) => r.category === id) || standing.removed.includes(id))
        .map(([id, label]) => {
            const box = h('input', {type: 'checkbox', checked: standing.removed.includes(id)});
            boxes.push([id, box]);
            return h('label', {class: 'chip check'}, box, label);
        })) : null;
    openDialog({
        title: mode === 'hide' ? `Hide ${name} from the hiscores` : mode === 'show' ? `Show ${name} on the hiscores` : `Hiscore entries: ${name}`,
        subtitle: mode === 'tables'
            ? 'Ticked tables leave their entry out. Their saves still update it, so putting it back is instant.'
            : mode === 'hide' ? 'Kept off every table until shown again.' : 'Back on every table they aren\'t removed from.',
        body: [tables, why.field, picked.field],
        actions: [{label: 'Cancel'}, {
            label: mode === 'hide' ? 'Hide' : mode === 'show' ? 'Show' : 'Save',
            kind: mode === 'hide' ? 'danger' : 'primary',
            icon: 'check',
            run: async () => {
                const body = {name, reason: why.value(), case: picked.value(), remove: [], restore: []};
                if (mode === 'hide') body.hidden = true;
                else if (mode === 'show') body.hidden = false;
                else {
                    for (const [id, box] of boxes) {
                        const was = standing.removed.includes(id);
                        if (box.checked && !was) body.remove.push(id);
                        if (!box.checked && was) body.restore.push(id);
                    }
                    if (!body.remove.length && !body.restore.length) throw new Error('Nothing changed.');
                }
                const result = await api('POST', '/hiscores/change', body);
                if (!result.ok) throw new Error(result.message);
                toast(result.message);
                onDone();
            },
        }],
    });
}

// A player's hiscore entries, and keeping them off the tables.
function hiscoresCard(name, extraActions) {
    const body = h('div', {class: 'card-body'}, h('div', {class: 'spinner'}));
    const actions = h('div', {class: 'actions'}, extraActions);
    const load = () => api('GET', `/hiscores/player?${query({name})}`).then((s) => {
        if (!s) return replace(body, h('div', {class: 'empty'}, `No player named ${name} is registered.`));
        // Player moderators are ranked like players; staff above aren't.
        const ranks = s.staff_level < 2;
        const why = !ranks ? 'Jagex moderators and maintainers are never ranked.'
            : s.banned ? 'Not ranked while banned.'
                : s.hidden ? 'Hidden from every table by staff.' : null;
        replace(actions, extraActions,
            state.canCommand && ranks && button(s.hidden ? 'Show on the hiscores' : 'Hide from the hiscores', {
                small: true,
                icon: s.hidden ? 'eye' : 'ban',
                on: {click: () => hiscoresChangeDialog(name, s, s.hidden ? 'show' : 'hide', load)},
            }),
            state.canCommand && ranks && !s.hidden && s.rows.length > 0 && button('Remove entries', {
                small: true,
                icon: 'trash',
                title: 'Take a cheater off some tables',
                on: {click: () => hiscoresChangeDialog(name, s, 'tables', load)},
            }));
        replace(body,
            why && h('div', {class: 'callout'}, icon('lock'), h('div', {}, why)),
            s.rows.length
                ? tableWindow(h('table', {class: 'table hiscores'},
                    h('thead', {}, h('tr', {}, h('th', {}, 'Table'), h('th', {}, 'Level'), h('th', {}, 'XP'), h('th', {}, 'Rank'))),
                    h('tbody', {}, s.rows.map((r) => h('tr', {},
                        h('td', {}, tableName(r.category)),
                        h('td', {class: 'num'}, r.level.toLocaleString()),
                        h('td', {class: 'num'}, r.xp.toLocaleString()),
                        h('td', {class: 'num'}, r.rank != null ? `#${r.rank.toLocaleString()}`
                            : h('span', {class: `badge ${s.removed.includes(r.category) ? 'danger' : ''}`}, s.removed.includes(r.category) ? 'Removed' : 'Not ranked')))))))
                : h('div', {class: 'empty'}, 'Nothing trained yet.'));
    }).catch((e) => failed(body, e));
    load();
    return h('section', {class: 'card'}, h('div', {class: 'card-head'}, h('h2', {}, 'Hiscores'), actions), body);
}

function hiscoresTab(ctx, params, go) {
    const name = params.get('name') || '';
    const nameInput = h('input', {class: 'input', value: name, placeholder: 'Player', 'aria-label': 'Player', autocomplete: 'off', spellcheck: 'false'});
    nameSuggest(nameInput, () => look());
    const look = () => {
        const n = nameInput.value.trim();
        if (!validName(n)) return toast('That isn\'t a valid player name.', 'error');
        go('hiscores', {name: n});
    };
    nameInput.addEventListener('keydown', (e) => e.key === 'Enter' && look());
    const kept = spinnerList();
    api('GET', '/hiscores/kept-off').then((rows) => replace(kept, rows.length
        ? rows.map((k) => h('div', {class: 'list-item'},
            h('div', {class: 'list-icon'}, icon(k.hidden ? 'ban' : 'trash')),
            h('div', {class: 'list-main'},
                h('div', {class: 'list-title'}, playerLink(k.name)),
                h('div', {class: 'list-sub'}, joined([
                    k.hidden && 'Hidden from every table',
                    k.removed.length > 0 && `Removed from ${k.removed.map(tableName).join(', ')}`,
                ]))),
            link(economyHref('hiscores', {name: k.name}), {class: 'btn small ghost'}, 'Change', icon('chevron'))))
        : h('div', {class: 'empty'}, 'Staff haven\'t kept anyone off.'))).catch((e) => failed(kept, e));
    return h('div', {class: 'column'},
        h('section', {class: 'card'}, h('div', {class: 'card-body form-row'},
            h('label', {class: 'field grow'}, h('span', {}, 'Player'), nameInput),
            button('Look up', {kind: 'primary', icon: 'search', on: {click: look}}))),
        name && hiscoresCard(name, link(`/player?${query({name})}`, {class: 'btn small ghost'}, 'Player page', icon('chevron'))),
        h('section', {class: 'card'},
            h('div', {class: 'card-head'}, h('h2', {}, 'Kept off by staff'),
                h('span', {class: 'hint hide-mobile'}, 'Staff (any level) and banned players are left off on their own')),
            h('div', {class: 'list'}, kept)));
}

// The player page's economy card: their hiscores, with the trade tools.
function economySection(name) {
    return hiscoresCard(name, [
        link(economyHref('trades', {name}), {class: 'btn small ghost'}, icon('users'), 'Trade graph'),
        link(economyHref('trace', {name}), {class: 'btn small ghost'}, icon('search'), 'Trace an item'),
    ]);
}

// Recompiles one world's scripts and swaps them in: for an emergency fix.
function reloadScriptsDialog(world, onDone) {
    const why = reasonField(true, 'e.g. Fixes the bank note dupe in bank.rs2.');
    const report = h('div');
    const show = (r) => replace(report, h('div', {class: `callout ${r.ok ? '' : 'danger'}`}, icon(r.ok ? 'check' : 'alert'),
        h('div', {}, r.message, r.compile_ms ? h('span', {class: 'muted'}, ` (${(r.compile_ms / 1000).toFixed(1)}s)`) : null)),
    r.errors && h('pre', {class: 'compile-errors'}, r.errors));
    const send = async (check) => {
        const r = await api('POST', `/worlds/${world.node}/reload-scripts`, {check_only: check, reason: why.value()});
        show(r);
        return r;
    };
    openDialog({
        title: `Reload scripts: ${worldName(world)}`,
        subtitle: 'Recompiles the scripts in this world\'s content folder and swaps them in, without a restart. Recorded in the audit log.',
        width: '640px',
        body: [
            h('div', {class: 'callout'}, icon('alert'), h('div', {},
                'Scripts only: the cache stays as it is, so config, map and interface changes need a restart. Scripts already running finish on the old code. Check first: nothing changes if they don\'t compile.')),
            why.field,
            report,
        ],
        actions: [{label: 'Close'}, {
            label: 'Check',
            icon: 'search',
            run: async () => {
                await send(true);
                return true;
            },
        }, {
            label: 'Reload',
            kind: 'danger',
            icon: 'refresh',
            run: async () => {
                const r = await send(false);
                if (r.applied) {
                    toast(r.message);
                    onDone?.();
                    return false;
                }
                return true;
            },
        }],
    });
}

// ---------------------------------------------------------------------------
// Staff accounts (maintainers)

const ROLES = [['trial', 'Trial'], ['moderator', 'Moderator'], ['admin', 'Admin'], ['maintainer', 'Maintainer']];
const ROLE_NAMES = {trial: 'trial moderator', moderator: 'moderator', admin: 'admin', maintainer: 'maintainer'};
const ROLE_HINTS = {
    trial: 'A moderator in training: kicks, and mutes of up to 2 days. No bans, lifting sanctions, bulk dismissing or deciding appeals.',
    moderator: 'Kicks, mutes and bans, looks players up and reads chat logs.',
    admin: 'Also reboots, broadcasts and toggles maintenance.',
    maintainer: 'Everything, including managing these accounts.',
};

// The acting maintainer's password and code, asked for with every change.
function confirmFields() {
    const password = h('input', {
        class: 'input',
        type: 'password',
        autocomplete: 'current-password',
        required: true,
        'aria-label': 'Your password'
    });
    const code = h('input', {
        class: 'input',
        inputmode: 'numeric',
        autocomplete: 'one-time-code',
        maxlength: '6',
        pattern: '[0-9]{6}',
        required: true,
        placeholder: '000000',
        'aria-label': 'Your authenticator code',
        'data-invalid': 'Authenticator codes are 6 digits.'
    });
    code.addEventListener('input', () => {
        code.value = code.value.replace(/\D/g, '').slice(0, 6);
    });
    return {
        el: h('div', {class: 'confirm-box'},
            h('p', {class: 'hint'}, 'Confirm it\'s you'),
            h('div', {class: 'confirm-grid'},
                h('label', {class: 'field'}, h('span', {}, 'Your password'), password),
                h('label', {class: 'field'}, h('span', {}, 'Authenticator code'), code))),
        value: () => ({password: password.value, code: code.value}),
    };
}

function roleChips(initial, onChange) {
    let role = initial;
    const chips = h('div', {class: 'segmented'});
    const hint = h('p', {class: 'hint'});
    const draw = () => {
        replace(chips, ROLES.map(([key, label]) => h('button', {
            type: 'button',
            'aria-pressed': String(role === key),
            on: {
                click: () => {
                    role = key;
                    draw();
                    onChange?.(role);
                }
            }
        }, label)));
        hint.textContent = ROLE_HINTS[role];
    };
    draw();
    return {el: [chips, hint], value: () => role};
}

function addStaffDialog(onDone) {
    const name = h('input', {
        class: 'input',
        maxlength: '12',
        pattern: NAME_PATTERN,
        'data-invalid': NAME_RULE,
        required: true,
        autocomplete: 'off',
        spellcheck: 'false',
        autofocus: true,
        placeholder: 'Their in-game name'
    });
    nameSuggest(name);
    const password = h('input', {
        class: 'input',
        type: 'password',
        autocomplete: 'new-password',
        minlength: '12',
        required: true,
        'data-invalid': 'Passwords are at least 12 characters.',
        placeholder: 'At least 12 characters'
    });
    const role = roleChips('moderator');
    const confirm = confirmFields();
    openDialog({
        title: 'Add a staff account',
        subtitle: 'They set up their authenticator app at their first sign-in.',
        body: [
            h('label', {class: 'field'}, h('span', {}, 'Name'), name, h('span', {class: 'hint'}, 'Their in-game name. They must already be staff (level 1+) in the game.')),
            h('div', {class: 'field'}, h('span', {}, 'Role'), role.el),
            h('label', {class: 'field'}, h('span', {}, 'First password'), password, h('span', {class: 'hint'}, 'Give it to them privately. They can change it under Account.')),
            confirm.el,
        ],
        actions: [{label: 'Cancel'}, {
            label: 'Add account',
            kind: 'primary',
            icon: 'plus',
            run: async () => {
                await api('POST', '/staff', {
                    name: name.value.trim(),
                    role: role.value(),
                    password: password.value,
                    confirm: confirm.value()
                });
                toast(`Added ${name.value.trim()}.`);
                onDone();
            },
        }],
    });
}

function staffChangeDialog(account, kind, onDone) {
    const confirm = confirmFields();
    const role = roleChips(account.role);
    const password = h('input', {
        class: 'input',
        type: 'password',
        autocomplete: 'new-password',
        minlength: '12',
        required: true,
        'data-invalid': 'Passwords are at least 12 characters.',
        placeholder: 'At least 12 characters'
    });
    const specs = {
        role: ['Change role', 'primary', 'key', [h('div', {class: 'field'}, h('span', {}, 'Role'), role.el)], () => ({
            kind: 'role',
            role: role.value()
        })],
        password: ['Set password', 'primary', 'lock', [h('label', {class: 'field'}, h('span', {}, 'New password'), password, h('span', {class: 'hint'}, 'Give it to them privately.'))], () => ({
            kind: 'password',
            password: password.value
        })],
        reset_2fa: ['Reset 2FA', 'primary', 'shield', [h('p', {class: 'muted'}, 'They set up a new authenticator app at their next sign-in. Use this when they lose their phone.')], () => ({kind: 'reset2fa'})],
        disable: ['Disable', 'danger', 'lock', [h('p', {class: 'muted'}, 'They can\'t sign in until a maintainer enables the account again.')], () => ({kind: 'disable'})],
        enable: ['Enable', 'primary', 'unlock', [h('p', {class: 'muted'}, 'They can sign in again.')], () => ({kind: 'enable'})],
        sign_out: ['Sign out everywhere', 'primary', 'logout', [h('p', {class: 'muted'}, 'Ends every session they have open. Nothing else about the account changes.')], () => ({kind: 'sign_out'})],
        remove: ['Remove account', 'danger', 'x', [h('div', {class: 'callout danger'}, icon('alert'), h('div', {}, 'The account is deleted. Its audit log entries stay.'))], () => ({kind: 'remove'})],
    };
    const [label, kindClass, iconName, body, change] = specs[kind];
    openDialog({
        title: `${label}: ${account.name}`,
        subtitle: 'Signs them out everywhere.',
        body: [...body, confirm.el],
        actions: [{label: 'Cancel'}, {
            label,
            kind: kindClass,
            icon: iconName,
            run: async () => {
                await api('POST', '/staff/update', {name: account.name, change: change(), confirm: confirm.value()});
                toast(`${label}: done for ${account.name}.`);
                onDone();
            },
        }],
    });
}

const staffLink = (name, props = {}) => link(`/staff/member?${query({name})}`, {class: 'name-link', ...props}, name);

const staffBadges = (a) => [
    h('span', {class: 'badge accent'}, a.role), ' ',
    a.you && h('span', {class: 'badge ok'}, 'You'), ' ',
    !a.two_factor && h('span', {class: 'badge warn'}, '2FA not set up'), ' ',
    a.disabled && h('span', {class: 'badge danger'}, 'Disabled')];

// The changes a maintainer can make to someone else's account.
function staffActions(a, onDone, {signOut = false} = {}) {
    if (a.you) return link('/account', {class: 'btn small ghost'}, 'Your account', icon('chevron'));
    const change = (kind) => () => staffChangeDialog(a, kind, () => onDone(kind));
    return h('div', {class: 'chips'},
        button('Role', {small: true, icon: 'key', on: {click: change('role')}}),
        button('Password', {small: true, icon: 'lock', on: {click: change('password')}}),
        a.two_factor && button('Reset 2FA', {small: true, icon: 'shield', on: {click: change('reset_2fa')}}),
        signOut && a.sessions > 0 && button('Sign out everywhere', {small: true, icon: 'logout', on: {click: change('sign_out')}}),
        a.disabled
            ? button('Enable', {small: true, icon: 'unlock', on: {click: change('enable')}})
            : button('Disable', {small: true, icon: 'lock', on: {click: change('disable')}}),
        button(null, {
            small: true,
            icon: 'x',
            'aria-label': `Remove ${a.name}`,
            title: 'Remove',
            on: {click: change('remove')}
        }));
}

function staffPage() {
    const list = h('div', {class: 'list'}, h('div', {class: 'empty'}, h('div', {class: 'spinner'})));
    const load = async () => {
        try {
            const accounts = await api('GET', '/staff');
            replace(list, accounts.map((a) => h('div', {class: 'list-item'},
                h('div', {class: 'avatar'}, initial(a.name)),
                h('div', {class: 'list-main'},
                    h('div', {class: 'list-title'}, h('b', {}, staffLink(a.name)), ' ', staffBadges(a)),
                    h('div', {class: 'list-sub'}, `${a.sessions ? plural(a.sessions, 'session') + ' open' : 'Signed out'} · added ${a.created_ms ? dateOnly(a.created_ms) : 'from the command line'}`)),
                staffActions(a, load))));
        } catch (e) {
            replace(list, h('div', {class: 'empty'}, e.message));
        }
    };
    load();
    return h('div', {class: 'page'},
        pageHead('Staff', 'Console accounts. Every change needs your password and authenticator code, and is audited.',
            button('Add account', {icon: 'plus', kind: 'primary', on: {click: () => addStaffDialog(load)}})),
        h('section', {class: 'card'}, list));
}

// One staff member, for maintainers: their account, where they're signed
// in, what they did and every audit entry that's theirs.

const MEMBER_TABS = [['audit', 'Audit log'], ['sanctions', 'Sanctions'], ['cases', 'Cases'], ['sign_ins', 'Sign-ins']];

function staffMemberPage(ctx) {
    const name = (ctx.params.get('name') || '').trim();
    let tab = MEMBER_TABS.some(([k]) => k === ctx.params.get('tab')) ? ctx.params.get('tab') : 'audit';
    let days = ACTIVITY_WINDOWS.some(([d]) => String(d) === ctx.params.get('days')) ? Number(ctx.params.get('days')) : 30;
    const head = h('div', {}, pageHead(name || 'Staff member', 'Loading...'));
    const chips = h('div', {class: 'chips'});
    const stats = h('div', {class: 'stats'});
    const sessions = h('div', {});
    const tabs = h('div', {class: 'segmented tabs', role: 'group', 'aria-label': 'View'});
    const body = h('div', {class: 'column'});
    const keepUrl = () => history.replaceState(null, '', `/staff/member?${query({name, tab: tab === 'audit' ? null : tab, days: days === 30 ? null : days})}`);
    const stat = (iconName, label, value, sub) => h('div', {class: 'card stat'},
        h('div', {class: 'label'}, icon(iconName), label), h('div', {class: 'value num'}, value), h('div', {class: 'sub'}, sub));
    const took = (ms) => (ms == null ? '-' : span(ms / 1000));

    const drawStats = (a) => {
        const rate = a.appealed ? Math.round(a.overturned / a.appealed * 100) : null;
        replace(stats,
            stat('shield', 'Sanctions', `${a.kicks} · ${a.mutes} · ${a.bans} · ${a.lifts}`, 'Kicks · mutes · bans · lifted, in the console'),
            stat('briefcase', 'Cases', `${a.cases_opened} / ${a.cases_closed}`,
                `Opened / closed · ${plural(a.comments, 'comment')} · ${a.dismissed} dismissed`),
            stat('undo', 'Overturned', a.appealed ? `${a.overturned} / ${a.appealed}` : '-',
                `${rate === null ? 'None of theirs appealed' : `${rate}% of their appealed sanctions`} · ${a.appeals_decided} decided`),
            stat('clock', 'Pickup', took(a.pickup_ms), `To close ${took(a.close_ms)} · to decide ${took(a.decide_ms)}`));
    };
    const drawSessions = (m) => replace(sessions, m.open_sessions.length
        ? m.open_sessions.map((s) => h('div', {class: 'session-row'},
            h('div', {class: 'list-icon'}, icon('monitor')),
            h('div', {class: 'list-main'},
                h('div', {class: 'list-title'}, h('b', {}, device(s.user_agent))),
                h('div', {class: 'list-sub'}, `${s.ip} · signed in ${ago(s.created_ms)} · active ${ago(s.last_seen_ms)}`))))
        : h('div', {class: 'empty'}, icon('monitor'), 'Not signed in anywhere.'));

    const load = async () => {
        keepUrl();
        replace(chips, ACTIVITY_WINDOWS.map(([d, label]) => h('button', {
            type: 'button',
            class: 'chip',
            'aria-pressed': String(days === d),
            on: {
                click: () => {
                    days = d;
                    load();
                }
            }
        }, label)));
        try {
            const m = await api('GET', `/staff/member?${query({name, days})}`);
            const active = m.activity?.last_active_ms;
            replace(head, pageHead(m.name,
                [`${ROLE_NAMES[m.role] || m.role} · added ${m.created_ms ? dateOnly(m.created_ms) : 'from the command line'} · `,
                    active ? h('span', {title: fullTime(active)}, `last moderation action ${ago(active)}`) : `no moderation actions in the last ${days} days`],
                link(`/player?${query({name: m.name})}`, {class: 'btn small ghost'}, 'Player page', icon('chevron'))),
            h('div', {class: 'toolbar'}, h('div', {class: 'chips'}, staffBadges(m)),
                staffActions(m, (kind) => (kind === 'remove' ? navigate('/staff') : load()), {signOut: true})));
            drawStats(m.activity || {
                kicks: 0, mutes: 0, bans: 0, lifts: 0, cases_opened: 0, cases_closed: 0, comments: 0,
                dismissed: 0, appeals_decided: 0, appealed: 0, overturned: 0
            });
            drawSessions(m);
        } catch (e) {
            replace(head, pageHead(name || 'Staff member', e.message));
        }
    };

    const auditTab = (q, title) => {
        const search = h('input', {
            class: 'input',
            type: 'search',
            placeholder: 'Search actions, players, details',
            'aria-label': `Search ${name}'s audit entries`,
            value: q || ''
        });
        const filter = () => query({actor: name, q: q ?? search.value.trim()});
        const paged = pagedList((before) => api('GET', `/audit?${query({limit: 50, before})}&${filter()}`), auditItem, 'No matching entries.');
        search.addEventListener('input', debounce(paged.reload, 250));
        return [
            q == null && h('div', {class: 'toolbar'}, h('div', {class: 'search'}, icon('search'), search),
                exportButton(() => `/export/audit?${filter()}`, 'Download every entry this search finds (up to 10,000) as a CSV file')),
            h('section', {class: 'card'}, title && h('div', {class: 'card-head'}, h('h2', {}, title)), paged.el)];
    };
    const drawTab = () => {
        keepUrl();
        replace(tabs, MEMBER_TABS.map(([k, label]) => h('button', {
            type: 'button', 'aria-pressed': String(tab === k), on: {
                click: () => {
                    tab = k;
                    drawTab();
                }
            }
        }, label)));
        if (tab === 'audit') {
            replace(body, auditTab(null));
        } else if (tab === 'sanctions') {
            const paged = pagedList((before) => api('GET', `/lookup/moderation?${query({moderator: name, before})}`),
                moderationItem, 'No moderation by them on record.');
            replace(body, h('section', {class: 'card'},
                h('div', {class: 'card-head'}, h('h2', {}, 'Every kick, mute and ban, in game or here, newest first')), paged.el));
        } else if (tab === 'cases') {
            const list = spinnerList();
            replace(body, h('section', {class: 'card'},
                h('div', {class: 'card-head'}, h('h2', {}, 'Cases they opened, closed or are on')), scrollWindow(list)));
            api('GET', `/cases?${query({staff: name, status: 'all'})}`).then((rows) => replace(list, rows.length
                ? rows.map((c) => caseItem(c))
                : h('div', {class: 'empty'}, icon('briefcase'), 'No cases.'))).catch((e) => failed(list, e));
        } else {
            replace(body,
                h('section', {class: 'card'}, h('div', {class: 'card-head'}, h('h2', {}, 'Signed in now')), sessions),
                auditTab('sign_in', 'Sign-ins and failed attempts'));
        }
    };

    const page = h('div', {class: 'page'}, head);
    if (!validName(name)) {
        replace(head, pageHead('Staff member', 'That isn\'t a valid name.'));
        return page;
    }
    load();
    drawTab();
    append(page, h('div', {class: 'toolbar'}, chips), stats, tabs, body);
    return page;
}

// ---------------------------------------------------------------------------
// A player's timeline: everything on record about them, every log merged,
// newest first.

const TIMELINE_KINDS = [
    ['login', 'Logins'],
    ['session', 'Activity'],
    ['wealth', 'Wealth'],
    ['chat', 'Chat'],
    ['moderation', 'Moderation'],
    ['reported', 'Reports against'],
    ['report', 'Reports sent'],
    ['replay', 'Sessions'],
];
// The content's ^log_* and ^wealth_* constants, then the engine's own
// (rs_db::log_type and rs_db::transfer).
const STAFF_COMMAND = 100;
const SESSION_TYPES = {0: 'Moderator log', 1: 'Adventure log', [STAFF_COMMAND]: 'Staff command'};
const WEALTH_TYPES = {
    5: 'Picked up',
    6: 'Bought from a shop',
    7: 'Sold to a shop',
    8: 'Low alchemy',
    9: 'High alchemy',
    10: 'Party room',
    100: 'Traded away',
    101: 'Lost as a duel stake',
    102: 'Handed over',
    103: 'Dropped',
    104: 'Picked up from the floor',
    105: 'Made with a staff command',
};
const coins = (n) => `${Number(n || 0).toLocaleString()} gp`;
const dayLabel = (ms) => new Date(ms).toLocaleDateString([], zoned({
    weekday: 'short',
    year: 'numeric',
    month: 'short',
    day: 'numeric'
}));
const timeOfDay = (ms) => new Date(ms).toLocaleTimeString([], zoned({hour: '2-digit', minute: '2-digit'}));
const joined = (parts) => parts.filter(Boolean).flatMap((p, i) => (i ? [' · ', p] : [p]));

function timelineItem(e, name) {
    let iconName = 'activity';
    let side = null;
    let tone = '';
    let title = e.text;
    let sub = [];
    switch (e.kind) {
        case 'login':
            iconName = 'key';
            title = 'Logged in';
            sub = [e.text && h('span', {class: 'mono'}, e.text),
                e.detail && (e.detail === 'desktop' ? 'Desktop client' : h('span', {title: e.detail}, device(e.detail)))];
            break;
        case 'session':
            if (e.event_type === STAFF_COMMAND) {
                iconName = 'key';
                title = h('span', {class: 'mono'}, e.text);
            }
            sub = [SESSION_TYPES[e.event_type] || `Log type ${e.event_type}`];
            break;
        case 'wealth': {
            iconName = 'zap';
            // A handover to them is logged under who gave it.
            const received = e.detail === 'received';
            const label = (received ? RECEIVED_TYPES : WEALTH_TYPES)[e.event_type] || `Wealth event ${e.event_type}`;
            title = [`${label}: ${Number(e.count).toLocaleString()} x ${e.text}`,
                e.other ? [h('span', {class: 'muted'}, received ? ' from ' : ' to '), playerLink(e.other)] : ''];
            sub = [coins(e.value)];
            const came = received || [5, 6, 10, TRANSFER.take].includes(e.event_type);
            side = h('div', {class: 'chips nowrap'},
                came && link(economyHref('trace', {name, item: e.text}), {
                    class: 'link-button',
                    title: 'Where it came from, back through the players it passed'
                }, icon('search'), 'Trace'),
                !received && state.canCommand && GAINED_WEALTH.includes(e.event_type) && h('button', {
                    type: 'button',
                    class: 'link-button',
                    title: 'Give it back, with this as the proof they had it',
                    on: {click: () => restoreFromWealth(name, e)}
                }, icon('plus'), 'Restore'));
            break;
        }
        case 'chat':
            iconName = 'chat';
            title = e.other ? [h('span', {class: 'muted'}, 'To '), playerLink(e.other), ': ', e.text] : e.text;
            sub = [e.other ? 'Private' : 'Public'];
            break;
        case 'moderation': {
            const [verb, iconFor, toneFor] = MOD_VERBS[e.text] || [e.text, 'shield', ''];
            iconName = iconFor;
            tone = toneFor;
            title = [h('b', {}, playerLink(e.other)), ` ${verb} them`, e.hours ? ` for ${hoursLabel(e.hours)}` : ''];
            sub = [e.detail];
            break;
        }
        case 'reported':
            iconName = 'flag';
            tone = 'warn';
            title = ['Reported by ', playerLink(e.other)];
            sub = [ruleName(e.event_type)];
            break;
        case 'report':
            iconName = 'flag';
            title = ['Reported ', playerLink(e.other)];
            sub = [ruleName(e.event_type)];
            break;
        case 'replay': {
            const flags = (e.text || '').split(',').filter(Boolean);
            iconName = flags.length ? 'alert' : 'play';
            tone = flags.length ? 'danger' : '';
            title = [link(`/replay?${query({id: e.id})}`, {class: 'name-link'}, 'Recorded session'), flags.length ? ' ' : '', flags.map(flagBadge)];
            sub = [clock((e.count || 0) / 1000), e.event_type ? `score ${e.event_type}` : null];
            break;
        }
        default:
            break;
    }
    const place = [e.world ? `World ${e.world}` : e.node != null ? `Node ${e.node}` : null, e.x != null ? where(e.x, e.z, e.level) : null];
    return h('div', {class: 'list-item'},
        h('div', {class: `list-icon ${tone}`}, icon(iconName)),
        h('div', {class: 'list-main'},
            h('div', {class: 'list-title'}, title),
            h('div', {class: 'list-sub'}, joined([...sub, ...place])),
            side),
        h('div', {class: 'list-side', title: fullTime(e.at_ms)}, timeOfDay(e.at_ms)));
}

function timelineCard(name) {
    const all = TIMELINE_KINDS.map(([k]) => k);
    const saved = prefs.get('timelineKinds', '').split(',').filter((k) => all.includes(k));
    const kinds = new Set(saved.length ? saved : all);
    const chips = h('div', {class: 'chips'});
    const list = h('div', {class: 'list'});
    const more = h('div', {class: 'more hidden'});
    // The records scroll in a box of their own, older ones loading as the
    // bottom comes into view, so a long history doesn't run down the page.
    const box = scrollWindow(list, more);
    box.classList.add('timeline-scroll');
    let cursor = null;
    let lastDay = null;
    let seq = 0;
    let fetching = false;

    const load = async (older) => {
        if (older && fetching) return;
        const mine = ++seq;
        if (!kinds.size) {
            replace(list, h('div', {class: 'empty'}, 'Pick at least one kind of record.'));
            more.classList.add('hidden');
            return;
        }
        if (!older) {
            cursor = null;
            lastDay = null;
            box.scrollTop = 0;
            more.classList.add('hidden');
            replace(list, h('div', {class: 'empty'}, h('div', {class: 'spinner'})));
        }
        fetching = true;
        try {
            const rows = await api('GET', `/lookup/timeline?${query({
                name,
                kinds: [...kinds].join(','),
                before_ms: older ? cursor.at_ms : null,
                before_kind: older ? cursor.kind : null,
                before_id: older ? cursor.id : null,
            })}`);
            if (mine !== seq) return;
            if (!older) list.replaceChildren();
            for (const e of rows) {
                const day = dayKey(e.at_ms);
                if (day !== lastDay) {
                    lastDay = day;
                    append(list, h('div', {class: 'list-day'}, dayLabel(e.at_ms)));
                }
                append(list, timelineItem(e, name));
            }
            if (!list.children.length) append(list, h('div', {class: 'empty'}, icon('list'), 'Nothing on record.'));
            const last = rows[rows.length - 1];
            if (last) cursor = {at_ms: last.at_ms, kind: last.kind, id: last.id};
            more.classList.toggle('hidden', rows.length < 100);
        } catch (e) {
            if (mine === seq) replace(list, h('div', {class: 'empty'}, e.message));
        } finally {
            if (mine === seq) fetching = false;
        }
    };
    loadOnScroll(box, more, () => load(true));

    const drawChips = () => {
        const every = kinds.size === all.length;
        replace(chips,
            h('button', {
                type: 'button', class: 'chip', 'aria-pressed': String(every), on: {
                    click: () => {
                        all.forEach((k) => kinds.add(k));
                        changed();
                    }
                }
            }, 'Everything'),
            TIMELINE_KINDS.map(([k, label]) => h('button', {
                type: 'button', class: 'chip', 'aria-pressed': String(!every && kinds.has(k)),
                on: {
                    click: () => {
                        // From "everything", a click picks just that kind.
                        if (every) {
                            kinds.clear();
                            kinds.add(k);
                        } else if (kinds.has(k)) kinds.delete(k); else kinds.add(k);
                        changed();
                    }
                },
            }, label)));
    };
    // Every kind is no filter at all.
    const filter = () => ({kinds: kinds.size === all.length ? '' : [...kinds].sort().join(',')});
    const changed = () => {
        prefs.set('timelineKinds', [...kinds].join(','));
        drawChips();
        savedBar.draw();
        load(false);
    };
    const savedBar = savedFilters('timeline', filter, (params) => {
        const picked = (params.kinds || '').split(',').filter((k) => all.includes(k));
        kinds.clear();
        (picked.length ? picked : all).forEach((k) => kinds.add(k));
        changed();
    });
    append(more, button('Load older', {small: true, on: {click: () => load(true)}}));
    drawChips();
    load(false);

    return h('section', {class: 'card'},
        h('div', {class: 'card-head'}, h('h2', {}, 'Timeline'),
            h('div', {class: 'actions'}, h('span', {class: 'hint hide-mobile'}, 'Every log, newest first: scroll for older'),
                exportButton(() => `/export/timeline?${query({name, kinds: [...kinds].join(',')})}`,
                    'Download the kinds shown (up to 5,000 rows) as a CSV file. Recorded in the audit log.'))),
        h('div', {class: 'card-body column timeline-filters'}, chips, savedBar.el),
        box);
}

// ---------------------------------------------------------------------------
// Linked accounts: where a player logs in from, and who else does.

function sanctionBadges(a) {
    return [
        a.banned_until_ms && h('span', {class: 'badge danger', title: `Ends ${fullTime(a.banned_until_ms)}`}, 'Banned'),
        a.muted_until_ms && h('span', {class: 'badge warn', title: `Ends ${fullTime(a.muted_until_ms)}`}, 'Muted'),
    ];
}

// User agent words only automation puts there (as the worlds check).
const AUTOMATION = /HeadlessChrome|PhantomJS/;
// Which platform each user agent word means, to catch hints that disagree.
const PLATFORM_WORDS = [['Windows', /Windows/], ['macOS', /Mac OS X|Macintosh/], ['Android', /Android/], ['iOS', /iPhone|iPad|iPod/], ['Linux', /Linux|X11/], ['Chrome OS', /CrOS/]];
// One browser used by this many accounts or more is a farm's, or a shared
// computer's.
const BUSY_DEVICE = 4;
// This many different browsers in a week means storage keeps being wiped,
// or many machines.
const DEVICE_CHURN = 5;

// What a player's devices, addresses and login ids give away: [tone, text].
function linkFlags(links) {
    const flags = [];
    const banned = links.banned || [];
    const evading = banned.filter((b) => b.after_ban);
    for (const via of ['browser', 'address']) {
        const names = evading.filter((b) => b.via === via).map((b) => b.name);
        if (names.length) flags.push(['danger', `Logged in from the ${via === 'browser' ? 'browser' : 'address'} of banned ${names.join(', ')} after the ban: possible ban evasion.`]);
    }
    const onBrowser = banned.filter((b) => !b.after_ban && b.via === 'browser').map((b) => b.name);
    if (onBrowser.length) flags.push(['danger', `Shares a browser with banned ${onBrowser.join(', ')}.`]);
    const onAddress = banned.filter((b) => !b.after_ban && b.via === 'address').map((b) => b.name);
    if (onAddress.length) flags.push(['warn', `Shares an address with banned ${onAddress.join(', ')} (could be a household).`]);
    const devices = links.devices || [];
    if (devices.some((d) => d.web && (!d.user_agent || AUTOMATION.test(d.user_agent)))) {
        flags.push(['danger', 'Logged in from an automated browser (headless, or no user agent).']);
    }
    const busy = Math.max(0, ...devices.map((d) => d.others + 1));
    if (busy >= BUSY_DEVICE) flags.push(['warn', `One of their browsers is used by ${busy} accounts.`]);
    const week = Date.now() - 7 * 86_400_000;
    const recent = devices.filter((d) => d.web !== null && d.last_ms >= week).length;
    if (recent >= DEVICE_CHURN) flags.push(['warn', `${recent} different browsers or installs in a week (storage wiped, or many machines).`]);
    for (const d of devices) {
        if (!d.user_agent || !d.platform) continue;
        const said = PLATFORM_WORDS.find(([, word]) => word.test(d.user_agent))?.[0];
        if (said && said !== d.platform && !(said === 'Linux' && d.platform === 'Android')) {
            flags.push(['warn', `A browser's hints disagree: its user agent says ${said}, its platform ${d.platform}. Possibly a faked user agent.`]);
            break;
        }
    }
    return flags;
}

// Every account a chain of shared browsers and addresses reaches from
// `name`, for following a ban to the rest of a farm.
function chainSection(name, onModerated) {
    const list = h('div', {class: 'list'});
    const act = h('div', {class: 'actions'});
    let depth = 2;
    const depthChips = h('div', {class: 'chips'});
    const load = async () => {
        replace(depthChips, [1, 2, 3].map((d) => h('button', {
            type: 'button', class: 'chip', 'aria-pressed': String(depth === d), on: {
                click: () => {
                    depth = d;
                    load();
                }
            }
        }, plural(d, 'link'))));
        replace(list, h('div', {class: 'empty'}, h('div', {class: 'spinner'})));
        try {
            const chain = await api('GET', `/lookup/link-chain?${query({name, depth})}`);
            replace(list, chain.length ? chain.map((c) => h('div', {class: 'list-item'},
                    h('div', {class: `list-icon ${c.banned_until_ms ? 'danger' : ''}`}, icon(c.link === 'browser' ? 'monitor' : 'globe')),
                    h('div', {class: 'list-main'},
                        h('div', {class: 'list-title'}, h('b', {}, playerLink(c.name)), ' ', playerBadges({staff_level: c.staff_level}), sanctionBadges(c)),
                        h('div', {class: 'list-sub'}, joined([
                            `${c.depth === 1 ? 'Directly' : `${c.depth} links away`}, through ${c.link === 'browser' ? 'a browser' : 'an address'} shared with `,
                        ]), playerLink(c.via))),
                    h('div', {class: 'list-side'}, c.last_ms ? ago(c.last_ms) : '')))
                : h('div', {class: 'empty'}, icon('users'), 'Nothing links to them.'));
            const open = chain.filter((c) => !c.banned_until_ms).map((c) => c.name).slice(0, 26);
            replace(act, open.length && !isTrial() ? button(`Ban ${plural(open.length, 'unbanned account')}`, {
                small: true,
                icon: 'ban',
                kind: 'danger',
                on: {
                    click: () => moderateDialog(open[0], 'ban', () => {
                        load();
                        onModerated?.();
                    }, undefined, {also: open.slice(1)})
                }
            }) : null);
        } catch (e) {
            failed(list, e);
        }
    };
    load();
    return h('section', {class: 'card'},
        h('div', {class: 'card-head'}, h('h2', {}, 'Ban chain'), act),
        h('div', {class: 'card-body'},
            h('p', {class: 'hint'}, 'Every account their browsers and addresses lead to, and the accounts those lead to. Addresses shared by more than 25 accounts (public Wi-Fi, mobile networks) are skipped.'),
            depthChips),
        list);
}

// What a device's client said it was: "Chrome on Windows".
function deviceName(d) {
    const what = d.web === null ? 'Before devices were recorded'
        : !d.web ? 'Desktop client'
            : d.user_agent ? device(d.user_agent) : 'Web client';
    return what + (d.platform && !what.includes(d.platform) ? ` (${d.platform})` : '');
}

// A device's short name on the page, so addresses can point at it: D1, D2...
const deviceTag = (n) => h('span', {class: 'device-tag mono', title: `Device ${n}`}, `D${n}`);

// One device a player logged in with: what its client said it was, its
// login id and the addresses it was used from. `n` is its number on the page.
function deviceItem(d, n) {
    const more = d.addresses - d.ips.length;
    return h('div', {class: 'list-item'},
        h('div', {class: 'list-icon'}, icon(d.touch ? 'phone' : 'monitor')),
        h('div', {class: 'list-main'},
            h('div', {class: 'list-title', title: d.user_agent || ''}, deviceTag(n), ' ', h('b', {}, deviceName(d)), ' ',
                d.touch && h('span', {class: 'badge', title: 'The browser said it is a phone or tablet'}, 'Touch'), ' ',
                d.others > 0 && h('span', {
                    class: 'badge warn',
                    title: 'Other accounts logged in from this same browser (or desktop client install)'
                }, plural(d.others, 'other account'))),
            h('div', {class: 'list-sub'}, joined([
                plural(d.logins, 'login'),
                d.language,
                d.uid && h('span', {title: 'The id this browser or install sends at login'}, 'Login id ', h('span', {class: 'mono'}, d.uid)),
                `first ${ago(d.first_ms)}`,
            ])),
            d.ips.length > 0 && h('div', {class: 'list-sub'}, 'From ',
                joined(d.ips.map((ip) => h('span', {class: 'mono address-ref'}, ip))),
                more > 0 ? ` and ${more} more` : '')),
        h('div', {class: 'list-side', title: fullTime(d.last_ms)}, ago(d.last_ms)));
}

// Lights up the rows `others()` gives while `row` is pointed at or focused.
function lightUp(row, others) {
    row.tabIndex = 0;
    const set = (on) => {
        row.classList.toggle('linked-source', on);
        for (const el of others()) el.classList.toggle('linked', on);
    };
    for (const [ev, on] of [['mouseenter', true], ['mouseleave', false], ['focus', true], ['blur', false]]) {
        row.addEventListener(ev, () => set(on));
    }
}

function linksSection(name, onModerated) {
    const addresses = spinnerList();
    const accounts = spinnerList();
    const devices = spinnerList();
    const flags = h('div', {});
    const note = h('p', {class: 'hint'});
    const act = h('div', {class: 'actions'});
    const load = async () => {
        try {
            const links = await api('GET', `/lookup/links?${query({name})}`);
            note.textContent = links.addresses_shown
                ? 'Addresses kept for 180 days. Viewing them is audited.'
                : 'Addresses show as labels: the same label is the same address. Admins see the addresses.';
            // Each address with the devices used from it, and each device
            // with its addresses: pointing at one lights up the other side.
            const deviceList = links.devices || [];
            const deviceRows = deviceList.map((d, i) => deviceItem(d, i + 1));
            const addressRows = new Map();
            replace(addresses, links.addresses.length ? links.addresses.map((a) => {
                    const used = deviceList.map((d, i) => [d, i]).filter(([d]) => d.ips.includes(a.ip));
                    const row = h('div', {class: 'list-item'},
                        h('div', {class: 'list-icon'}, icon('globe')),
                        h('div', {class: 'list-main'},
                            h('div', {class: 'list-title'}, h('span', {class: 'mono'}, a.ip),
                                a.others ? h('span', {class: 'badge warn'}, `${plural(a.others, 'other account')}`) : null),
                            h('div', {class: 'list-sub'}, `${plural(a.logins, 'login')} · first ${ago(a.first_ms)}`),
                            used.length > 0 && h('div', {class: 'list-sub'}, 'On ',
                                joined(used.map(([d, i]) => h('span', {class: 'device-ref'}, deviceTag(i + 1), ' ', deviceName(d)))))),
                        h('div', {class: 'list-side', title: fullTime(a.last_ms)}, ago(a.last_ms)));
                    addressRows.set(a.ip, row);
                    lightUp(row, () => used.map(([, i]) => deviceRows[i]));
                    return row;
                })
                : h('div', {class: 'empty'}, icon('globe'), 'No logins on record.'));
            deviceList.forEach((d, i) => lightUp(deviceRows[i], () => d.ips.map((ip) => addressRows.get(ip)).filter(Boolean)));
            replace(accounts, links.accounts.length ? links.accounts.map((a) => h('div', {class: 'list-item'},
                    h('div', {class: `list-icon ${a.banned_until_ms ? 'danger' : ''}`}, icon('user')),
                    h('div', {class: 'list-main'},
                        h('div', {class: 'list-title'}, h('b', {}, playerLink(a.name)), ' ', playerBadges({staff_level: a.staff_level}), sanctionBadges(a),
                            a.shared_clients > 0 && h('span', {
                                class: 'badge warn',
                                title: 'Logged in from the same browser (or desktop client install) as them'
                            }, icon('monitor'), 'Same browser')),
                        h('div', {class: 'list-sub'}, joined([
                            a.shared.length > 0 && `Shares ${a.shared.length === 1 ? a.shared[0] : plural(a.shared.length, 'address', 'addresses')}`,
                            a.logins ? plural(a.logins, 'login') : 'registered there',
                        ]))),
                    h('div', {class: 'list-side', title: fullTime(a.last_ms)}, ago(a.last_ms))))
                : h('div', {class: 'empty'}, icon('users'), 'No other account shares an address or a browser with them.'));
            replace(devices, deviceRows.length ? deviceRows
                : h('div', {class: 'empty'}, icon('monitor'), 'No logins on record.'));
            const raised = linkFlags(links);
            replace(flags, raised.length > 0 && h('div', {class: `callout ${raised.some(([tone]) => tone === 'danger') ? 'danger' : 'warn'}`}, icon('alert'),
                h('div', {}, h('b', {}, 'Their devices and links raise flags.'),
                    h('ul', {class: 'flag-list'}, raised.map(([, text]) => h('li', {}, text))))));
            const open = links.accounts.filter((a) => !a.banned_until_ms).map((a) => a.name);
            replace(act, open.length
                && !isTrial() ? button(`Ban with ${plural(open.length, 'linked account')}`, {
                    small: true,
                    icon: 'ban',
                    kind: 'danger',
                    on: {
                        click: () => moderateDialog(name, 'ban', () => {
                            load();
                            onModerated?.();
                        }, undefined, {also: open})
                    }
                })
                : null);
        } catch (e) {
            failed(addresses, e);
            failed(accounts, e);
            failed(devices, e);
        }
    };
    load();
    return h('div', {class: 'column'},
        flags,
        h('div', {class: 'split'},
            h('section', {class: 'card'},
                h('div', {class: 'card-head'}, h('h2', {}, 'Linked accounts'), act),
                accounts),
            h('section', {class: 'card'},
                h('div', {class: 'card-head'}, h('h2', {}, 'Login addresses')),
                h('div', {class: 'card-body'}, note),
                addresses)),
        h('section', {class: 'card'},
            h('div', {class: 'card-head'}, h('h2', {}, 'Devices'),
                h('span', {class: 'hint hide-mobile'}, 'What each browser or install said it was (browsers can claim anything). Point at a device or an address to see the other side.')),
            devices),
        chainSection(name, () => {
            load();
            onModerated?.();
        }));
}

// ---------------------------------------------------------------------------
// Controls: kill switches and the login lockdown, on every world. The
// console keeps them and sets them again on any world that comes back
// without them.

const LOCKDOWNS = [[0, 'Off'], [1, '1 hour'], [6, '6 hours'], [24, '1 day'], [72, '3 days'], [168, '1 week']];
const lockdownLabel = (hours) => LOCKDOWNS.find(([h0]) => h0 === hours)?.[1].toLowerCase() ?? plural(hours, 'hour');

function worldSyncRow(w) {
    const [label, tone] = !w.online ? ['Offline', '']
        : !w.supported ? ['Too old for controls', 'warn']
            : w.error ? ['Refused', 'danger']
                : w.in_sync ? ['In sync', 'ok'] : ['Updating', 'warn'];
    return h('tr', {},
        h('td', {}, h('b', {}, `World ${w.world}`), h('span', {class: 'muted'}, ` node ${w.node}`)),
        h('td', {}, h('span', {class: `badge ${tone}`}, label)),
        h('td', {class: 'muted'}, w.error || (w.matched ? plural(w.matched.reduce((a, b) => a + b, 0), 'script') + ' switched off' : '')));
}

// What's wrong with patterns as the worlds' scripts have them, from a
// check (`/controls/check`): for each, a line if it matches no script on
// some or every world that answered.
function patternProblems(check) {
    const out = new Map();
    if (!check || !check.checked.length) return out;
    for (const p of check.patterns) {
        const none = check.checked.filter((_, i) => p.matched[i] === 0);
        if (!none.length) continue;
        out.set(p.pattern, none.length === check.checked.length
            ? 'matches no script on any world'
            : `matches no script on ${none.length === 1 ? 'world' : 'worlds'} ${none.join(', ')}`);
    }
    return out;
}

// The same from what the worlds report for the switches that are on: their
// counts are current, a script reload included.
function livePatternProblems(page) {
    const out = new Map();
    const reporting = page.worlds.filter((w) => w.online && w.in_sync && Array.isArray(w.matched));
    if (!reporting.length) return out;
    page.wanted.switches.forEach((pattern, i) => {
        const none = reporting.filter((w) => w.matched[i] === 0).map((w) => w.world);
        if (!none.length) return;
        out.set(pattern, none.length === reporting.length
            ? 'matches no script on any world: it switches nothing off'
            : `matches no script on ${none.length === 1 ? 'world' : 'worlds'} ${none.join(', ')}: it switches nothing off there`);
    });
    return out;
}

const problemLines = (patterns, problems) => patterns.filter((p) => problems.has(p)).map((p) =>
    h('div', {class: 'list-sub tone-warn'}, icon('alert'), ' ', h('span', {class: 'mono'}, p), ` ${problems.get(p)}.`));

// After a switch went on: what the world check found off about it.
function switchWarnings(page) {
    for (const w of page.warnings || []) toast(w, 'error');
}

function customSwitchDialog(onDone) {
    const label = h('input', {
        class: 'input',
        maxlength: '60',
        required: true,
        placeholder: 'e.g. Party room',
        autofocus: true
    });
    const patterns = h('textarea', {
        class: 'input mono',
        required: true,
        placeholder: '[oploc1,partyroom_lever]\n[if_button,partyroom_*]',
        rows: '5'
    });
    const results = h('div', {class: 'pattern-check'});
    const lines = () => patterns.value.split('\n').map((l) => l.trim()).filter(Boolean);
    const checkNow = async () => {
        const wanted = lines();
        if (!wanted.length) {
            replace(results);
            return;
        }
        replace(results, h('div', {class: 'muted'}, h('span', {class: 'spinner'}), ' Checking against the worlds\' scripts...'));
        try {
            const check = await api('POST', '/controls/check', {patterns: wanted});
            if (!check.checked.length) {
                replace(results, h('div', {class: 'tone-warn'}, 'No world answered, so they can\'t be checked.'));
                return;
            }
            replace(results, check.patterns.map((p) => {
                const most = Math.max(0, ...p.matched);
                const none = check.checked.filter((_, i) => p.matched[i] === 0);
                const tone = most === 0 || most > 300 ? 'tone-danger' : none.length ? 'tone-warn' : 'tone-ok';
                const text = most === 0 ? 'matches no script'
                    : most > 300 ? `matches ${most} scripts: too many`
                        : `${plural(most, 'script')}${none.length ? `, none on ${none.length === 1 ? 'world' : 'worlds'} ${none.join(', ')}` : ''}`;
                return h('div', {class: 'pattern-row'}, h('span', {class: 'mono'}, p.pattern), h('span', {class: tone}, text));
            }), check.unreachable.length > 0 && h('div', {class: 'muted'}, `Not checked on world ${check.unreachable.join(', ')}, which didn't answer.`));
        } catch (e) {
            replace(results, h('div', {class: 'tone-danger'}, e.message));
        }
    };
    patterns.addEventListener('input', debounce(checkNow, 500));
    openDialog({
        title: 'Custom kill switch',
        subtitle: 'Switch off the scripts whose names match, on every world.',
        body: [
            h('label', {class: 'field'}, h('span', {}, 'Label'), label),
            h('label', {class: 'field'}, h('span', {}, 'Script patterns, one per line'), patterns,
                h('p', {class: 'hint'}, 'Names as in the content, like [opheld5,_] or [inv_button1,bank_main:inv]. * matches anything. A pattern that matches no script, or over 300, is refused.'),
                results),
            h('div', {class: 'callout warn'}, icon('alert'), h('div', {}, 'Players trying a switched-off script are told it\'s switched off for now. Scripts already running finish.')),
        ],
        actions: [{label: 'Cancel'}, {
            label: 'Switch off', kind: 'danger', icon: 'zap',
            run: async () => {
                const page = await api('POST', '/controls/switch', {
                    on: true,
                    label: label.value.trim(),
                    patterns: patterns.value.split('\n')
                });
                toast(`${label.value.trim()} is switched off.`);
                switchWarnings(page);
                onDone?.(page);
            },
        }],
    });
}

function controlsPage(ctx) {
    const lockdownBody = h('div', {class: 'card-body'});
    const switches = spinnerList();
    const worlds = h('tbody');
    const head = h('div', {});
    let current = null;
    // How the presets' patterns match the worlds' scripts now: checked when
    // the page opens and every few minutes, so a preset whose scripts were
    // renamed shows it before anyone tries it.
    let presetCheck = null;

    const setSwitch = async (body, message) => {
        try {
            draw(await api('POST', '/controls/switch', body));
            toast(message);
        } catch (e) {
            toast(e.message, 'error');
        }
    };
    // Turning one on from its dialog: a refusal (a pattern matching nothing)
    // shows there.
    const switchOn = async (body, message) => {
        const page = await api('POST', '/controls/switch', body);
        draw(page);
        toast(message);
        switchWarnings(page);
    };
    const checkPresets = async () => {
        if (!current) return;
        try {
            presetCheck = await api('POST', '/controls/check', {patterns: [...new Set(current.presets.flatMap((p) => p.patterns))]});
            draw(current);
        } catch { /* the switches work without it */
        }
    };

    const draw = (page) => {
        current = page;
        const {desired, can_change: canChange} = page;
        const lockdown = desired.lockdown;
        const active = new Map(desired.switches.map((s) => [s.key, s]));
        replace(head, pageHead('Controls', 'Kill switches and the login lockdown, on every world.',
                canChange && button('Custom switch', {icon: 'plus', on: {click: () => customSwitchDialog(draw)}})),
            !canChange && h('div', {class: 'callout'}, icon('shield'), h('div', {}, 'Changing these needs the admin role.')));

        let hours = lockdown?.hours ?? 24;
        const custom = h('input', {
            class: 'input',
            type: 'number',
            min: '1',
            max: '8760',
            step: '1',
            placeholder: 'Custom, in hours',
            inputmode: 'numeric'
        });
        const pick = h('div', {class: 'chips'});
        const drawPick = () => replace(pick, LOCKDOWNS.filter(([h0]) => h0).map(([h0, label]) =>
            h('button', {
                type: 'button',
                class: 'chip',
                'aria-pressed': String(hours === h0 && !custom.value),
                on: {
                    click: () => {
                        custom.value = '';
                        hours = h0;
                        drawPick();
                    }
                }
            }, label)));
        custom.addEventListener('input', () => {
            const n = Math.round(Number(custom.value));
            if (n >= 1) hours = n;
            drawPick();
        });
        drawPick();
        const apply = (to) => async () => {
            try {
                draw(await api('POST', '/controls/lockdown', {hours: to}));
                toast(to ? `Accounts younger than ${lockdownLabel(to)} are turned away.` : 'Lockdown lifted.');
            } catch (e) {
                toast(e.message, 'error');
            }
        };
        replace(lockdownBody,
            h('div', {class: `callout ${lockdown ? 'danger' : ''}`}, icon(lockdown ? 'lock' : 'unlock'), h('div', {}, lockdown
                ? [h('b', {}, `On: accounts younger than ${lockdownLabel(lockdown.hours)} can't log in.`), ` Set by ${lockdown.by} ${ago(lockdown.at_ms)}. Staff and older accounts log in as usual.`]
                : 'Off. Any account can log in.')),
            canChange && h('div', {class: 'field'}, h('span', {}, lockdown ? 'Change to accounts younger than' : 'Turn away accounts younger than'), pick, custom,
                h('p', {class: 'hint'}, 'For a bot wave: new accounts are refused with "could not complete login". Account creation stays open; they just can\'t play until they\'re old enough.')),
            canChange && h('div', {class: 'chips'},
                button(lockdown ? 'Change lockdown' : 'Lock down', {
                    icon: 'lock', kind: 'danger', on: {
                        click: () => confirmDialog({
                            title: 'Lock down logins',
                            subtitle: 'Every world',
                            body: [h('p', {}, `Non-staff accounts registered less than ${lockdownLabel(hours)} ago won't be able to log in. Players already on stay on.`)],
                            label: 'Lock down', kind: 'danger', run: apply(hours),
                        })
                    }
                }),
                lockdown && button('Lift lockdown', {icon: 'unlock', on: {click: apply(0)}})));

        const live = livePatternProblems(page);
        const checked = patternProblems(presetCheck);
        const presetRows = page.presets.map((p) => {
            const on = active.get(p.key);
            const problems = on ? live : checked;
            const wrong = p.patterns.some((x) => problems.has(x));
            return h('div', {class: 'list-item'},
                h('div', {class: `list-icon ${on ? 'danger' : ''}`}, icon('zap')),
                h('div', {class: 'list-main'},
                    h('div', {class: 'list-title'}, h('b', {}, p.label), on && h('span', {class: 'badge danger'}, 'Switched off'),
                        wrong && h('span', {
                            class: 'badge warn',
                            title: 'Part of it matches no script: its scripts may have been renamed'
                        }, icon('alert'), 'Check patterns')),
                    h('div', {class: 'list-sub'}, p.description),
                    h('div', {class: 'list-sub mono'}, p.patterns.join('  ')),
                    problemLines(p.patterns, problems),
                    on && h('div', {class: 'list-sub'}, `By ${on.by} ${ago(on.at_ms)}`)),
                canChange && h('div', {class: 'chips'}, on
                    ? button('Turn back on', {
                        small: true,
                        icon: 'undo',
                        on: {click: () => setSwitch({key: p.key, on: false}, `${p.label} is back on.`)}
                    })
                    : button('Switch off', {
                        small: true, icon: 'zap', kind: 'danger', on: {
                            click: () => confirmDialog({
                                title: `Switch off ${p.label.toLowerCase()}`,
                                subtitle: 'Every world',
                                body: [h('p', {}, p.description), h('p', {class: 'muted'}, 'Players trying it are told it\'s switched off for now. It stays off until you turn it back on, restarts included.')],
                                label: 'Switch off',
                                kind: 'danger',
                                run: () => switchOn({key: p.key, on: true}, `${p.label} is switched off.`),
                            })
                        }
                    })));
        });
        const customRows = desired.switches.filter((s) => !page.presets.some((p) => p.key === s.key)).map((s) =>
            h('div', {class: 'list-item'},
                h('div', {class: 'list-icon danger'}, icon('zap')),
                h('div', {class: 'list-main'},
                    h('div', {class: 'list-title'}, h('b', {}, s.label), h('span', {class: 'badge danger'}, 'Switched off'), h('span', {class: 'badge'}, 'Custom'),
                        s.patterns.some((x) => live.has(x)) && h('span', {class: 'badge warn'}, icon('alert'), 'Check patterns')),
                    h('div', {class: 'list-sub mono'}, s.patterns.join('  ')),
                    problemLines(s.patterns, live),
                    h('div', {class: 'list-sub'}, `By ${s.by} ${ago(s.at_ms)}`)),
                canChange && button('Turn back on', {
                    small: true,
                    icon: 'undo',
                    on: {click: () => setSwitch({key: s.key, on: false}, `${s.label} is back on.`)}
                })));
        replace(switches, ...customRows, ...presetRows);
        replace(worlds, page.worlds.length ? page.worlds.map(worldSyncRow) : h('tr', {}, h('td', {
            colspan: '3',
            class: 'muted'
        }, 'No worlds configured.')));
    };

    // Polled, but only redrawn on a change, so a lockdown length being picked
    // isn't reset under the pointer.
    let seen = '';
    const load = async () => {
        try {
            const page = await api('GET', '/controls');
            const json = JSON.stringify(page);
            if (json !== seen) {
                seen = json;
                draw(page);
            }
            if (!presetCheck) checkPresets();
        } catch (e) {
            failed(switches, e);
        }
    };
    load();
    const presetTimer = setInterval(() => {
        if (!document.hidden) checkPresets();
    }, 5 * 60 * 1000);
    ctx.cleanup(() => clearInterval(presetTimer));
    const timer = setInterval(() => {
        if (!document.hidden && !document.querySelector('dialog[open]')) load();
    }, 10000);
    ctx.cleanup(() => clearInterval(timer));

    return h('div', {class: 'page'},
        head,
        h('div', {class: 'split'},
            h('section', {class: 'card'}, h('div', {class: 'card-head'}, h('h2', {}, 'Kill switches')), switches),
            h('div', {class: 'column'},
                h('section', {class: 'card'}, h('div', {class: 'card-head'}, h('h2', {}, 'Login lockdown')), lockdownBody),
                h('section', {class: 'card'}, h('div', {class: 'card-head'}, h('h2', {}, 'Worlds')),
                    h('div', {class: 'table-wrap'}, h('table', {}, worlds))))));
}

// A banner for the overview while any control is on.
async function controlsBanner(el) {
    try {
        const {desired} = await api('GET', '/controls');
        const off = desired.switches.map((s) => s.label);
        const parts = [
            off.length && `${listNames(off)} ${off.length === 1 ? 'is' : 'are'} switched off`,
            desired.lockdown && `logins are locked down to accounts older than ${lockdownLabel(desired.lockdown.hours)}`,
        ].filter(Boolean);
        replace(el, parts.length ? h('div', {class: 'callout danger'}, icon('zap'),
            h('div', {}, `On every world, ${parts.join(', and ')}. `, link('/controls', {class: 'name-link'}, 'Controls'))) : null);
    } catch { /* the page works without it */
    }
}

// ---------------------------------------------------------------------------
// Schedule: reboots and broadcasts set to go out later, once or on repeat.

const REPEATS = [[null, 'Once'], [1440, 'Every day'], [10080, 'Every week']];
const repeatLabel = (mins) => (mins ? REPEATS.find(([m]) => m === mins)?.[1] ?? `Every ${span(mins * 60)}` : 'Once');
// A datetime-local input's value for `ms`, and the time a value stands
// for, in the zone times are shown in.
const localInput = (ms) => {
    const d = new Date(ms);
    const pad = (n) => String(n).padStart(2, '0');
    return inUtc()
        ? `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())}T${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`
        : `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
};
const inputTime = (value) => (value ? new Date(inUtc() ? `${value}Z` : value).getTime() : NaN);

function jobWorlds(job) {
    if (!job.nodes.length) return 'Every world online';
    const names = job.nodes.map((n) => {
        const w = state.worlds.find((x) => x.node === n);
        return w ? worldName(w) : `Node ${n}`;
    });
    return names.length <= 4 ? names.join(', ') : plural(names.length, 'world');
}

function jobItem(job, canChange, onChange) {
    const reboot = job.task.kind === 'reboot';
    const title = reboot
        ? [h('b', {}, 'Reboot'), ` · ${clock(job.task.countdown_secs)} countdown`, job.task.stagger_secs ? `, ${job.task.stagger_secs}s apart` : '', job.task.reason ? [' · ', h('span', {class: 'muted'}, `"${job.task.reason}"`)] : '']
        : [h('b', {}, 'Broadcast'), ': ', job.task.message];
    const last = job.last;
    return h('div', {class: 'list-item'},
        h('div', {class: `list-icon ${reboot ? 'warn' : 'accent'}`}, icon(reboot ? 'power' : 'megaphone')),
        h('div', {class: 'list-main'},
            h('div', {class: 'list-title'}, title),
            h('div', {class: 'list-sub'}, joined([jobWorlds(job), repeatLabel(job.every_mins), `set by ${job.by}`])),
            last && h('div', {class: `list-sub ${last.ok ? '' : 'tone-danger'}`}, `Last ran ${ago(last.at_ms)}: ${last.summary}`)),
        h('div', {class: 'list-side', title: fullTime(job.next_ms)},
            h('div', {}, shortTime(job.next_ms)), h('div', {}, 'in ', countdown(job.next_ms))),
        canChange && button(null, {
            small: true, icon: 'x', 'aria-label': 'Cancel this job', title: 'Cancel', on: {
                click: () => confirmDialog({
                    title: 'Cancel scheduled job',
                    body: [h('p', {}, reboot ? 'This reboot won\'t go out.' : 'This broadcast won\'t go out.'), job.every_mins && h('p', {class: 'muted'}, 'Nor will any of its repeats.')],
                    label: 'Cancel job', kind: 'danger',
                    run: async () => {
                        onChange(await api('POST', `/schedule/${job.id}/delete`));
                        toast('Cancelled.');
                    },
                })
            }
        }));
}

function scheduleDialog(kind = 'reboot', picked = [], onDone) {
    let task = kind;
    let everyWorld = !picked.length;
    const chosen = new Set(picked.map((w) => w.node));
    let countdown_ = 300;
    let stagger = 0;
    let every = null;
    const start = new Date(Date.now() + 3600 * 1000);
    start.setMinutes(0, 0, 0);
    const at = h('input', {class: 'input', type: 'datetime-local', required: true, value: localInput(start.getTime())});
    const kindPick = h('div', {class: 'segmented', role: 'group', 'aria-label': 'What'});
    const worldMode = h('div', {class: 'segmented', role: 'group', 'aria-label': 'Worlds'});
    const worldChips = h('div', {class: 'chips'});
    const delayChips = h('div', {class: 'chips'});
    const staggerChips = h('div', {class: 'chips'});
    const repeatChips = h('div', {class: 'chips'});
    const customEvery = h('input', {
        class: 'input',
        type: 'number',
        min: '1',
        max: '744',
        step: '1',
        placeholder: 'Or every N hours',
        inputmode: 'numeric'
    });
    const reason = h('input', {
        class: 'input',
        maxlength: '80',
        placeholder: 'Optional, e.g. "daily restart"',
        autocomplete: 'off'
    });
    const message = h('textarea', {class: 'input', maxlength: '400', placeholder: 'Remember to vote for us!'});
    const rebootFields = h('div', {class: 'column'});
    const broadcastFields = h('div', {class: 'column'});
    const when = h('p', {class: 'hint'});

    const chip = (pressed, label, onClick) => h('button', {
        type: 'button',
        class: 'chip',
        'aria-pressed': String(pressed),
        on: {click: onClick}
    }, label);
    const draw = () => {
        replace(kindPick,
            h('button', {
                type: 'button', 'aria-pressed': String(task === 'reboot'), on: {
                    click: () => {
                        task = 'reboot';
                        draw();
                    }
                }
            }, 'Reboot'),
            h('button', {
                type: 'button', 'aria-pressed': String(task === 'broadcast'), on: {
                    click: () => {
                        task = 'broadcast';
                        draw();
                    }
                }
            }, 'Broadcast'));
        replace(worldMode,
            h('button', {
                type: 'button', 'aria-pressed': String(everyWorld), on: {
                    click: () => {
                        everyWorld = true;
                        draw();
                    }
                }
            }, 'Every world'),
            h('button', {
                type: 'button', 'aria-pressed': String(!everyWorld), on: {
                    click: () => {
                        everyWorld = false;
                        draw();
                    }
                }
            }, 'Pick worlds'));
        worldChips.classList.toggle('hidden', everyWorld);
        replace(worldChips, state.worlds.map((w) => chip(chosen.has(w.node), worldName(w), () => {
            if (chosen.has(w.node)) chosen.delete(w.node); else chosen.add(w.node);
            draw();
        })));
        replace(delayChips, [[60, '1 min'], [300, '5 min'], [600, '10 min'], [1800, '30 min'], [3600, '1 hour']].map(([s, label]) => chip(countdown_ === s, label, () => {
            countdown_ = s;
            draw();
        })));
        replace(staggerChips, [[0, 'None'], [15, '15s'], [30, '30s'], [60, '1 min'], [300, '5 min']].map(([s, label]) => chip(stagger === s, label, () => {
            stagger = s;
            draw();
        })));
        replace(repeatChips, REPEATS.map(([m, label]) => chip(every === m && !customEvery.value, label, () => {
            customEvery.value = '';
            every = m;
            draw();
        })));
        rebootFields.classList.toggle('hidden', task !== 'reboot');
        broadcastFields.classList.toggle('hidden', task !== 'broadcast');
        message.required = task === 'broadcast';
        const ms = inputTime(at.value);
        when.textContent = Number.isFinite(ms)
            ? task === 'reboot'
                ? `The countdown starts ${fullTime(ms)}; the first world goes down at ${new Date(ms + countdown_ * 1000).toLocaleTimeString([], zoned({
                    hour: '2-digit',
                    minute: '2-digit'
                }))}.`
                : `Goes out ${fullTime(ms)}.`
            : '';
    };
    customEvery.addEventListener('input', () => {
        const n = Math.round(Number(customEvery.value));
        if (n >= 1) every = n * 60;
        draw();
    });
    at.addEventListener('input', draw);

    append(rebootFields,
        h('div', {class: 'field'}, h('span', {}, 'Countdown'), delayChips),
        h('div', {class: 'field'}, h('span', {}, 'Stagger between worlds'), staggerChips),
        h('label', {class: 'field'}, h('span', {}, 'Reason'), reason));
    append(broadcastFields, h('label', {class: 'field'}, h('span', {}, 'Message'), message));
    draw();

    openDialog({
        title: 'Schedule',
        subtitle: 'Runs from the console at the time you pick, in your time zone.',
        body: [
            kindPick,
            h('div', {class: 'field'}, h('span', {}, 'Worlds'), worldMode, worldChips,
                h('p', {class: 'hint'}, 'Every world means every world online when it runs.')),
            h('label', {class: 'field'}, h('span', {}, inUtc() ? 'When (UTC)' : 'When'), at, when),
            h('div', {class: 'field'}, h('span', {}, 'Repeat'), repeatChips, customEvery),
            rebootFields,
            broadcastFields,
        ],
        actions: [{label: 'Cancel'}, {
            label: 'Schedule', kind: 'primary', icon: 'clock',
            run: async () => {
                const nodes = everyWorld ? [] : [...chosen];
                if (!everyWorld && !nodes.length) throw new Error('Pick at least one world.');
                const atMs = inputTime(at.value);
                if (!Number.isFinite(atMs)) throw new Error('Pick a time.');
                const body = {
                    task: task === 'reboot'
                        ? {
                            kind: 'reboot',
                            countdown_secs: countdown_,
                            stagger_secs: stagger,
                            reason: reason.value.trim() || null
                        }
                        : {kind: 'broadcast', message: message.value.trim()},
                    nodes,
                    at_ms: atMs,
                    every_mins: every,
                };
                const page = await api('POST', '/schedule', body);
                toast(`Scheduled for ${fullTime(atMs)}.`);
                onDone?.(page);
            },
        }],
    });
}

function schedulePage(ctx) {
    const list = spinnerList();
    const head = h('div', {});
    const draw = (page) => {
        replace(head, pageHead('Schedule', 'Reboots and broadcasts set to go out later, once or on repeat. Times are in your time zone.',
                page.can_change && button('Schedule broadcast', {
                    icon: 'megaphone',
                    on: {click: () => scheduleDialog('broadcast', [], draw)}
                }),
                page.can_change && button('Schedule reboot', {
                    icon: 'power',
                    kind: 'primary',
                    on: {click: () => scheduleDialog('reboot', [], draw)}
                })),
            !page.can_change && h('div', {class: 'callout'}, icon('shield'), h('div', {}, 'Scheduling needs the admin role.')));
        replace(list, page.jobs.length ? page.jobs.map((j) => jobItem(j, page.can_change, draw))
            : h('div', {class: 'empty'}, icon('clock'), 'Nothing scheduled.'));
    };
    const load = async () => {
        try {
            draw(await api('GET', '/schedule'));
        } catch (e) {
            failed(list, e);
        }
    };
    load();
    const timer = setInterval(() => {
        if (!document.hidden && !document.querySelector('dialog[open]')) load();
    }, 15000);
    ctx.cleanup(() => clearInterval(timer));
    return h('div', {class: 'page'}, head,
        h('div', {class: 'callout'}, icon('alert'), h('div', {}, 'A job due while the console is down is skipped if it\'s more than 5 minutes late, and noted in the audit log.')),
        h('section', {class: 'card'}, list));
}

// ---------------------------------------------------------------------------
// Sanction templates: a named offence with a ladder of mutes and bans. A
// player's next offence of a template gets the next step up.

const stepLabel = (s) => `${s.action === 'ban' ? 'Ban' : 'Mute'} ${hoursLabel(s.hours)}`;

async function loadTemplates() {
    state.templates = (await api('GET', '/templates')).templates;
    return state.templates;
}

function ladder(steps, next) {
    return h('div', {class: 'chips'}, steps.map((s, i) => h('span', {
        class: `badge ${i === next ? (s.action === 'ban' ? 'danger' : 'warn') : ''}`,
        title: i === next ? 'Their next offence gets this' : `Offence ${i + 1}${i === steps.length - 1 ? ' and after' : ''}`,
    }, `${i + 1}. ${stepLabel(s)}`)));
}

function templateDialog(templates, index, onDone) {
    const t = index === null
        ? {id: '', name: '', description: '', steps: [{action: 'mute', hours: 48}], expire_days: 365}
        : structuredClone(templates[index]);
    const name = h('input', {
        class: 'input',
        maxlength: '60',
        required: true,
        value: t.name,
        placeholder: 'e.g. Macroing',
        autofocus: true
    });
    const description = h('textarea', {
        class: 'input',
        maxlength: '300',
        value: t.description,
        placeholder: 'What it covers, for whoever applies it.'
    });
    const expire = h('input', {
        class: 'input',
        type: 'number',
        min: '1',
        max: '3650',
        step: '1',
        required: true,
        value: String(t.expire_days),
        inputmode: 'numeric'
    });
    const steps = h('div', {class: 'column'});
    const drawSteps = () => replace(steps, t.steps.map((s, i) => {
        const custom = h('input', {
            class: 'input',
            type: 'number',
            min: '1',
            max: '87600',
            step: '1',
            placeholder: 'Hours',
            inputmode: 'numeric',
            value: DURATIONS.some(([h0]) => h0 === s.hours) ? '' : String(s.hours)
        });
        custom.addEventListener('input', () => {
            const n = Math.round(Number(custom.value));
            if (n >= 1) s.hours = n;
        });
        return h('div', {class: 'field'},
            h('span', {}, `Offence ${i + 1}${i === t.steps.length - 1 ? ' and after' : ''}`),
            h('div', {class: 'chips'},
                h('div', {class: 'segmented', role: 'group', 'aria-label': 'Action'},
                    ['mute', 'ban'].map((a) => h('button', {
                        type: 'button',
                        'aria-pressed': String(s.action === a),
                        on: {
                            click: () => {
                                s.action = a;
                                drawSteps();
                            }
                        }
                    }, a === 'ban' ? 'Ban' : 'Mute'))),
                t.steps.length > 1 && button(null, {
                    small: true,
                    icon: 'x',
                    'aria-label': 'Remove step',
                    on: {
                        click: () => {
                            t.steps.splice(i, 1);
                            drawSteps();
                        }
                    }
                })),
            h('div', {class: 'chips'}, DURATIONS.map(([h0, label]) => h('button', {
                type: 'button',
                class: 'chip',
                'aria-pressed': String(s.hours === h0),
                on: {
                    click: () => {
                        s.hours = h0;
                        drawSteps();
                    }
                }
            }, label))),
            custom);
    }));
    drawSteps();
    const save = async (list) => {
        await api('POST', '/templates', {templates: list}).then((page) => {
            state.templates = page.templates;
        });
        onDone?.();
    };
    openDialog({
        title: index === null ? 'New template' : `Edit ${t.name}`,
        body: [
            h('label', {class: 'field'}, h('span', {}, 'Name'), name),
            h('label', {class: 'field'}, h('span', {}, 'Description'), description),
            h('div', {class: 'field'}, h('span', {}, 'Ladder'), steps,
                button('Add step', {
                    small: true, icon: 'plus', on: {
                        click: () => {
                            const last = t.steps[t.steps.length - 1];
                            t.steps.push({...last});
                            drawSteps();
                        }
                    }
                })),
            h('label', {class: 'field'}, h('span', {}, 'Offences count for (days)'), expire,
                h('p', {class: 'hint'}, 'Older offences of this template don\'t move them up the ladder.')),
        ],
        actions: [
            index !== null && {
                label: 'Delete', kind: 'danger', icon: 'trash', run: async () => {
                    await save(templates.filter((_, i) => i !== index));
                    toast(`Deleted ${t.name}.`);
                }
            },
            {label: 'Cancel'},
            {
                label: 'Save', kind: 'primary', run: async () => {
                    t.name = name.value.trim();
                    t.description = description.value.trim();
                    t.expire_days = Math.round(Number(expire.value));
                    const list = templates.slice();
                    if (index === null) list.push(t); else list[index] = t;
                    await save(list);
                    toast(`Saved ${t.name}.`);
                }
            },
        ].filter(Boolean),
    });
}

function templatesTab() {
    const list = spinnerList();
    const head = h('div', {class: 'card-head'}, h('h2', {}, 'Sanction templates'));
    const load = async () => {
        try {
            const page = await api('GET', '/templates');
            state.templates = page.templates;
            const edit = (i) => page.can_edit && button('Edit', {
                small: true,
                icon: 'edit',
                on: {click: () => templateDialog(page.templates, i, load)}
            });
            replace(head, h('h2', {}, 'Sanction templates'), h('div', {class: 'actions'},
                page.can_edit && button('New template', {
                    small: true,
                    icon: 'plus',
                    on: {click: () => templateDialog(page.templates, null, load)}
                })));
            replace(list, page.templates.length ? page.templates.map((t, i) => h('div', {class: 'list-item'},
                    h('div', {class: 'list-icon'}, icon('shield')),
                    h('div', {class: 'list-main'},
                        h('div', {class: 'list-title'}, h('b', {}, t.name)),
                        t.description && h('div', {class: 'list-sub'}, t.description),
                        ladder(t.steps, -1),
                        h('div', {class: 'list-sub'}, `Offences count for ${plural(t.expire_days, 'day')}.`)),
                    edit(i)))
                : h('div', {class: 'empty'}, icon('shield'), 'No templates.'));
        } catch (e) {
            failed(list, e);
        }
    };
    load();
    const canned = cannedCard();
    return {
        load: () => {
            load();
            canned.load();
        },
        el: [h('section', {class: 'card'}, head,
            h('div', {class: 'card-body'}, h('p', {class: 'hint'}, 'Pick one when you mute or ban: it sets the length from how many times the player has had it before, and logs the template as the reason.')),
            list),
            canned.el]
    };
}

function cannedDialog(all, index, onDone) {
    const t = index === null ? {id: '', name: '', use: 'close', resolution: 'no_action', text: ''} : structuredClone(all[index]);
    const name = h('input', {
        class: 'input',
        maxlength: '60',
        required: true,
        value: t.name,
        placeholder: 'e.g. Nothing found',
        autofocus: true
    });
    const text = h('textarea', {class: 'input', required: true, value: t.text, rows: '5'});
    const uses = h('div', {class: 'segmented', role: 'group', 'aria-label': 'Used for'});
    const outcome = h('div', {class: 'field'});
    const textHint = h('span', {class: 'hint'});
    const draw = () => {
        replace(uses, CANNED_USES.map(([k, label]) => h('button', {
            type: 'button', 'aria-pressed': String(t.use === k), on: {
                click: () => {
                    t.use = k;
                    if (k === 'close' && !t.resolution) t.resolution = 'no_action';
                    draw();
                }
            }
        }, label)));
        outcome.classList.toggle('hidden', t.use !== 'close');
        replace(outcome, h('span', {}, 'Outcome it picks'), h('div', {class: 'segmented', role: 'group', 'aria-label': 'Outcome'},
            Object.entries(RESOLUTIONS).map(([k, label]) => h('button', {
                type: 'button', 'aria-pressed': String(t.resolution === k), on: {
                    click: () => {
                        t.resolution = k;
                        draw();
                    }
                }
            }, label))));
        const close = t.use === 'close';
        text.maxLength = close ? 4000 : 2000;
        text.placeholder = close ? 'What was found and what was done.' : 'What the player is told, on the website\'s appeal status page.';
        textHint.textContent = `${close ? 'Markdown works.' : 'Plain text: the player sees it as written.'} ${CANNED_HINT}`;
    };
    draw();
    const save = async (list) => {
        state.canned = (await api('POST', '/canned', {canned: list})).canned;
        onDone?.();
    };
    openDialog({
        title: index === null ? 'New saved text' : `Edit ${t.name}`,
        body: [
            h('label', {class: 'field'}, h('span', {}, 'Name'), name),
            h('div', {class: 'field'}, h('span', {}, 'Used for'), uses),
            outcome,
            h('label', {class: 'field'}, h('span', {}, 'Text'), text, textHint),
        ],
        actions: [
            index !== null && {
                label: 'Delete', kind: 'danger', icon: 'trash', run: async () => {
                    await save(all.filter((_, i) => i !== index));
                    toast(`Deleted ${t.name}.`);
                }
            },
            {label: 'Cancel'},
            {
                label: 'Save', kind: 'primary', run: async () => {
                    t.name = name.value.trim();
                    t.text = text.value.trim();
                    if (t.use !== 'close') delete t.resolution;
                    const list = all.slice();
                    if (index === null) list.push(t); else list[index] = t;
                    await save(list);
                    toast(`Saved ${t.name}.`);
                }
            },
        ].filter(Boolean),
    });
}

// The saved closing notes and appeal decisions, by what they're for.
function cannedCard() {
    const list = spinnerList();
    const head = h('div', {class: 'card-head'}, h('h2', {}, 'Saved texts'));
    const load = async () => {
        try {
            const page = await api('GET', '/canned');
            state.canned = page.canned;
            replace(head, h('h2', {}, 'Saved texts'), h('div', {class: 'actions'},
                page.can_edit && button('New text', {
                    small: true,
                    icon: 'plus',
                    on: {click: () => cannedDialog(page.canned, null, load)}
                })));
            const row = (t, i) => h('div', {class: 'list-item'},
                h('div', {class: 'list-icon'}, icon(t.use === 'close' ? 'check' : 'flag')),
                h('div', {class: 'list-main'},
                    h('div', {class: 'list-title'}, h('b', {}, t.name), ' ',
                        t.use === 'close' && t.resolution && h('span', {class: 'badge'}, RESOLUTIONS[t.resolution])),
                    h('div', {class: 'list-sub canned-text'}, t.text)),
                page.can_edit && button('Edit', {
                    small: true,
                    icon: 'edit',
                    on: {click: () => cannedDialog(page.canned, i, load)}
                }));
            const groups = CANNED_USES.map(([use, label]) => {
                const rows = page.canned.map((t, i) => [t, i]).filter(([t]) => t.use === use);
                return rows.length > 0 && [h('div', {class: 'list-group'}, label), rows.map(([t, i]) => row(t, i))];
            }).filter(Boolean);
            replace(list, groups.length ? groups : h('div', {class: 'empty'}, icon('edit'), 'No saved texts.'));
        } catch (e) {
            failed(list, e);
        }
    };
    load();
    return {
        load, el: h('section', {class: 'card'}, head,
            h('div', {class: 'card-body'}, h('p', {class: 'hint'}, `Closing notes and appeal decisions to pick from instead of typing them out: they show above the note when you close a case or decide an appeal. ${CANNED_HINT}`)),
            list)
    };
}

// ---------------------------------------------------------------------------
// Staff notes on players: private, apart from cases. "Keep an eye on this
// one" notes put the player on the watchlist.

const watchBadge = () => h('span', {class: 'badge warn', title: 'On the staff watchlist'}, icon('eye'), 'Watched');

function noteItem(n, onRemove) {
    const mine = isMe(n.by);
    return h('div', {class: 'list-item'},
        h('div', {class: `list-icon ${n.watch ? 'warn' : ''}`}, icon(n.watch ? 'eye' : 'edit')),
        h('div', {class: 'list-main'},
            h('div', {class: 'list-title'}, h('b', {}, n.by), n.watch && [' ', watchBadge()]),
            h('div', {class: 'note-text'}, n.text)),
        h('div', {class: 'list-side stack'},
            h('span', {title: fullTime(n.at_ms)}, ago(n.at_ms)),
            (mine || state.canCommand) && button(null, {
                small: true, kind: 'ghost', icon: 'trash', 'aria-label': 'Remove note', title: 'Remove', on: {
                    click: () => confirmDialog({
                        title: 'Remove note',
                        subtitle: `On ${n.player}, by ${n.by}`,
                        body: [h('p', {class: 'muted'}, n.text)],
                        label: 'Remove', kind: 'danger',
                        run: async () => {
                            await api('POST', `/notes/${n.id}/remove`);
                            onRemove();
                        },
                    })
                }
            })));
}

// A player's notes, with a box to add one. `onWatch` hears whether they're
// on the watchlist, each time the notes load.
function notesSection(name, onWatch) {
    const list = spinnerList();
    const text = h('textarea', {
        class: 'input',
        maxlength: '1000',
        rows: '2',
        placeholder: 'A note for other staff: what to look out for, who they are, why. Only staff see it.',
        'aria-label': 'New note'
    });
    const watch = h('input', {type: 'checkbox'});
    const add = button('Add note', {small: true, kind: 'primary', icon: 'plus'});
    const load = async () => {
        try {
            const notes = await api('GET', `/notes?${query({player: name})}`);
            replace(list, notes.length ? notes.map((n) => noteItem(n, load)) : h('div', {class: 'empty'}, icon('edit'), 'No notes on them.'));
            onWatch?.(notes.some((n) => n.watch));
        } catch (e) {
            failed(list, e);
        }
    };
    add.addEventListener('click', async () => {
        if (!text.value.trim()) return text.focus();
        add.disabled = true;
        try {
            await api('POST', '/notes', {player: name, text: text.value, watch: watch.checked});
            text.value = '';
            watch.checked = false;
            load();
        } catch (e) {
            toast(e.message, 'error');
        } finally {
            add.disabled = false;
        }
    });
    load();
    return h('section', {class: 'card'},
        h('div', {class: 'card-head'}, h('h2', {}, 'Staff notes'), h('div', {class: 'actions'}, h('span', {class: 'hint hide-mobile'}, 'Private to staff, apart from cases'))),
        h('div', {class: 'card-body note-compose'}, text,
            h('div', {class: 'chips'}, h('label', {class: 'check-row'}, watch, 'Keep an eye on them (adds them to the watchlist)'), add)),
        list);
}

// ---------------------------------------------------------------------------
// Command palette (Ctrl/Cmd+K)

// What the console found for the palette's query (see /api/search), as
// palette items.
function foundItems(found) {
    const items = [];
    for (const p of found.players) {
        items.push({
            group: 'Players',
            label: p.name,
            icon: 'user',
            hint: p.online_world ? `Online on world ${p.online_world}` : p.registered ? 'Registered' : '',
            run: () => navigate(`/player?name=${encodeURIComponent(p.name)}`)
        });
    }
    for (const c of found.cases) {
        items.push({
            group: 'Cases',
            label: `#${c.id} ${c.title}`,
            icon: c.status === 'closed' ? 'check' : 'briefcase',
            hint: `${c.player} · ${CASE_STATUS[c.status]?.[0] || c.status}`,
            run: () => navigate(caseHref(c.id))
        });
    }
    for (const r of found.replays) {
        items.push({
            group: 'Replays',
            label: `Session of ${r.player}`,
            icon: r.score > 0 ? 'alert' : 'play',
            hint: `${shortTime(r.started_ms)} · ${clock(r.duration_ms / 1000)}`,
            run: () => navigate(replayHref(r))
        });
    }
    for (const e of found.audit) {
        const [verb] = VERBS[e.action] || [e.action.replace(/_/g, ' ')];
        items.push({
            group: 'Audit log',
            label: `${e.actor || 'Someone'} ${verb}${e.target ? ` ${e.target}` : ''}`,
            icon: 'list',
            hint: ago(e.at_ms),
            run: () => navigate(`/audit?${query({q: found.q})}`)
        });
    }
    return items;
}

function paletteItems(query, found) {
    const admin = state.canCommand;
    const {online, rebooting} = totals();
    const items = [
        ...navItems().map((n) => ({group: 'Go to', label: n.label, icon: n.icon, run: () => navigate(n.path)})),
        admin && {
            group: 'Actions',
            label: 'Broadcast to all worlds...',
            icon: 'megaphone',
            run: () => broadcastDialog(online)
        },
        admin && {
            group: 'Actions',
            label: 'Reboot all worlds...',
            icon: 'power',
            run: () => online.length && rebootDialog(online)
        },
        admin && rebooting.length && {
            group: 'Actions',
            label: 'Cancel every scheduled reboot...',
            icon: 'undo',
            run: () => cancelRebootDialog(rebooting)
        },
        admin && {
            group: 'Actions',
            label: 'Schedule a reboot or broadcast...',
            icon: 'clock',
            run: () => scheduleDialog('reboot')
        },
        {group: 'Actions', label: 'Kill switches and login lockdown', icon: 'zap', run: () => navigate('/controls')},
        {group: 'Actions', label: 'Kick a player...', icon: 'kick', run: () => moderateDialog('', 'kick')},
        {group: 'Actions', label: 'Mute a player...', icon: 'mute', run: () => moderateDialog('', 'mute')},
        !isTrial() && {group: 'Actions', label: 'Ban a player...', icon: 'ban', run: () => moderateDialog('', 'ban')},
        {group: 'Actions', label: 'Open a case...', icon: 'briefcase', run: () => newCaseDialog({})},
        {
            group: 'Actions',
            label: 'Triage reports and flagged sessions',
            icon: 'inbox',
            run: () => navigate('/moderation?tab=triage')
        },
        ...state.worlds.map((w) => ({
            group: 'Worlds',
            label: `${worldName(w)}${w.region ? ` · ${w.region}` : ''}`,
            hint: w.online ? `${w.status.players} players` : 'offline',
            icon: 'globe',
            run: () => (w.online ? worldPlayersDialog(w) : navigate('/worlds')),
        })),
    ].filter(Boolean);
    const q = query.trim().toLowerCase();
    const p = state.prefs || NO_PREFS;
    // Pinned and recent players first: all of them with no query, else those
    // that match.
    const known = [
        ...p.pinned.map((name) => ({group: 'Pinned', name})),
        ...p.recent.filter((r) => !isPinned(r.name)).map((r) => ({group: 'Recently viewed', name: r.name, hint: ago(r.at_ms)})),
    ].filter((k) => !q || nameKey(k.name).includes(nameKey(q)))
        .map((k) => ({group: k.group, label: k.name, hint: k.hint, icon: k.group === 'Pinned' ? 'star' : 'clock', run: () => navigate(`/player?name=${encodeURIComponent(k.name)}`)}));
    const ready = found && found.q === query.trim();
    const shownNames = new Set(known.map((k) => nameKey(k.label)));
    const results = ready ? foundItems(found).filter((i) => i.group !== 'Players' || !shownNames.has(nameKey(i.label))) : [];
    const pages = q ? items.filter((i) => i.label.toLowerCase().includes(q)) : items.filter((i) => i.group !== 'Worlds');
    const audit = results.filter((i) => i.group === 'Audit log');
    const matches = [...known.slice(0, q ? 6 : 10), ...results.filter((i) => i.group !== 'Audit log'), ...pages];

    const caseId = /^#?(\d{1,9})$/.exec(query.trim());
    if (caseId && !(ready && found.cases.some((c) => String(c.id) === caseId[1]))) matches.unshift({
        group: 'Cases',
        label: `Go to case #${caseId[1]}`,
        icon: 'briefcase',
        run: () => navigate(caseHref(caseId[1]))
    });
    if (q && /^[a-z0-9 _]{1,12}$/i.test(query.trim())) {
        matches.push({
            group: 'For this name',
            label: `Look up ${query.trim()}`,
            icon: 'user',
            run: () => navigate(`/player?name=${encodeURIComponent(query.trim())}`)
        });
        matches.push({
            group: 'For this name',
            label: `Find "${query.trim()}" online`,
            icon: 'search',
            run: () => navigate(`/players?q=${encodeURIComponent(query.trim())}`)
        });
        matches.push({
            group: 'For this name',
            label: `Chat logs of ${query.trim()}`,
            icon: 'chat',
            run: () => navigate(`/chat?name=${encodeURIComponent(query.trim())}`)
        });
        if (!isTrial()) matches.push({
            group: 'For this name',
            label: `Ban ${query.trim()} by template...`,
            icon: 'ban',
            run: () => moderateDialog(query.trim(), 'ban')
        });
        matches.push({
            group: 'For this name',
            label: `Cases on ${query.trim()}`,
            icon: 'briefcase',
            run: () => navigate(`/moderation?tab=cases&show=all&q=${encodeURIComponent(query.trim())}`)
        });
    }
    if (q && !ready) matches.push({group: 'Searching', label: 'Searching players, cases, replays and the audit log...', icon: 'search', run: () => {}, pending: true});
    if (q) {
        matches.push(...audit, {
            group: 'Audit log',
            label: `Search the whole audit log for "${query.trim()}"`,
            icon: 'list',
            run: () => navigate(`/audit?${new URLSearchParams({q: query.trim()})}`)
        });
    }
    return matches.slice(0, 60);
}

function openPalette() {
    if (state.session?.stage !== 'signed_in' || document.querySelector('dialog.palette')) return;
    const input = h('input', {
        placeholder: 'Search players, cases, replays, the audit log, pages and actions',
        'aria-label': 'Command',
        autocomplete: 'off',
        spellcheck: 'false'
    });
    const list = h('div', {class: 'palette-list', role: 'listbox'});
    const dialog = h('dialog', {class: 'palette', 'aria-label': 'Command palette'},
        h('div', {class: 'palette-input'}, icon('search'), input, h('kbd', {}, 'Esc')),
        list,
        h('div', {class: 'palette-foot hide-mobile'}, h('span', {}, h('kbd', {}, '↑'), ' ', h('kbd', {}, '↓'), ' to move'), h('span', {}, h('kbd', {}, 'Enter'), ' to run')));
    let items = [];
    let active = 0;
    // The console's search, for the query typed.
    let found = null;
    let seq = 0;
    const search = debounce(async () => {
        const q = input.value.trim();
        const mine = ++seq;
        if (!q) return;
        try {
            const result = await api('GET', `/search?${new URLSearchParams({q})}`);
            if (mine !== seq || !dialog.open) return;
            found = {...result, q};
        } catch {
            if (mine !== seq) return;
            found = {players: [], cases: [], replays: [], audit: [], q};
        }
        draw();
    }, 180);
    const run = (item) => {
        if (item.pending) return;
        dialog.close();
        item.run();
    };
    const draw = () => {
        items = paletteItems(input.value, found);
        active = Math.min(active, Math.max(0, items.length - 1));
        list.replaceChildren();
        let group = null;
        items.forEach((item, i) => {
            if (item.group !== group) {
                group = item.group;
                list.append(h('div', {class: 'palette-group'}, group));
            }
            list.append(h('button', {
                type: 'button', class: 'palette-item', role: 'option', 'aria-selected': String(i === active),
                on: {
                    click: () => run(item), mousemove: () => {
                        if (active !== i) {
                            active = i;
                            draw();
                        }
                    }
                },
            }, icon(item.icon), h('span', {}, item.label), item.hint && h('small', {}, item.hint)));
        });
        if (!items.length) list.append(h('div', {class: 'empty'}, 'Nothing matches.'));
        list.querySelector('[aria-selected="true"]')?.scrollIntoView({block: 'nearest'});
    };
    input.addEventListener('input', () => {
        active = 0;
        draw();
        search();
    });
    input.addEventListener('keydown', (e) => {
        if (e.key === 'ArrowDown') {
            e.preventDefault();
            active = (active + 1) % Math.max(1, items.length);
            draw();
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            active = (active - 1 + items.length) % Math.max(1, items.length);
            draw();
        } else if (e.key === 'Enter' && items[active]) {
            e.preventDefault();
            run(items[active]);
        }
    });
    dialog.addEventListener('close', () => dialog.remove());
    dialog.addEventListener('click', (e) => {
        if (e.target === dialog) dialog.close();
    });
    document.body.append(dialog);
    draw();
    dialog.showModal();
    input.focus();
}

document.addEventListener('keydown', (e) => {
    const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement?.tagName);
    if ((e.key === 'k' && (e.metaKey || e.ctrlKey)) || (e.key === '/' && !typing && !document.querySelector('dialog[open]'))) {
        e.preventDefault();
        openPalette();
    }
});

// ---------------------------------------------------------------------------
// Start

(async () => {
    try {
        state.session = await api('GET', '/session');
    } catch (e) {
        state.session = {stage: 'signed_out'};
        toast(e.message, 'error');
    }
    // Kept recordings are only for a signed-in visit: a session that ended
    // while the console was closed leaves none behind.
    if (state.session?.stage === 'signed_in') trimReplayStore();
    else if (state.session?.stage === 'signed_out') clearReplayStore();
    render();
})();
