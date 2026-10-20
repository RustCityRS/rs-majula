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
  clip: ['M20 11.5 12 19.5a5 5 0 0 1-7-7l8.5-8.5a3.3 3.3 0 0 1 4.7 4.7L9.7 17.2a1.7 1.7 0 0 1-2.4-2.4L15 7'],
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
  const { icon: iconName, kind, small, ...rest } = props;
  const classes = ['btn', kind, small && 'small', !label && 'icon-only'].filter(Boolean).join(' ');
  return h('button', { type: 'button', class: classes, ...rest }, iconName && icon(iconName), label);
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
  return new Date(ms).toLocaleDateString();
}

const fullTime = (ms) => new Date(ms).toLocaleString();
const shortTime = (ms) => {
  const d = new Date(ms);
  const sameDay = d.toDateString() === new Date().toDateString();
  return sameDay ? d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
    : d.toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
};

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
const playerLink = (name, props = {}) => link(`/player?name=${encodeURIComponent(name)}`, { class: 'name-link', ...props }, name);

function device(ua) {
  const browser = /Edg\//.test(ua) ? 'Edge' : /OPR\//.test(ua) ? 'Opera' : /Firefox\//.test(ua) ? 'Firefox'
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

// ---------------------------------------------------------------------------
// Preferences (per browser; the page works without them)

const prefs = {
  get(key, fallback) {
    try { return localStorage.getItem(`rs-admin:${key}`) ?? fallback; } catch { return fallback; }
  },
  set(key, value) {
    try { localStorage.setItem(`rs-admin:${key}`, value); } catch { /* storage blocked */ }
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
  worlds: [],
  canCommand: false,
  worldsLoaded: false,
  caseSummary: null,
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
  try { data = await res.json(); } catch { /* not JSON */ }
  if (!res.ok) {
    if (res.status === 401 && state.session?.stage === 'signed_in' && !path.startsWith('/auth/')) {
      sessionEnded();
    }
    throw new ApiError(data?.error || `Request failed (${res.status})`, res.status);
  }
  return data;
}

function sessionEnded() {
  state.session = { stage: 'signed_out' };
  stopPolling();
  toast('Your session ended. Sign in again.', 'error');
  render();
}

// ---------------------------------------------------------------------------
// Toasts

const toasts = h('div', { class: 'toasts', role: 'status', 'aria-live': 'polite' });
document.body.append(toasts);

function toast(message, kind = 'ok') {
  const el = h('div', { class: `toast ${kind}` }, icon(kind === 'error' ? 'alert' : 'check'), h('div', {}, message));
  toasts.append(el);
  setTimeout(() => {
    el.classList.add('leaving');
    setTimeout(() => el.remove(), 220);
  }, kind === 'error' ? 6000 : 3500);
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

function openDialog({ title, subtitle, body = [], actions = [], className, onClose, width }) {
  const error = h('p', { class: 'error-text', role: 'alert' });
  const foot = h('div', { class: 'dialog-foot' });
  const dialog = h('dialog', { class: className, 'aria-label': title });
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
    const btn = button(action.label, { kind: action.kind, icon: action.icon });
    btn.addEventListener('click', async () => {
      if (!action.run) return close();
      // Nothing is sent until the form is filled out.
      error.textContent = firstProblem(dialog) ?? '';
      if (error.textContent) return;
      const all = foot.querySelectorAll('button');
      all.forEach((b) => (b.disabled = true));
      const label = btn.textContent;
      btn.replaceChildren(h('span', { class: 'spinner' }), label);
      try {
        const keepOpen = await action.run({ close, error });
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
    h('div', { class: 'dialog-form' },
      h('div', { class: 'dialog-head' },
        h('div', { class: 'grow' }, h('h2', {}, title), subtitle && h('p', {}, subtitle)),
        button(null, { kind: 'ghost', icon: 'x', 'aria-label': 'Close', on: { click: close } })),
      h('div', { class: 'dialog-body' }, body, error),
      actions.length ? foot : null),
  );
  document.body.append(dialog);
  dialog.showModal();
  dialog.querySelector('[autofocus]')?.focus();
  return { dialog, close, error };
}

function confirmDialog({ title, subtitle, body, label, kind = 'primary', run }) {
  return openDialog({
    title,
    subtitle,
    body,
    actions: [{ label: 'Cancel' }, { label, kind, run }],
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
  caseTimer = setInterval(() => {
    if (!document.hidden) loadCaseSummary();
  }, 30000);
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
  return h('span', { class: 'countdown', dataset: { deadline: String(deadline) } }, left > 0 ? clock(left) : 'now');
}

// ---------------------------------------------------------------------------
// World commands

function commandResults(title, replies, retry) {
  const failed = replies.filter((r) => r.status !== 'answered' || !r.answer.ok);
  const ok = replies.length - failed.length;
  const rows = replies.map((r) => {
    const good = r.status === 'answered' && r.answer.ok;
    const message = r.status === 'answered' ? r.answer.message : r.error;
    return h('div', { class: `result ${good ? 'ok' : 'fail'}` },
      icon(good ? 'check' : 'x'), h('b', {}, `World ${r.world}`), h('span', {}, message));
  });
  const actions = [{ label: 'Done', kind: failed.length ? undefined : 'primary' }];
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
    body: [h('div', { class: 'results' }, rows)],
    actions,
  });
}

async function sendCommand(path, body, title) {
  const run = async (nodes) => {
    const result = await api('POST', path, { ...body, nodes });
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

function rebootDialog(worlds, onDone) {
  const presets = [[60, '1 min'], [300, '5 min'], [600, '10 min'], [1800, '30 min'], [3600, '1 hour']];
  const staggers = [[0, 'None'], [15, '15s'], [30, '30s'], [60, '1 min'], [300, '5 min']];
  let seconds = 300;
  let stagger = 0;
  const many = worlds.length > 1;

  const custom = h('input', { class: 'input', type: 'number', min: '0', max: '600', step: '1', placeholder: 'Minutes', inputmode: 'numeric', 'data-invalid': 'Countdowns are 0 to 600 whole minutes.' });
  const delayChips = h('div', { class: 'chips' });
  const staggerChips = h('div', { class: 'chips' });
  const timeline = h('div', { class: 'timeline' });
  const reason = h('input', { class: 'input', maxlength: '80', placeholder: 'Optional, e.g. "update"', autocomplete: 'off' });
  const preview = h('p', { class: 'hint' });
  const confirmText = `reboot ${worlds.length} worlds`;
  const confirmInput = h('input', { class: 'input', placeholder: confirmText, autocomplete: 'off', spellcheck: 'false' });
  let action;

  const update = () => {
    replace(delayChips, presets.map(([s, label]) =>
      h('button', { type: 'button', class: 'chip', 'aria-pressed': String(seconds === s && !custom.value), on: { click: () => { custom.value = ''; seconds = s; update(); } } }, label)));
    replace(staggerChips, staggers.map(([s, label]) =>
      h('button', { type: 'button', class: 'chip', 'aria-pressed': String(stagger === s), on: { click: () => { stagger = s; update(); } } }, label)));
    const shown = worlds.slice(0, 50);
    replace(timeline, shown.map((w, i) => h('div', {}, h('span', {}, worldName(w)), h('span', { class: 'num' }, `in ${clock(seconds + stagger * i)}`))),
      worlds.length > shown.length && h('div', { class: 'muted' }, `and ${worlds.length - shown.length} more`));
    preview.textContent = reason.value.trim()
      ? `Players also get the message: "Rebooting in ${clock(seconds)}: ${reason.value.trim()}"`
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
    label: many ? `Reboot ${worlds.length} worlds` : `Reboot ${worldName(worlds[0])}`,
    kind: 'danger',
    icon: 'power',
    refresh: (btn) => {
      btn.disabled = many && confirmInput.value.trim().toLowerCase() !== confirmText;
    },
    run: async () => {
      await sendCommand('/worlds/reboot', { nodes: worlds.map((w) => w.node), seconds, stagger_secs: many ? stagger : 0, reason: reason.value.trim() || null }, 'Reboot scheduled');
      onDone?.();
    },
  };

  openDialog({
    title: many ? `Reboot ${worlds.length} worlds` : `Reboot ${worldName(worlds[0])}`,
    subtitle: many ? targetsSummary(worlds) : worlds[0].region,
    body: [
      h('div', { class: 'field' }, h('span', {}, 'Countdown'), delayChips, custom),
      many && h('div', { class: 'field' }, h('span', {}, 'Stagger'), staggerChips,
        h('p', { class: 'hint' }, 'Spaces the worlds out so they don\'t all go down, and fill back up, at once.')),
      h('div', { class: 'field' }, h('span', {}, 'Schedule'), timeline),
      h('label', { class: 'field' }, h('span', {}, 'Reason'), reason, preview),
      h('div', { class: 'callout warn' }, icon('alert'),
        h('div', {}, 'Players see the countdown, then are saved and logged out. The world process exits when it ends; your service manager starts it again.')),
      many && h('label', { class: 'field' }, h('span', {}, `Type "${confirmText}" to confirm`), confirmInput),
    ],
    actions: [{ label: 'Cancel' }, action],
  });
}

function cancelRebootDialog(worlds, onDone) {
  const pending = worlds.filter(isRebooting);
  const targets = pending.length ? pending : worlds;
  confirmDialog({
    title: 'Cancel reboot',
    subtitle: targetsSummary(targets),
    body: [h('p', { class: 'muted' }, pending.length
      ? `Stops the countdown on ${plural(pending.length, 'world')}. Worlds already logging players out can't be stopped.`
      : 'None of these worlds has a reboot scheduled that the console knows of. Send the cancel anyway?')],
    label: 'Cancel reboot',
    run: async () => {
      await sendCommand('/worlds/cancel-reboot', { nodes: targets.map((w) => w.node) }, 'Reboot cancelled');
      onDone?.();
    },
  });
}

function broadcastDialog(worlds, onDone) {
  const max = 400;
  const text = h('textarea', { class: 'input', maxlength: String(max), placeholder: 'The game will be updating shortly...', required: true, autofocus: true });
  const counter = h('span', { class: 'hint num' }, `0 / ${max}`);
  text.addEventListener('input', () => {
    counter.textContent = `${text.value.length} / ${max}`;
  });
  const action = {
    label: 'Send broadcast',
    kind: 'primary',
    icon: 'megaphone',
    run: async () => {
      await sendCommand('/worlds/broadcast', { nodes: worlds.map((w) => w.node), message: text.value.trim() }, 'Broadcast sent');
      onDone?.();
    },
  };
  openDialog({
    title: 'Broadcast',
    subtitle: `To everyone on ${targetsSummary(worlds).replace(/^All/, 'all')}`,
    body: [h('label', { class: 'field' }, h('span', {}, 'Message'), text, counter)],
    actions: [{ label: 'Cancel' }, action],
  });
}

function maintenanceDialog(worlds, onDone) {
  const open = worlds.filter((w) => !w.status?.maintenance);
  let enabled = open.length > 0;
  const choice = h('div', { class: 'segmented', role: 'group', 'aria-label': 'Maintenance' });
  const effect = h('div', { class: 'callout' });
  let action;
  const update = () => {
    replace(choice,
      h('button', { type: 'button', 'aria-pressed': String(enabled), on: { click: () => { enabled = true; update(); } } }, 'Turn on'),
      h('button', { type: 'button', 'aria-pressed': String(!enabled), on: { click: () => { enabled = false; update(); } } }, 'Turn off'));
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
      await sendCommand('/worlds/maintenance', { nodes: worlds.map((w) => w.node), enabled }, enabled ? 'Maintenance on' : 'Maintenance off');
      onDone?.();
    },
  };
  update();
  openDialog({
    title: 'Maintenance mode',
    subtitle: targetsSummary(worlds),
    body: [choice, effect],
    actions: [{ label: 'Cancel' }, action],
  });
  update();
}

// ---------------------------------------------------------------------------
// Moderation

const DURATIONS = [[1, '1 hour'], [24, '1 day'], [48, '2 days'], [168, '1 week'], [720, '30 days'], [8760, '1 year'], [87600, 'Permanent']];

// `caseId`: the case it's sent from, whose timeline notes it.
function moderateDialog(name, preset = 'kick', onDone, caseId) {
  let action = preset;
  let hours = 48;
  const nameInput = h('input', { class: 'input', value: name || '', placeholder: 'Player name', maxlength: '12', pattern: NAME_PATTERN, 'data-invalid': NAME_RULE, required: true, autocomplete: 'off', spellcheck: 'false', autofocus: !name });
  const reason = h('textarea', { class: 'input', placeholder: 'Optional. Saved in the audit log.', maxlength: '300', value: caseId ? `Case #${caseId}: ` : '' });
  const segmented = h('div', { class: 'segmented', role: 'group', 'aria-label': 'Action' });
  const durationField = h('div', { class: 'field' });
  const custom = h('input', { class: 'input', type: 'number', min: '1', max: '87600', step: '1', placeholder: 'Custom length in hours', inputmode: 'numeric', 'data-invalid': 'Lengths are 1 to 87600 whole hours.' });
  const effect = h('div', { class: 'callout' });
  const ACTIONS = { kick: 'Kick', mute: 'Mute', ban: 'Ban', unmute: 'Unmute', unban: 'Unban' };
  let confirm;

  const length = () => DURATIONS.find(([h]) => h === hours)?.[1].toLowerCase() ?? `${hours} hours`;
  const update = () => {
    replace(segmented, Object.entries(ACTIONS).map(([key, label]) =>
      h('button', { type: 'button', 'aria-pressed': String(action === key), on: { click: () => { action = key; update(); } } }, label)));
    const timed = action === 'mute' || action === 'ban';
    durationField.classList.toggle('hidden', !timed);
    replace(durationField, h('span', {}, 'Length'), h('div', { class: 'chips' }, DURATIONS.map(([h0, label]) =>
      h('button', { type: 'button', class: 'chip', 'aria-pressed': String(hours === h0 && !custom.value), on: { click: () => { custom.value = ''; hours = h0; update(); } } }, label))), custom);
    const who = nameInput.value.trim() || 'They';
    const effects = {
      kick: `${who} is logged out of whichever world they're on. They can log straight back in.`,
      mute: `${who} can't use public or private chat for ${length()}.`,
      ban: `${who} is logged out now and can't log in for ${length()}.`,
      unmute: `${who} can chat again straight away.`,
      unban: `${who} can log in again straight away.`,
    };
    effect.className = `callout ${action === 'ban' ? 'danger' : ''}`;
    replace(effect, icon(action === 'ban' ? 'ban' : 'alert'), h('div', {}, effects[action]));
    if (confirm?.button) {
      replace(confirm.button, ACTIONS[action] + (nameInput.value.trim() ? ` ${nameInput.value.trim()}` : ''));
      confirm.button.className = `btn ${action === 'ban' || action === 'kick' ? 'danger' : 'primary'}`;
    }
  };
  custom.addEventListener('input', () => {
    const value = Math.round(Number(custom.value));
    if (custom.value && value >= 1) hours = value;
    update();
  });
  nameInput.addEventListener('input', update);

  confirm = {
    label: 'Confirm',
    kind: 'danger',
    run: async () => {
      const target = nameInput.value.trim();
      // "Case #12: " left as it was is just "Case #12".
      const why = reason.value.trim().replace(/:$/, '');
      const result = action === 'kick'
        ? await api('POST', '/players/kick', { name: target, reason: why, case: caseId ?? null })
        : await api('POST', '/players/moderate', { name: target, action, hours: action === 'mute' || action === 'ban' ? hours : null, reason: why, case: caseId ?? null });
      const ok = action === 'kick' ? result.kicked_from.length > 0 : result.ok;
      toast(result.message, ok ? 'ok' : 'error');
      if (ok) onDone?.();
      return !ok;
    },
  };
  update();
  openDialog({
    title: 'Moderate player',
    subtitle: caseId ? `From case #${caseId}: noted in its timeline.` : name ? `Acting on ${name}` : 'Online or offline: it applies wherever they are.',
    body: [
      h('label', { class: 'field' }, h('span', {}, 'Player'), nameInput),
      segmented,
      durationField,
      effect,
      h('label', { class: 'field' }, h('span', {}, 'Reason'), reason),
    ],
    actions: [{ label: 'Cancel' }, confirm],
  });
  update();
}

// ---------------------------------------------------------------------------
// Routing

const NAV = [
  { path: '/', label: 'Overview', icon: 'home', page: overviewPage },
  { path: '/worlds', label: 'Worlds', icon: 'globe', page: worldsPage },
  { path: '/players', label: 'Players', icon: 'users', page: playersPage },
  { path: '/moderation', label: 'Moderation', icon: 'shield', page: moderationPage },
  { path: '/chat', label: 'Chat logs', icon: 'chat', page: chatPage },
  { path: '/replays', label: 'Replays', icon: 'play', page: replaysPage },
  { path: '/audit', label: 'Audit log', icon: 'list', page: auditPage },
  { path: '/news', label: 'News', icon: 'news', page: newsPage, developer: true },
  { path: '/staff', label: 'Staff', icon: 'key', page: staffPage, developer: true },
  { path: '/account', label: 'Account', icon: 'user', page: accountPage },
];
// Pages reached from others, shown under their parent in the nav.
const PAGES = [
  ...NAV,
  { path: '/player', label: 'Player', page: playerPage, parent: '/players' },
  { path: '/replay', label: 'Replay', page: replayPage, parent: '/replays' },
  { path: '/case', label: 'Case', page: casePage, parent: '/moderation' },
];
// The first few fit the phone tab bar; the rest go under More.
const TABS = 4;

const isDeveloper = () => state.session?.role === 'developer';
const navItems = () => NAV.filter((n) => !n.developer || isDeveloper());

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

const link = (href, props, ...children) => h('a', { href, 'data-link': true, ...props }, ...children);

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
    state.canCommand = state.session.role === 'admin' || state.session.role === 'developer';
    renderShell();
    renderPage();
    startPolling();
  } else {
    stopPolling();
    replace(app, authView());
    app.querySelector('[autofocus]')?.focus();
  }
}

function themeButton() {
  const icons = { auto: 'monitor', light: 'sun', dark: 'moon' };
  const next = { auto: 'light', light: 'dark', dark: 'auto' };
  const btn = button(null, { kind: 'ghost', 'aria-label': 'Theme' });
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

async function signOut() {
  try { await api('POST', '/auth/logout'); } catch { /* signed out regardless */ }
  state.session = { stage: 'signed_out' };
  history.replaceState(null, '', '/');
  render();
}

function renderShell() {
  const { name, role } = state.session;
  const navLinks = navItems().map((item) => link(item.path, { dataset: { nav: item.path } },
    icon(item.icon), h('span', {}, item.label), h('span', { class: 'count num', dataset: { count: item.path } })));
  const sidebar = h('aside', { class: 'sidebar' },
    h('div', { class: 'brand' }, h('img', { src: '/icon.svg', alt: '' }), h('div', {}, 'RS Admin', h('small', {}, 'Staff console'))),
    h('button', { type: 'button', class: 'palette-trigger', on: { click: openPalette } }, icon('search'), h('span', {}, 'Search'), h('kbd', {}, navigator.platform.includes('Mac') ? '⌘K' : 'Ctrl K')),
    h('nav', { class: 'nav', 'aria-label': 'Main' }, navLinks),
    h('div', { class: 'sidebar-foot' },
      h('div', { class: 'me' },
        h('div', { class: 'avatar' }, initial(name)),
        h('div', { class: 'grow' }, h('b', {}, name), h('small', {}, role)),
        themeButton(),
        button(null, { kind: 'ghost', icon: 'logout', 'aria-label': 'Sign out', title: 'Sign out', on: { click: signOut } }))));

  const topbar = h('header', { class: 'topbar' },
    h('img', { src: '/icon.svg', alt: '' }), h('b', {}, 'RS Admin'),
    h('div', { class: 'actions' },
      button(null, { kind: 'ghost', icon: 'search', 'aria-label': 'Search', on: { click: openPalette } }),
      themeButton()));

  const more = h('button', { type: 'button', class: 'tab-more', dataset: { more: '' }, on: { click: moreSheet } }, icon('more'), h('span', {}, 'More'));
  const tabbar = h('nav', { class: 'tabbar', 'aria-label': 'Main' },
    navItems().slice(0, TABS).map((item) => link(item.path, { dataset: { nav: item.path } }, icon(item.icon), h('span', {}, item.short || item.label))),
    more);

  const main = h('main', { class: 'main', id: 'main' });
  shell = { main };
  replace(app, h('div', { class: 'shell' }, sidebar, topbar, main, tabbar));
  updateNavCounts();
}

function moreSheet() {
  const { close } = openDialog({
    title: 'More',
    body: [h('nav', { class: 'nav', 'aria-label': 'More pages' },
      navItems().slice(TABS).map((item) => link(item.path, { on: { click: () => close() }, 'aria-current': location.pathname === item.path ? 'page' : null },
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
  const { online, players } = totals();
  const counts = {
    '/worlds': state.worldsLoaded ? `${online.length}/${state.worlds.length}` : '',
    '/players': state.worldsLoaded ? players.toLocaleString() : '',
    // Open cases, with how many are yours first when any are.
    '/moderation': moderationCount(),
  };
  for (const el of document.querySelectorAll('[data-count]')) el.textContent = counts[el.dataset.count] || '';
}

function renderPage() {
  if (!shell) return render();
  for (const fn of pageCleanup) fn();
  pageCleanup = [];
  const path = location.pathname;
  const allowed = PAGES.filter((n) => !n.developer || isDeveloper());
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
  const ctx = {
    cleanup: (fn) => pageCleanup.push(fn),
    params: new URLSearchParams(location.search),
  };
  replace(shell.main, item.page(ctx));
  window.scrollTo(0, 0);
}

function pageHead(title, subtitle, ...actions) {
  return h('div', { class: 'page-head' },
    h('div', {}, h('h1', {}, title), subtitle && h('p', {}, subtitle)),
    actions.length ? h('div', { class: 'actions' }, actions) : null);
}

// ---------------------------------------------------------------------------
// Sign-in

function authView() {
  const stage = state.session?.stage || 'signed_out';
  const card = h('div', { class: 'auth-card' });
  const error = h('p', { class: 'error-text', role: 'alert' });

  const header = (title, text) => h('header', {}, h('img', { src: '/icon.svg', alt: '' }), h('h1', {}, title), h('p', {}, text));

  const submitting = (form, fn) => form.addEventListener('submit', async (e) => {
    e.preventDefault();
    const submit = form.querySelector('button[type=submit]');
    const label = submit.textContent;
    submit.disabled = true;
    submit.replaceChildren(h('span', { class: 'spinner' }), label);
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

  const startOver = h('p', { class: 'auth-foot' }, h('button', { type: 'button', on: { click: signOut } }, 'Use a different account'));

  const codeInput = () => {
    const input = h('input', { class: 'input code', name: 'code', inputmode: 'numeric', autocomplete: 'one-time-code', maxlength: '6', pattern: '[0-9]{6}', placeholder: '000000', required: true, autofocus: true, 'aria-label': 'Authenticator code' });
    input.addEventListener('input', () => {
      input.value = input.value.replace(/\D/g, '').slice(0, 6);
      if (input.value.length === 6) input.form.requestSubmit();
    });
    return input;
  };

  const signedIn = (result) => {
    state.session = { stage: 'signed_in', name: result.name, role: result.role, csrf: result.csrf };
    render();
  };

  if (stage === 'signed_out') {
    const name = h('input', { class: 'input', name: 'username', autocomplete: 'username', required: true, autofocus: true, maxlength: '64' });
    const password = h('input', { class: 'input', type: 'password', name: 'password', autocomplete: 'current-password', required: true, maxlength: '256' });
    const form = h('form', {},
      h('label', { class: 'field' }, h('span', {}, 'Name'), name),
      h('label', { class: 'field' }, h('span', {}, 'Password'), password),
      error,
      h('button', { type: 'submit', class: 'btn primary block' }, 'Continue'));
    submitting(form, async () => {
      const result = await api('POST', '/auth/login', { name: name.value, password: password.value });
      state.session = { stage: result.stage, name: result.name, csrf: result.csrf };
      render();
    });
    append(card, header('Staff console', 'Sign in to manage the worlds.'), form,
      h('p', { class: 'auth-foot' }, 'No account? Ask a developer to add you.'));
  } else if (stage === 'totp') {
    const code = codeInput();
    const form = h('form', {}, code, error, h('button', { type: 'submit', class: 'btn primary block' }, 'Verify'));
    submitting(form, async () => signedIn(await api('POST', '/auth/totp', { code: code.value })));
    append(card, header('Two-factor authentication', `Enter the 6-digit code from your authenticator app for ${state.session.name}.`), form, startOver);
  } else if (stage === 'enroll') {
    const code = codeInput();
    const qr = h('div', { class: 'qr' });
    const key = h('span', {}, '...');
    const copy = button(null, { kind: 'ghost', small: true, icon: 'copy', 'aria-label': 'Copy key', title: 'Copy key' });
    copy.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(key.textContent.replace(/\s/g, ''));
        toast('Key copied.');
      } catch { toast('Copy the key by hand.', 'error'); }
    });
    api('GET', '/auth/enroll').then((data) => {
      key.textContent = data.key;
      qr.append(h('img', { src: `/api/auth/enroll/qr.svg?t=${Date.now()}`, alt: 'QR code for your authenticator app' }));
    }).catch((e) => { error.textContent = e.message; });
    const form = h('form', {}, code, error, h('button', { type: 'submit', class: 'btn primary block' }, 'Turn on and sign in'));
    submitting(form, async () => signedIn(await api('POST', '/auth/enroll', { code: code.value })));
    append(card,
      header('Set up two-factor authentication', 'Every console account needs an authenticator app.'),
      h('ol', { class: 'steps' },
        h('li', {}, 'Open an authenticator app (1Password, Google Authenticator, Authy...).'),
        h('li', {}, 'Scan this code, or enter the key by hand.'),
        h('li', {}, 'Type the 6-digit code it shows.')),
      qr,
      h('div', { class: 'secret' }, key, copy),
      form,
      startOver);
  }
  return h('div', { class: 'auth' }, card);
}

// ---------------------------------------------------------------------------
// Overview

function overviewPage(ctx) {
  const admin = state.canCommand;
  const stats = h('div', { class: 'stats' });
  const bars = h('div', { class: 'bars' });
  const activity = h('div', { class: 'list' });

  const stat = (iconName, label, value, sub) => h('div', { class: 'card stat' },
    h('div', { class: 'label' }, icon(iconName), label), h('div', { class: 'value num' }, value), h('div', { class: 'sub' }, sub));

  const draw = (worlds) => {
    if (!state.worldsLoaded) return;
    const { online, players, rebooting } = totals();
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
      return h('div', { class: 'bar-row' },
        link('/worlds', {}, h('b', {}, worldName(w))),
        h('div', { class: `meter ${w.online ? '' : 'offline'}` }, fill),
        h('span', { class: 'num muted' }, w.online ? w.status.players.toLocaleString() : 'offline'));
    }) : h('div', { class: 'empty' }, 'No worlds configured.'));
  };

  const loadActivity = async () => {
    try {
      const entries = await api('GET', '/audit?limit=8');
      replace(activity, entries.length ? entries.map(auditItem) : h('div', { class: 'empty' }, icon('list'), 'Nothing yet.'));
    } catch (e) {
      replace(activity, h('div', { class: 'empty' }, e.message));
    }
  };
  loadActivity();
  const timer = setInterval(() => { if (!document.hidden) loadActivity(); }, 15000);
  ctx.cleanup(() => clearInterval(timer));
  ctx.cleanup(onWorlds(draw));
  draw(state.worlds);

  const hour = new Date().getHours();
  const greeting = hour < 12 ? 'Good morning' : hour < 18 ? 'Good afternoon' : 'Good evening';
  return h('div', { class: 'page' },
    pageHead('Overview', `${greeting}, ${state.session.name}.`,
      admin && button('Broadcast', { icon: 'megaphone', on: { click: () => broadcastDialog(totals().online) } }),
      admin && button('Reboot worlds', { icon: 'power', on: { click: () => navigate('/worlds') } })),
    stats,
    h('div', { class: 'split' },
      h('section', { class: 'card' },
        h('div', { class: 'card-head' }, h('h2', {}, 'Population'), h('div', { class: 'actions' }, link('/worlds', { class: 'btn small ghost' }, 'All worlds', icon('chevron')))),
        h('div', { class: 'card-body' }, bars)),
      h('section', { class: 'card' },
        h('div', { class: 'card-head' }, h('h2', {}, 'Recent activity'), h('div', { class: 'actions' }, link('/audit', { class: 'btn small ghost' }, 'Audit log', icon('chevron')))),
        activity)));
}

// ---------------------------------------------------------------------------
// Worlds

function worldsPage(ctx) {
  const selected = new Set();
  let filter = ctx.params.get('filter') || 'all';
  let query = '';
  const tiles = h('div', { class: 'tiles' });
  const subtitle = h('p');
  const filters = h('div', { class: 'chips' });
  const countLabel = h('span', { class: 'count' });
  const bar = h('div', { class: 'selection-bar', role: 'toolbar', 'aria-label': 'Selected worlds' });

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
  const clear = () => { selected.clear(); draw(); };

  const tile = (w) => {
    const status = w.status;
    const pressed = selected.has(w.node);
    const fill = h('i');
    fill.style.setProperty('width', `${status ? Math.min(100, (status.players / 2000) * 100) : 0}%`);
    const meta = w.online
      ? [h('span', { class: 'dot ok' }), 'Online',
        h('span', { class: 'sep' }), `${w.latency_ms} ms`,
        h('span', { class: 'sep' }), `up ${span(status.uptime_secs)}`]
      : [h('span', { class: 'dot danger' }), w.error || 'Offline'];
    const el = h('div', {
      class: `tile ${w.online ? '' : 'offline'}`,
      role: state.canCommand ? 'button' : null,
      tabindex: '0',
      dataset: { node: String(w.node) },
      'aria-pressed': state.canCommand ? String(pressed) : null,
      'aria-label': `${worldName(w)}, ${w.online ? `${status.players} players` : 'offline'}`,
    },
      h('div', { class: 'tile-top' },
        h('div', {}, h('div', { class: 'tile-num' }, w.id, h('small', {}, `node ${w.node}`)), h('div', { class: 'tile-region' }, w.region || 'No region')),
        state.canCommand && h('span', { class: 'check' }, icon('check'))),
      status
        ? h('div', { class: 'tile-stats' }, h('b', { class: 'num' }, status.players.toLocaleString()), h('span', { class: 'muted' }, 'players'))
        : h('div', { class: 'tile-stats' }, h('b', { class: 'muted' }, 'Offline')),
      h('div', { class: `meter ${w.online ? '' : 'offline'}` }, fill),
      h('div', { class: 'tile-meta' }, meta),
      status?.health && healthLine(status.health),
      (status?.shutting_down || w.rebootAt || status?.maintenance) && h('div', { class: 'tile-badges' },
        status?.shutting_down
          ? h('span', { class: 'badge danger' }, icon('power'), 'Shutting down')
          : w.rebootAt && h('span', { class: 'badge warn', title: status.reboot_reason || '' }, icon('clock'), 'Reboot in ', countdown(w.rebootAt)),
        status?.maintenance && h('span', { class: 'badge warn' }, icon('lock'), 'Maintenance')),
      status?.reboot_reason && w.rebootAt && h('div', { class: 'tile-reason' }, status.reboot_reason),
      w.online && button(null, { kind: 'ghost', small: true, icon: 'users', class: 'btn ghost small icon-only tile-link', 'aria-label': `Players on ${worldName(w)}`, title: 'Players', on: { click: (e) => { e.stopPropagation(); worldPlayersDialog(w); } } }));
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
      if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); activate(); }
    });
    return el;
  };

  const draw = () => {
    const shown = visible();
    for (const node of [...selected]) if (!state.worlds.some((w) => w.node === node)) selected.delete(node);
    const { online, players } = totals();
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
    replace(filters, Object.entries({ all: 'All', online: 'Online', offline: 'Offline', rebooting: 'Rebooting', maintenance: 'Maintenance' })
      .filter(([key]) => key !== 'maintenance' || counts.maintenance > 0).map(([key, label]) =>
      h('button', { type: 'button', class: 'chip', 'aria-pressed': String(filter === key), on: { click: () => { filter = key; draw(); } } }, `${label} `, h('span', { class: 'num muted' }, counts[key]))));
    // Redrawn every poll: keep keyboard focus on the same world.
    const focused = document.activeElement?.closest?.('.tile')?.dataset.node;
    replace(tiles, !state.worldsLoaded
      ? h('div', { class: 'empty' }, h('div', { class: 'spinner' }))
      : shown.length ? shown.map(tile) : h('div', { class: 'empty' }, icon('globe'), 'No worlds match.'));
    if (focused) tiles.querySelector(`[data-node="${focused}"]`)?.focus();
    updateBar();
  };

  const updateBar = () => {
    const picked = chosen();
    countLabel.textContent = `${plural(picked.length, 'world')} selected`;
    bar.classList.toggle('open', picked.length > 0);
  };

  const search = h('input', { class: 'input', type: 'search', placeholder: 'World number or region', 'aria-label': 'Filter worlds' });
  search.addEventListener('input', () => { query = search.value.trim(); draw(); });

  append(bar,
    countLabel,
    button('Broadcast', { icon: 'megaphone', on: { click: () => broadcastDialog(chosen(), clear) } }),
    button('Reboot', { icon: 'power', on: { click: () => rebootDialog(chosen().sort((a, b) => a.id - b.id), clear) } }),
    button('Cancel reboot', { icon: 'undo', on: { click: () => cancelRebootDialog(chosen(), clear) } }),
    button('Maintenance', { icon: 'lock', on: { click: () => maintenanceDialog(chosen(), clear) } }),
    button(null, { icon: 'x', 'aria-label': 'Clear selection', title: 'Clear selection', on: { click: clear } }));

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

  const refresh = button('Refresh', { icon: 'refresh', on: { click: async () => { refresh.disabled = true; await loadWorlds(); refresh.disabled = false; } } });
  return h('div', { class: 'page' },
    pageHead('Worlds', null, refresh, state.canCommand && button('Select shown', { icon: 'check', on: { click: selectShown } })),
    subtitle,
    h('div', { class: 'toolbar' }, h('div', { class: 'search' }, icon('search'), search), filters),
    !state.canCommand && h('div', { class: 'callout' }, icon('shield'), h('div', {}, 'Rebooting and broadcasting need the admin role. Tap a world to see who\'s on it.')),
    tiles);
}

// Tick time, memory and connections over the last minute.
function healthLine(health) {
  const tone = health.overruns > 0 || health.tick_max_ms > 600 ? 'danger' : health.tick_ms > 300 ? 'warn' : '';
  return h('div', { class: 'tile-health', title: `Ticks over the last minute: ${health.tick_ms.toFixed(1)} ms on average, ${health.tick_max_ms.toFixed(0)} ms at worst, ${health.overruns} over 600 ms` },
    h('span', { class: tone }, `${health.tick_ms < 10 ? health.tick_ms.toFixed(1) : Math.round(health.tick_ms)} ms tick`),
    health.memory_bytes != null && h('span', {}, megabytes(health.memory_bytes)),
    h('span', {}, plural(health.connections, 'conn')));
}

function worldPlayersDialog(world) {
  const list = h('div', { class: 'list' }, h('div', { class: 'empty' }, h('div', { class: 'spinner' })));
  const filterInput = h('input', { class: 'input', type: 'search', placeholder: 'Filter by name', 'aria-label': 'Filter players' });
  let players = [];
  const draw = () => {
    const q = filterInput.value.trim().toLowerCase().replace(/_/g, ' ');
    const shown = players.filter((p) => p.name.toLowerCase().includes(q)).slice(0, 300);
    replace(list, shown.length ? shown.map((p) => playerItem(p, null, close)) : h('div', { class: 'empty' }, 'Nobody matches.'));
  };
  filterInput.addEventListener('input', draw);
  const { close } = openDialog({
    title: `${worldName(world)} players`,
    subtitle: world.region,
    body: [h('div', { class: 'search' }, icon('search'), filterInput), list],
    width: 'min(560px, calc(100vw - 32px))',
  });
  api('GET', `/worlds/${world.node}/players`).then((data) => {
    players = data;
    draw();
  }).catch((e) => replace(list, h('div', { class: 'empty' }, e.message)));
}

// ---------------------------------------------------------------------------
// Players

const STAFF = ['', 'Mod', 'Admin', 'Developer'];

function playerBadges(p) {
  return [
    p.staff_level > 0 && h('span', { class: 'badge accent' }, icon('shield'), STAFF[p.staff_level] || 'Staff'),
    p.muted && h('span', { class: 'badge warn' }, icon('mute'), 'Muted'),
  ];
}

function playerItem(p, world, onAct, refresh) {
  const act = (action) => () => {
    onAct?.();
    moderateDialog(p.name, action, refresh);
  };
  return h('div', { class: 'list-item' },
    h('div', { class: 'avatar' }, initial(p.name)),
    h('div', { class: 'list-main' },
      h('div', { class: 'list-title' }, h('b', {}, playerLink(p.name, { on: { click: () => onAct?.() } })), ' ', playerBadges(p)),
      h('div', { class: 'list-sub num' }, world ? `World ${world} · ` : '', `${p.x}, ${p.z}${p.level ? ` · floor ${p.level}` : ''}`)),
    h('div', { class: 'chips' },
      button(null, { small: true, icon: 'kick', 'aria-label': `Kick ${p.name}`, title: 'Kick', on: { click: act('kick') } }),
      button(null, { small: true, icon: 'mute', 'aria-label': `Mute ${p.name}`, title: 'Mute', on: { click: act('mute') } }),
      button(null, { small: true, icon: 'ban', 'aria-label': `Ban ${p.name}`, title: 'Ban', on: { click: act('ban') } })));
}

function playersPage(ctx) {
  const results = h('div', { class: 'list' });
  const summary = h('span', { class: 'muted' });
  const query = h('input', { class: 'input', type: 'search', placeholder: 'Search online players by name', autofocus: true, 'aria-label': 'Search players', value: ctx.params.get('q') || '' });
  let seq = 0;

  const load = async () => {
    const mine = ++seq;
    const q = query.value.trim();
    try {
      const data = await api('GET', `/players?q=${encodeURIComponent(q)}`);
      if (mine !== seq) return;
      summary.textContent = data.total > data.players.length
        ? `Showing ${data.players.length} of ${plural(data.total, 'player')}`
        : plural(data.total, 'player') + (q ? ' match' : ' online');
      if (data.unreachable.length) summary.textContent += ` · ${plural(data.unreachable.length, 'world')} didn't answer`;
      replace(results, data.players.length
        ? data.players.map((p) => playerItem(p, p.world, null, load))
        : h('div', { class: 'empty' }, icon('users'), q ? `Nobody online matches "${q}".` : 'Nobody is online.',
          q && validName(q) && button(`Look up ${q}`, { small: true, icon: 'search', on: { click: () => navigate(`/player?name=${encodeURIComponent(q)}`) } })));
    } catch (e) {
      if (mine === seq) replace(results, h('div', { class: 'empty' }, e.message));
    }
  };
  const later = debounce(() => {
    const q = query.value.trim();
    history.replaceState(null, '', q ? `/players?q=${encodeURIComponent(q)}` : '/players');
    load();
  }, 250);
  query.addEventListener('input', later);
  replace(results, h('div', { class: 'empty' }, h('div', { class: 'spinner' })));
  load();
  const timer = setInterval(() => { if (!document.hidden && !document.querySelector('dialog[open]')) load(); }, 15000);
  ctx.cleanup(() => clearInterval(timer));

  return h('div', { class: 'page' },
    pageHead('Players', 'Everyone online, across every world. Look up anyone, online or not.',
      button('Moderate by name', { icon: 'shield', on: { click: () => moderateDialog('', 'mute', load) } })),
    h('form', { class: 'toolbar', on: { submit: (e) => {
      e.preventDefault();
      const q = query.value.trim();
      if (validName(q)) navigate(`/player?name=${encodeURIComponent(q)}`);
    } } },
      h('div', { class: 'search grow' }, icon('search'), query),
      h('button', { type: 'submit', class: 'btn' }, icon('user'), 'Look up')),
    h('section', { class: 'card' },
      h('div', { class: 'card-head' }, h('h2', {}, 'Online'), h('div', { class: 'actions' }, summary)),
      results));
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
};

function auditItem(e) {
  const [verb, iconName, tone] = VERBS[e.action] || [e.action.replace(/_/g, ' '), 'activity'];
  const failed = !e.ok;
  // Sign-ins under a name that isn't an account have no actor, only the
  // name tried.
  const title = !e.actor && e.target
    ? (e.action === 'sign_in_locked'
      ? [h('b', {}, e.target), ' was locked out after repeated failed sign-ins']
      : ['Failed sign-in as ', h('b', {}, e.target)])
    : [h('b', {}, e.actor || 'Someone'), ` ${verb}`, e.target ? [' ', h('b', {}, e.target)] : null];
  // A sign-in's detail is the browser's user agent.
  const detail = e.action === 'sign_in' && e.detail ? device(e.detail) : e.detail;
  return h('div', { class: 'list-item' },
    h('div', { class: `list-icon ${failed ? 'danger' : tone || ''}` }, icon(failed ? 'alert' : iconName)),
    h('div', { class: 'list-main' },
      h('div', { class: 'list-title' }, title,
        failed && e.actor && [' ', h('span', { class: 'badge danger' }, 'Failed')]),
      detail && h('div', { class: 'list-sub' }, detail)),
    h('div', { class: 'list-side', title: `${fullTime(e.at_ms)}${e.ip ? ` from ${e.ip}` : ''}` }, ago(e.at_ms)));
}

function auditPage() {
  const list = h('div', { class: 'list' });
  const more = h('div', { class: 'more hidden' });
  const search = h('input', { class: 'input', type: 'search', placeholder: 'Search actions, players, details', 'aria-label': 'Search the audit log' });
  let mine = false;
  let last = null;
  let seq = 0;
  const chips = h('div', { class: 'chips' });

  const load = async (append_) => {
    const current = ++seq;
    const params = new URLSearchParams({ limit: '50' });
    if (search.value.trim()) params.set('q', search.value.trim());
    if (mine) params.set('actor', state.session.name);
    if (append_ && last) params.set('before', String(last));
    try {
      const entries = await api('GET', `/audit?${params}`);
      if (current !== seq) return;
      if (!append_) list.replaceChildren();
      append(list, entries.map(auditItem));
      if (!list.children.length) append(list, h('div', { class: 'empty' }, icon('list'), 'No matching entries.'));
      last = entries.length ? entries[entries.length - 1].id : last;
      more.classList.toggle('hidden', entries.length < 50);
    } catch (e) {
      replace(list, h('div', { class: 'empty' }, e.message));
    }
  };
  const drawChips = () => replace(chips,
    h('button', { type: 'button', class: 'chip', 'aria-pressed': String(!mine), on: { click: () => { mine = false; drawChips(); load(); } } }, 'Everyone'),
    h('button', { type: 'button', class: 'chip', 'aria-pressed': String(mine), on: { click: () => { mine = true; drawChips(); load(); } } }, 'Only mine'));
  drawChips();
  search.addEventListener('input', debounce(() => load(), 250));
  append(more, button('Load older', { small: true, on: { click: () => load(true) } }));
  load();

  return h('div', { class: 'page' },
    pageHead('Audit log', 'Everything done through the console, sign-ins included.'),
    h('div', { class: 'toolbar' }, h('div', { class: 'search' }, icon('search'), search), chips),
    h('section', { class: 'card' }, list, more));
}

// ---------------------------------------------------------------------------
// Account

function accountPage() {
  const { name, role } = state.session;
  const sessions = h('div', {});
  const endOthers = button('Sign out other sessions', { small: true, kind: 'ghost' });

  const loadSessions = async () => {
    try {
      const account = await api('GET', '/account');
      const others = account.sessions.filter((s) => !s.current).length;
      endOthers.classList.toggle('hidden', others === 0);
      replace(sessions, account.sessions.map((s) => h('div', { class: 'session-row' },
        h('div', { class: 'list-icon' }, icon('monitor')),
        h('div', { class: 'list-main' },
          h('div', { class: 'list-title' }, h('b', {}, device(s.user_agent)), ' ', s.current && h('span', { class: 'badge ok' }, 'This device')),
          h('div', { class: 'list-sub' }, `${s.ip} · signed in ${ago(s.created_ms)} · active ${ago(s.last_seen_ms)}`)),
        !s.current && button('Sign out', { small: true, on: { click: async () => {
          try {
            await api('POST', `/account/sessions/${s.id}/end`);
            toast('Session signed out.');
            loadSessions();
          } catch (e) { toast(e.message, 'error'); }
        } } }))));
    } catch (e) {
      replace(sessions, h('div', { class: 'empty' }, e.message));
    }
  };
  endOthers.addEventListener('click', async () => {
    try {
      const result = await api('POST', '/account/sessions/end-others');
      toast(`Signed out ${plural(result.ended, 'other session')}.`);
      loadSessions();
    } catch (e) { toast(e.message, 'error'); }
  });
  loadSessions();

  const current = h('input', { class: 'input', type: 'password', autocomplete: 'current-password', required: true });
  const next = h('input', { class: 'input', type: 'password', autocomplete: 'new-password', minlength: '12', required: true });
  const again = h('input', { class: 'input', type: 'password', autocomplete: 'new-password', required: true });
  const code = h('input', { class: 'input', inputmode: 'numeric', autocomplete: 'one-time-code', maxlength: '6', pattern: '[0-9]{6}', required: true, placeholder: '000000' });
  const error = h('p', { class: 'error-text', role: 'alert' });
  const form = h('form', { class: 'form-grid' },
    h('label', { class: 'field' }, h('span', {}, 'Current password'), current),
    h('label', { class: 'field' }, h('span', {}, 'New password'), next, h('span', { class: 'hint' }, 'At least 12 characters. A passphrase works well.')),
    h('label', { class: 'field' }, h('span', {}, 'New password again'), again),
    h('label', { class: 'field' }, h('span', {}, 'Authenticator code'), code),
    error,
    h('div', {}, h('button', { type: 'submit', class: 'btn primary' }, 'Change password')));
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
      await api('POST', '/account/password', { current: current.value, new: next.value, code: code.value });
      form.reset();
      toast('Password changed. Your other sessions were signed out.');
      loadSessions();
    } catch (err) {
      error.textContent = err.message;
    } finally {
      submit.disabled = false;
    }
  });

  const themes = h('div', { class: 'segmented', role: 'group', 'aria-label': 'Theme' });
  const drawThemes = () => replace(themes, ['auto', 'light', 'dark'].map((t) =>
    h('button', { type: 'button', 'aria-pressed': String(prefs.get('theme', 'dark') === t), on: { click: () => { prefs.set('theme', t); applyTheme(t); drawThemes(); } } },
      t === 'auto' ? 'System' : t[0].toUpperCase() + t.slice(1))));
  drawThemes();

  return h('div', { class: 'page' },
    pageHead('Account', null, button('Sign out', { icon: 'logout', on: { click: signOut } })),
    h('section', { class: 'card' },
      h('div', { class: 'card-body' },
        h('div', { class: 'list-item' },
          h('div', { class: 'avatar' }, initial(name)),
          h('div', { class: 'list-main' },
            h('div', { class: 'list-title' }, h('b', {}, name), ' ', h('span', { class: 'badge accent' }, role)),
            h('div', { class: 'list-sub' }, {
              developer: 'Can do everything, including managing staff accounts.',
              admin: 'Can reboot, broadcast, toggle maintenance, kick, mute and ban.',
              moderator: 'Can kick, mute and ban. World controls need the admin role.',
            }[role])),
          h('span', { class: 'badge ok' }, icon('shield'), '2FA on')))),
    h('div', { class: 'split' },
      h('section', { class: 'card' },
        h('div', { class: 'card-head' }, h('h2', {}, 'Sessions'), h('div', { class: 'actions' }, endOthers)),
        sessions),
      h('section', { class: 'card' },
        h('div', { class: 'card-head' }, h('h2', {}, 'Appearance')),
        h('div', { class: 'card-body' }, themes))),
    h('section', { class: 'card' },
      h('div', { class: 'card-head' }, h('h2', {}, 'Change password')),
      h('div', { class: 'card-body' }, form)));
}

// ---------------------------------------------------------------------------
// Lookups shared by the player, moderation and chat pages

const MOD_VERBS = {
  mute: ['muted', 'mute', 'warn'],
  unmute: ['unmuted', 'undo', ''],
  ban: ['banned', 'ban', 'danger'],
  unban: ['unbanned', 'undo', ''],
  kick: ['kicked', 'kick', ''],
};

const hoursLabel = (hours) => DURATIONS.find(([h0]) => h0 === hours)?.[1].toLowerCase() ?? plural(hours, 'hour');

function moderationItem(m) {
  const [verb, iconName, tone] = MOD_VERBS[m.action] || [m.action, 'shield', ''];
  return h('div', { class: 'list-item' },
    h('div', { class: `list-icon ${tone}` }, icon(iconName)),
    h('div', { class: 'list-main' },
      h('div', { class: 'list-title' }, h('b', {}, playerLink(m.moderator)), ` ${verb} `, h('b', {}, playerLink(m.target)),
        m.hours ? ` for ${hoursLabel(m.hours)}` : ''),
      h('div', { class: 'list-sub' }, m.world ? `World ${m.world}` : `Node ${m.node}`)),
    h('div', { class: 'list-side', title: fullTime(m.at_ms) }, ago(m.at_ms)));
}

function chatItem(c) {
  return h('div', { class: 'chat-row' },
    h('span', { class: 'chat-time', title: fullTime(c.at_ms) }, shortTime(c.at_ms)),
    h('span', { class: 'chat-world', title: `${c.x}, ${c.z}${c.level ? `, floor ${c.level}` : ''}` }, c.world ? `W${c.world}` : `N${c.node}`),
    h('span', { class: 'chat-who' }, playerLink(c.from),
      c.to && [h('span', { class: 'muted' }, ' to '), playerLink(c.to)]),
    h('span', { class: `chat-text ${c.to ? 'private' : ''}` }, c.message));
}

// A list that loads a page at a time, newest first, with "Load older".
function pagedList(fetchPage, item, emptyText, cursor = (row) => row.id) {
  const list = h('div', { class: 'list' }, h('div', { class: 'empty' }, h('div', { class: 'spinner' })));
  const more = h('div', { class: 'more hidden' });
  let last = null;
  let seq = 0;
  const load = async (older) => {
    const mine = ++seq;
    try {
      const rows = await fetchPage(older ? last : null);
      if (mine !== seq) return;
      if (!older) list.replaceChildren();
      append(list, rows.map((row) => item(row)));
      if (!list.children.length) append(list, h('div', { class: 'empty' }, emptyText));
      if (rows.length) last = cursor(rows[rows.length - 1]);
      more.classList.toggle('hidden', rows.length < 50);
    } catch (e) {
      if (mine === seq) replace(list, h('div', { class: 'empty' }, e.message));
    }
  };
  append(more, button('Load older', { small: true, on: { click: () => load(true) } }));
  load(false);
  return { el: [list, more], reload: () => load(false) };
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
  const page = h('div', { class: 'page' });
  if (!validName(name)) {
    append(page, pageHead('Player', null), h('div', { class: 'empty' }, icon('user'), 'That isn\'t a valid player name.'));
    return page;
  }
  const head = h('div', {}, pageHead(name, 'Looking them up...'));
  const facts = h('div', { class: 'stats' });
  const fact = (iconName, label, value, sub) => h('div', { class: 'card stat' },
    h('div', { class: 'label' }, icon(iconName), label), h('div', { class: 'value' }, value), h('div', { class: 'sub' }, sub));

  const load = async () => {
    let p;
    try {
      p = await api('GET', `/lookup/player?${query({ name })}`);
    } catch (e) {
      replace(head, pageHead(name, e.message));
      return;
    }
    if (!p) {
      replace(head, pageHead(name, 'No registered player has this name.'));
      facts.classList.add('hidden');
      return;
    }
    document.title = `${p.name} · RS Admin`;
    const muted = p.muted_until_ms != null;
    const banned = p.banned_until_ms != null;
    const act = (action) => () => moderateDialog(p.name, action, load);
    replace(head, pageHead(h('span', {}, p.name, ' ', playerBadges({ staff_level: p.staff_level })),
      p.online_node != null ? `Online on ${p.online_world ? `world ${p.online_world}` : `node ${p.online_node}`}` : 'Offline',
      p.online_node != null && button('Kick', { icon: 'kick', on: { click: act('kick') } }),
      button(muted ? 'Unmute' : 'Mute', { icon: muted ? 'undo' : 'mute', on: { click: act(muted ? 'unmute' : 'mute') } }),
      button(banned ? 'Unban' : 'Ban', { icon: banned ? 'undo' : 'ban', kind: banned ? undefined : 'danger', on: { click: act(banned ? 'unban' : 'ban') } }),
      button('Open case', { icon: 'briefcase', on: { click: () => newCaseDialog({ player: p.name }) } })));
    const seen = p.online_node != null
      ? fact('activity', 'Status', 'Online', `${p.online_world ? `World ${p.online_world}` : `Node ${p.online_node}`}${p.online_since_ms ? ` since ${ago(p.online_since_ms)}` : ''}`)
      : fact('clock', 'Last seen', p.last_seen_ms ? ago(p.last_seen_ms) : 'Never', p.last_seen_ms ? `${p.last_world ? `World ${p.last_world}` : 'Logged out'} · ${fullTime(p.last_seen_ms)}` : 'No logout on record');
    replace(facts,
      seen,
      fact('user', 'Registered', new Date(p.registered_ms).toLocaleDateString(), ago(p.registered_ms)),
      fact('mute', 'Mute', muted ? h('span', { class: 'tone-warn' }, 'Muted') : 'None', muted ? `${until(p.muted_until_ms)} · ends ${fullTime(p.muted_until_ms)}` : 'Can chat'),
      fact('ban', 'Ban', banned ? h('span', { class: 'tone-danger' }, 'Banned') : 'None', banned ? `${until(p.banned_until_ms)} · ends ${fullTime(p.banned_until_ms)}` : 'Can log in'));
  };
  load();

  const mods = pagedList(
    (before) => api('GET', `/lookup/moderation?${query({ name, before })}`),
    moderationItem, 'No moderation on record.');
  const chat = pagedList(
    (before) => api('GET', `/lookup/chat?${query({ name, before })}`),
    chatItem, 'No chat on record.');
  const sessions = pagedList(
    (before) => api('GET', `/replays?${query({ name, before })}`).then((rows) => { if (!before) dockWarm(rows); return rows; }),
    replayItem, 'No recorded sessions.', (r) => r.started_ms);
  const cases = spinnerList();
  const loadCases = () => api('GET', `/cases?${query({ player: name, status: 'all' })}`).then((rows) =>
    replace(cases, rows.length ? rows.map((c) => caseItem(c)) : h('div', { class: 'empty' }, icon('briefcase'), 'No cases on them.')))
    .catch((e) => failed(cases, e));
  loadCases();
  const reports = pagedList(
    (before) => api('GET', `/lookup/reports?${query({ name, before })}`),
    (r) => reportItem(r, () => { reports.reload(); loadCases(); }), 'No abuse reports against them.');

  append(page, head, facts,
    isDeveloper() && editSection(name),
    h('div', { class: 'split' },
      card('Cases', button('Open case', { small: true, icon: 'briefcase', on: { click: () => newCaseDialog({ player: name }) } }), cases),
      card('Abuse reports against them', link(`/moderation?${query({ tab: 'triage', all: 1, name })}`, { class: 'btn small ghost' }, 'In the queue', icon('chevron')), reports.el)),
    h('section', { class: 'card' },
      h('div', { class: 'card-head' }, h('h2', {}, 'Recorded sessions'),
        h('div', { class: 'actions' }, link(`/replays?${query({ name })}`, { class: 'btn small ghost' }, 'All their sessions', icon('chevron')))),
      sessions.el),
    h('div', { class: 'split' },
      h('section', { class: 'card' },
        h('div', { class: 'card-head' }, h('h2', {}, 'Chat'),
          h('div', { class: 'actions' }, link(`/chat?${query({ name })}`, { class: 'btn small ghost' }, 'All chat logs', icon('chevron')))),
        h('div', { class: 'chat-list' }, chat.el)),
      h('section', { class: 'card' },
        h('div', { class: 'card-head' }, h('h2', {}, 'Moderation history')),
        mods.el)));
  return page;
}

// ---------------------------------------------------------------------------
// Player editing on the player page: stats, items and position, changed
// live while they're logged in, or in their save while they're not.

const INV_LABELS = { inv: 'Backpack', worn: 'Worn', bank: 'Bank' };
const invLabel = (name) => INV_LABELS[name] || name;

async function editPlayer(name, edit, onDone) {
  const result = await api('POST', '/players/edit', { name, edit });
  if (!result.ok) throw new Error(result.message);
  toast(result.message);
  onDone();
}

function setStatDialog(name, stat, onDone) {
  const level = h('input', { class: 'input', type: 'number', min: '1', max: '99', step: '1', required: true, autofocus: true, value: String(stat.base), inputmode: 'numeric', 'data-invalid': 'Levels are 1 to 99.' });
  openDialog({
    title: `${stat.name}: ${name}`,
    subtitle: `Level ${stat.base}, ${stat.xp.toLocaleString()} xp now.`,
    body: [h('label', { class: 'field' }, h('span', {}, 'Level'), level,
      h('span', { class: 'hint' }, 'Sets the level and its experience, as ::setstat does.'))],
    actions: [{ label: 'Cancel' }, {
      label: 'Set level',
      kind: 'primary',
      run: () => editPlayer(name, { kind: 'set_stat', stat: stat.id, level: Number(level.value) }, onDone),
    }],
  });
}

function giveDialog(name, sheet, onDone) {
  const targets = sheet.invs.filter((i) => i.name !== 'worn').map((i) => i.name);
  let inv = targets.includes('inv') ? 'inv' : targets[0];
  let chosen = null;
  const invChips = h('div', { class: 'segmented' });
  const drawInvs = () => replace(invChips, targets.map((t) =>
    h('button', { type: 'button', 'aria-pressed': String(inv === t), on: { click: () => { inv = t; drawInvs(); } } }, invLabel(t))));
  drawInvs();
  const search = h('input', { class: 'input', type: 'search', required: true, autofocus: true, autocomplete: 'off', spellcheck: 'false', placeholder: 'Item name, config name or id', 'data-invalid': 'Pick an item from the results.' });
  search.setCustomValidity('Pick an item');
  const results = h('div', { class: 'obj-results' });
  const count = h('input', { class: 'input', type: 'number', min: '1', max: '2147483647', step: '1', value: '1', required: true, inputmode: 'numeric', 'data-invalid': 'Counts are 1 to 2147483647.' });
  let timer;
  let seq = 0;
  const pick = (obj) => {
    chosen = obj;
    search.value = obj.name;
    search.setCustomValidity('');
    search.removeAttribute('aria-invalid');
    replace(results, h('div', { class: 'obj-picked' }, icon('check'), h('b', {}, obj.name), h('span', { class: 'muted num' }, `${obj.debugname} · id ${obj.id}${obj.stackable ? ' · stacks' : ''}`)));
  };
  search.addEventListener('input', () => {
    chosen = null;
    search.setCustomValidity('Pick an item');
    clearTimeout(timer);
    const q = search.value.trim();
    if (!q) return replace(results);
    timer = setTimeout(async () => {
      const mine = ++seq;
      try {
        const objs = await api('GET', `/objs?${query({ q })}`);
        if (mine !== seq) return;
        replace(results, objs.length
          ? objs.map((o) => h('button', { type: 'button', class: 'obj-row', on: { click: () => pick(o) } },
            h('span', {}, o.name), h('span', { class: 'muted num' }, `${o.debugname} · ${o.id}`)))
          : h('p', { class: 'hint' }, 'No items match.'));
      } catch (e) {
        if (mine === seq) replace(results, h('p', { class: 'error-text' }, e.message));
      }
    }, 200);
  });
  openDialog({
    title: `Give an item: ${name}`,
    subtitle: sheet.online ? 'They get it straight away.' : 'Goes into their save; they\'ll have it at their next login.',
    body: [
      h('div', { class: 'field' }, h('span', {}, 'Into'), invChips),
      h('label', { class: 'field' }, h('span', {}, 'Item'), search, results),
      h('label', { class: 'field' }, h('span', {}, 'Count'), count),
    ],
    actions: [{ label: 'Cancel' }, {
      label: 'Give',
      kind: 'primary',
      icon: 'plus',
      run: () => editPlayer(name, { kind: 'give', inv, obj: chosen.id, count: Number(count.value) }, onDone),
    }],
  });
}

function takeDialog(name, inv, item, onDone) {
  const count = h('input', { class: 'input', type: 'number', min: '1', max: '2147483647', step: '1', value: String(item.count), required: true, autofocus: true, inputmode: 'numeric', 'data-invalid': 'Counts are 1 to 2147483647.' });
  openDialog({
    title: `Take ${item.name}: ${name}`,
    subtitle: `${item.count.toLocaleString()} in ${invLabel(inv.name).toLowerCase()} slot ${item.slot}.`,
    body: [h('label', { class: 'field' }, h('span', {}, 'Count'), count)],
    actions: [{ label: 'Cancel' }, {
      label: 'Take',
      kind: 'danger',
      icon: 'trash',
      run: () => editPlayer(name, { kind: 'take', inv: inv.name, obj: item.id, count: Number(count.value) }, onDone),
    }],
  });
}

function moveDialog(name, sheet, onDone) {
  const coord = (label, value, max) => h('input', { class: 'input', type: 'number', min: '0', max: String(max), step: '1', value: String(value), required: true, inputmode: 'numeric', 'aria-label': label, 'data-invalid': `${label} is 0 to ${max}.` });
  const x = coord('X', sheet.x, 16383);
  const z = coord('Z', sheet.z, 16383);
  const level = coord('Level', sheet.level, 3);
  openDialog({
    title: `Move: ${name}`,
    subtitle: sheet.online ? 'Teleports them now.' : 'Changes where they log in.',
    body: [h('div', { class: 'coord-grid' },
      h('label', { class: 'field' }, h('span', {}, 'X'), x),
      h('label', { class: 'field' }, h('span', {}, 'Z'), z),
      h('label', { class: 'field' }, h('span', {}, 'Level'), level))],
    actions: [{ label: 'Cancel' }, {
      label: 'Move',
      kind: 'primary',
      icon: 'pin',
      run: () => editPlayer(name, { kind: 'move', x: Number(x.value), z: Number(z.value), level: Number(level.value) }, onDone),
    }],
  });
}

function editSection(name) {
  const body = h('div', { class: 'card-body' }, h('div', { class: 'spinner' }));
  const where = h('span', { class: 'badge' });
  const actions = h('div', { class: 'actions' });
  const load = async () => {
    let sheet;
    try {
      sheet = await api('GET', `/players/sheet?${query({ name })}`);
    } catch (e) {
      replace(body, h('p', { class: 'error-text' }, e.message));
      return;
    }
    if (!sheet) {
      replace(body, h('p', { class: 'muted' }, 'No registered player has this name.'));
      return;
    }
    where.className = `badge ${sheet.online ? 'ok' : ''}`;
    replace(where, sheet.online ? `Live on world ${sheet.world ?? '?'}` : 'From their save');
    replace(actions,
      button('Give item', { small: true, icon: 'plus', on: { click: () => giveDialog(sheet.name, sheet, load) } }),
      button('Move', { small: true, icon: 'pin', on: { click: () => moveDialog(sheet.name, sheet, load) } }),
      button(null, { small: true, icon: 'refresh', 'aria-label': 'Reload', title: 'Reload', on: { click: load } }));
    const total = sheet.stats.reduce((sum, s) => sum + s.base, 0);
    replace(body,
      h('p', { class: 'hint' }, `At ${sheet.x}, ${sheet.z}${sheet.level ? `, level ${sheet.level}` : ''} · total level ${total}. Click a stat to set it, or an item to take it.`),
      h('div', { class: 'skill-grid' }, sheet.stats.map((s) =>
        h('button', { type: 'button', class: 'skill', title: `${s.xp.toLocaleString()} xp`, on: { click: () => setStatDialog(sheet.name, s, load) } },
          h('span', { class: 'skill-name' }, s.name),
          h('span', { class: 'num' }, s.level === s.base ? String(s.base) : `${s.level}/${s.base}`)))),
      sheet.invs.map((inv) => h('div', { class: 'inv' },
        h('div', { class: 'inv-head' }, h('b', {}, invLabel(inv.name)), h('span', { class: 'muted num' }, `${inv.items.length} / ${inv.size}`)),
        inv.items.length
          ? h('div', { class: 'item-grid' }, inv.items.map((item) =>
            h('button', { type: 'button', class: 'item', title: `Slot ${item.slot} · id ${item.id}`, on: { click: () => takeDialog(sheet.name, inv, item, load) } },
              h('span', { class: 'item-name' }, item.name),
              item.count > 1 && h('span', { class: 'num muted' }, `x${item.count.toLocaleString()}`))))
          : h('p', { class: 'hint' }, 'Empty.'))));
  };
  load();
  return h('section', { class: 'card' },
    h('div', { class: 'card-head' }, h('h2', {}, 'Stats and items'), where, actions),
    body);
}

// ---------------------------------------------------------------------------
// News on the website (developers)

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
      headers: { 'Content-Type': 'application/octet-stream', 'X-CSRF-Token': state.session?.csrf ?? '' },
      credentials: 'same-origin',
      body: file,
    });
  } catch {
    throw new Error('Could not reach the console. Check your connection.');
  }
  let data = null;
  try { data = await res.json(); } catch { /* not JSON */ }
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
  const status = h('span', { class: 'hint' });
  const input = h('input', { type: 'file', class: 'hidden', accept: MEDIA_TYPES.join(','), multiple: true });
  const add = button(label, { small: true, icon: 'clip', on: { click: () => input.click() } });
  let uploading = 0;
  const insert = (text, at) => {
    textarea.setRangeText(text, at, at, 'end');
    textarea.dispatchEvent(new Event('input', { bubbles: true }));
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
  input.addEventListener('change', () => { send([...input.files]); input.value = ''; });
  textarea.addEventListener('paste', (e) => {
    const pasted = [...(e.clipboardData?.files ?? [])];
    if (pasted.length) { e.preventDefault(); send(pasted); }
  });
  textarea.addEventListener('dragover', (e) => { if (e.dataTransfer?.types.includes('Files')) e.preventDefault(); });
  textarea.addEventListener('drop', (e) => {
    const dropped = [...(e.dataTransfer?.files ?? [])];
    if (dropped.length) { e.preventDefault(); send(dropped); }
  });
  return { button: add, status, input, busy: () => uploading > 0 };
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
  const chips = h('div', { class: 'chips' });
  const draw = () => replace(chips, categories.map((c) =>
    h('button', { type: 'button', class: 'chip', 'aria-pressed': String(category === c.id), on: { click: () => { category = c.id; draw(); } } }, c.name)));
  draw();
  const title = h('input', { class: 'input', maxlength: '120', required: true, autofocus: true, value: post?.title ?? '', placeholder: 'The Grand Exchange opens' });
  const content = h('textarea', { class: 'input code-area', required: true, rows: '14', spellcheck: 'true', 'aria-label': 'Text',
    placeholder: '## What\'s new\n\nThe **Grand Exchange** opens today.\n\n![The new trading floor](...)' });
  content.value = post?.markdown ?? '';

  const media = mediaUploader(content, (file) => uploadMedia(file));

  // Write and Preview.
  let mode = 'write';
  const preview = h('div', { class: 'news-preview hidden' });
  const writeArea = h('div', { class: 'news-write' },
    h('div', { class: 'news-tools' }, media.button, media.status, media.input), content,
    h('span', { class: 'hint' }, 'Markdown: **bold**, *italic*, ## headings, - lists, [links](https://...). Add images, GIFs and videos with the button, or paste or drop them in.'));
  const tabs = h('div', { class: 'segmented tabs', role: 'group', 'aria-label': 'View' });
  const showPreview = async () => {
    replace(preview, h('div', { class: 'spinner' }));
    try {
      const { html } = await api('POST', '/news/preview', { markdown: content.value });
      const doc = new DOMParser().parseFromString(html, 'text/html');
      const nodes = previewNodes(doc.body.childNodes);
      replace(preview, nodes.length ? nodes : h('p', { class: 'muted' }, 'Nothing to preview yet.'));
    } catch (e) {
      replace(preview, h('p', { class: 'error-text' }, e.message));
    }
  };
  const drawTabs = () => {
    replace(tabs, [['write', 'Write'], ['preview', 'Preview']].map(([key, label]) =>
      h('button', { type: 'button', 'aria-pressed': String(mode === key), on: { click: () => { mode = key; drawTabs(); } } }, label)));
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
      h('div', { class: 'field' }, h('span', {}, 'Category'), chips),
      h('label', { class: 'field' }, h('span', {}, 'Title'), title),
      h('div', { class: 'field' }, h('span', {}, 'Text'), tabs, writeArea, preview),
    ],
    actions: [{ label: 'Cancel' }, {
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
        await api('POST', '/news/save', { id: post?.id ?? null, category_id: category, title: title.value.trim(), markdown: content.value });
        toast(post ? 'Post saved.' : 'Posted.');
        onDone();
      },
    }],
  });
}

function newsPage() {
  const list = h('div', { class: 'list' }, h('div', { class: 'empty' }, h('div', { class: 'spinner' })));
  const more = h('div', { class: 'more hidden' });
  const newButton = button('New post', { icon: 'plus', kind: 'primary', disabled: true });
  let categories = [];
  let last = null;
  const categoryName = (id) => categories.find((c) => c.id === id)?.name ?? `Category ${id}`;
  const item = (p) => h('div', { class: 'list-item' },
    h('div', { class: 'list-icon accent' }, icon('news')),
    h('div', { class: 'list-main' },
      h('div', { class: 'list-title' }, h('b', {}, p.title), h('span', { class: 'badge' }, categoryName(p.category_id))),
      h('div', { class: 'list-sub' }, `#${p.id} · ${fullTime(p.created_ms)}`)),
    h('div', { class: 'chips' },
      button('Edit', { small: true, icon: 'edit', on: { click: () => newsDialog(categories, p, () => load(false)) } }),
      button(null, { small: true, icon: 'trash', 'aria-label': `Delete ${p.title}`, title: 'Delete', on: { click: () => confirmDialog({
        title: 'Delete post',
        subtitle: p.title,
        body: [h('p', { class: 'muted' }, 'It comes off the website for good.')],
        label: 'Delete',
        kind: 'danger',
        run: async () => {
          const result = await api('POST', '/news/delete', { id: p.id, title: p.title });
          toast(result.message, result.ok ? 'ok' : 'error');
          load(false);
        },
      }) } })));
  const load = async (older) => {
    try {
      const page = await api('GET', `/news?${query({ before: older ? last : null })}`);
      categories = page.categories;
      newButton.disabled = !categories.length;
      if (!older) list.replaceChildren();
      append(list, page.posts.map(item));
      if (!list.children.length) append(list, h('div', { class: 'empty' }, icon('news'), 'No posts yet.'));
      if (page.posts.length) last = page.posts[page.posts.length - 1].id;
      more.classList.toggle('hidden', page.posts.length < 20);
    } catch (e) {
      replace(list, h('div', { class: 'empty' }, e.message));
    }
  };
  newButton.addEventListener('click', () => newsDialog(categories, null, () => load(false)));
  append(more, button('Load older', { small: true, on: { click: () => load(true) } }));
  load(false);
  return h('div', { class: 'page' },
    pageHead('News', 'Posts on the website\'s front page and news archive.', newButton),
    h('section', { class: 'card' }, list, more));
}

// ---------------------------------------------------------------------------
// Moderation center: cases staff work on together, the triage queue of abuse
// reports and flagged sessions, mutes and bans in force, and every action.

// The Report Abuse screen's 12 rules, by the index the client sends.
const REPORT_RULES = ['Offensive language', 'Item scamming', 'Password scamming', 'Bug abuse', 'Jagex staff impersonation',
  'Account sharing/trading', 'Macroing', 'Multiple logging in', 'Encouraging others to break rules',
  'Misuse of customer support', 'Advertising / website', 'Real world item trading'];
const ruleName = (reason) => `Rule ${reason + 1}: ${REPORT_RULES[reason] || 'unknown'}`;

const CASE_STATUS = { open: ['Open', 'warn'], investigating: ['Investigating', 'accent'], waiting: ['Waiting', ''], closed: ['Closed', 'ok'] };
const PRIORITIES = [['low', 'Low'], ['normal', 'Normal'], ['high', 'High'], ['urgent', 'Urgent']];
const PRIORITY_TONE = { urgent: 'danger', high: 'warn' };
const RESOLUTIONS = { actioned: 'Actioned', no_action: 'No action', duplicate: 'Duplicate' };

const caseHref = (id) => `/case?id=${id}`;
const caseUpload = (file) => uploadMedia(file, '/api/cases/media');
// Only the console's own case uploads load in a comment.
const caseMedia = (src) => (src.startsWith('/api/cases/media/') ? src : '');
const MARKDOWN_HINT = 'Markdown works: **bold**, *italic*, - lists, [links](https://...). Paste or drop in images, GIFs and videos.';

// A comment or note: its rendered Markdown, or the text as written.
function richText(html, text) {
  if (!html) return h('div', { class: 'comment-text' }, text);
  return h('div', { class: 'comment-text md' }, previewNodes(new DOMParser().parseFromString(html, 'text/html').body.childNodes, caseMedia));
}
const nameKey = (name) => (name || '').trim().toLowerCase().replace(/_/g, ' ');
const isMe = (name) => nameKey(name) === nameKey(state.session.name);
const worldOf = (node) => {
  const w = state.worlds.find((x) => x.node === node);
  return w ? worldName(w) : `Node ${node}`;
};
const where = (x, z, level) => `${x}, ${z}${level ? ` · floor ${level}` : ''}`;
const listNames = (names) => (names.length <= 2 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);
const evidenceKey = (e) => `${e.type}:${e.id}`;
const clip = (text, n) => (text.length > n ? `${text.slice(0, n - 1)}…` : text);

function statusBadge(c) {
  const [label, tone] = CASE_STATUS[c.status] || [c.status, ''];
  return h('span', { class: `badge ${tone}` }, c.status === 'closed' && c.resolution ? `${label} · ${RESOLUTIONS[c.resolution]}` : label);
}

// Shown only when it's out of the ordinary, unless `always`.
function priorityBadge(priority, always) {
  if (!always && (priority === 'normal' || priority === 'low')) return null;
  return h('span', { class: `badge ${PRIORITY_TONE[priority] || ''}` }, PRIORITIES.find(([k]) => k === priority)?.[1] || priority);
}

const caseLinks = (ids) => (ids || []).map((id) => link(caseHref(id), { class: 'badge accent', title: `Evidence in case #${id}` }, icon('briefcase'), `#${id}`));

function avatars(names) {
  return h('div', { class: 'avatars', title: names.join(', ') },
    names.slice(0, 4).map((n) => h('div', { class: 'avatar' }, initial(n))),
    names.length > 4 && h('span', { class: 'num muted' }, `+${names.length - 4}`));
}

function evidenceCounts(c) {
  const n = { report: 0, replay: 0, chat: 0 };
  for (const e of c.evidence) n[e.type]++;
  return [n.report && plural(n.report, 'report'), n.replay && plural(n.replay, 'session'), n.chat && plural(n.chat, 'message')].filter(Boolean).join(' · ');
}

function caseItem(c, extra) {
  const closed = c.status === 'closed';
  const tone = closed ? 'ok' : PRIORITY_TONE[c.priority] || (c.assignees.length ? 'accent' : '');
  const counts = evidenceCounts(c);
  return h('div', { class: 'list-item' },
    h('div', { class: `list-icon ${tone}` }, icon(closed ? 'check' : 'briefcase')),
    h('div', { class: 'list-main' },
      h('div', { class: 'list-title' }, h('span', { class: 'case-id num' }, `#${c.id}`),
        link(caseHref(c.id), { class: 'name-link' }, h('b', {}, c.title)), ' ', statusBadge(c), ' ', priorityBadge(c.priority)),
      h('div', { class: 'list-sub' }, playerLink(c.player),
        closed ? ` · closed by ${c.closed_by} ${ago(c.closed_ms)}` : ` · opened by ${c.created_by} ${ago(c.created_ms)}`,
        counts && ` · ${counts}`, c.comments ? ` · ${plural(c.comments, 'comment')}` : ''),
      extra),
    h('div', { class: 'list-side stack' },
      c.assignees.length ? avatars(c.assignees) : !closed && h('span', { class: 'badge warn' }, 'Nobody'),
      h('span', { title: `Last change ${fullTime(c.updated_ms)}` }, ago(c.updated_ms))));
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
    state.caseSummary = await api('GET', '/cases/summary');
    updateNavCounts();
    for (const fn of caseListeners) fn(state.caseSummary);
  } catch (e) {
    if (e.status !== 401) console.warn(e);
  }
}

async function addRefs(id, refs) {
  let added = 0;
  let last = null;
  const problems = [];
  for (const item of refs) {
    try {
      last = await api('POST', `/cases/${id}`, { op: 'add_evidence', item });
      added++;
    } catch (e) {
      problems.push(e.message);
    }
  }
  if (added) toast(`Added ${plural(added, 'item')} to case #${id}.`);
  if (problems.length) toast(problems[0], 'error');
  loadCaseSummary();
  return last;
}

// A new case on `player`, starting with the evidence `refs` (`what` says
// what they are). Any open case on them is offered first, to add to instead.
function newCaseDialog({ player = '', title = '', refs = [], what = [], priority = 'normal' } = {}, onDone) {
  const nameInput = h('input', { class: 'input', value: player, placeholder: 'Player name', maxlength: '12', pattern: NAME_PATTERN, 'data-invalid': NAME_RULE, required: true, autocomplete: 'off', spellcheck: 'false', autofocus: !player });
  let titled = !!title;
  const titleInput = h('input', { class: 'input', value: title || player, maxlength: '100', required: true, autocomplete: 'off', placeholder: 'What it\'s about, e.g. "Gold selling at the GE"', autofocus: !!player, 'aria-label': 'Title' });
  titleInput.addEventListener('input', () => { titled = true; });
  let prio = priority;
  const prioChips = h('div', { class: 'segmented', role: 'group', 'aria-label': 'Priority' });
  const drawPrio = () => replace(prioChips, PRIORITIES.map(([k, label]) =>
    h('button', { type: 'button', 'aria-pressed': String(prio === k), on: { click: () => { prio = k; drawPrio(); } } }, label)));
  drawPrio();
  const note = h('textarea', { class: 'input', maxlength: '4000', placeholder: 'Optional. What you know so far, for whoever picks it up.' });
  const noteMedia = mediaUploader(note, caseUpload, 'Attach');
  const join = h('input', { type: 'checkbox', checked: true });
  const existing = h('div', {});
  let seq = 0;
  let close = () => {};

  const lookExisting = debounce(async () => {
    const name = nameInput.value.trim();
    const mine = ++seq;
    if (!titled) titleInput.value = name;
    if (!validName(name)) return replace(existing);
    try {
      const cases = await api('GET', `/cases?${query({ player: name })}`);
      if (mine !== seq) return;
      replace(existing, cases.length > 0 && h('div', { class: 'callout warn' }, icon('briefcase'), h('div', { class: 'grow' },
        h('div', {}, `${name} already has ${plural(cases.length, 'open case')}.`, refs.length ? ' Add to it instead?' : ''),
        h('div', { class: 'case-picks' }, cases.slice(0, 5).map((c) => h('div', { class: 'case-pick' },
          link(caseHref(c.id), { class: 'name-link', on: { click: () => close() } }, h('span', { class: 'num' }, `#${c.id}`), ` ${c.title}`),
          refs.length > 0 && button(`Add to #${c.id}`, { small: true, icon: 'plus', on: { click: async () => {
            await addRefs(c.id, refs);
            close();
            onDone?.();
          } } })))))));
    } catch {
      replace(existing);
    }
  }, 250);
  nameInput.addEventListener('input', lookExisting);
  if (player) lookExisting();

  ({ close } = openDialog({
    title: 'Open a case',
    subtitle: what.length ? `With ${what.join(', ')}.` : 'A case gathers the evidence on one player while staff look into it together.',
    body: [
      h('label', { class: 'field' }, h('span', {}, 'Player'), nameInput),
      existing,
      h('label', { class: 'field' }, h('span', {}, 'Title'), titleInput),
      h('div', { class: 'field' }, h('span', {}, 'Priority'), prioChips),
      h('div', { class: 'field' }, h('span', {}, 'First comment'), note,
        h('div', { class: 'news-tools' }, noteMedia.button, noteMedia.status, noteMedia.input), h('span', { class: 'hint' }, MARKDOWN_HINT)),
      h('label', { class: 'check-row' }, join, h('span', {}, 'Put me on it')),
    ],
    actions: [{ label: 'Cancel' }, {
      label: 'Open case',
      kind: 'primary',
      icon: 'briefcase',
      run: async () => {
        if (noteMedia.busy()) throw new Error('Wait for the upload to finish.');
        const c = await api('POST', '/cases', { player: nameInput.value.trim(), title: titleInput.value.trim(), priority: prio, note: note.value, evidence: refs, join: join.checked });
        toast(`Opened case #${c.id}.`);
        loadCaseSummary();
        onDone?.(c);
        navigate(caseHref(c.id));
      },
    }],
  }));
}

// Adds `refs` to an open case, those on `player` listed first.
function addToCaseDialog({ refs, player = '', what = [], title }, onDone) {
  const search = h('input', { class: 'input', type: 'search', placeholder: 'Search open cases by title, player or #', autofocus: true, 'aria-label': 'Search cases' });
  const list = h('div', { class: 'list pick-list' }, h('div', { class: 'empty' }, h('div', { class: 'spinner' })));
  let cases = [];
  let close = () => {};
  const draw = () => {
    const q = search.value.trim().toLowerCase();
    const shown = cases
      .filter((c) => !q || c.title.toLowerCase().includes(q) || c.player.toLowerCase().includes(q) || `#${c.id}` === q || String(c.id) === q)
      .sort((a, b) => (nameKey(b.player) === nameKey(player)) - (nameKey(a.player) === nameKey(player)));
    replace(list, shown.length
      ? shown.map((c) => caseItem(c, h('div', { class: 'chips item-actions' }, button(`Add to #${c.id}`, { small: true, icon: 'plus', on: { click: async () => {
        await addRefs(c.id, refs);
        close();
        onDone?.();
      } } }))))
      : h('div', { class: 'empty' }, icon('briefcase'), q ? 'No open case matches.' : 'No open cases.'));
  };
  api('GET', '/cases').then((rows) => { cases = rows; draw(); }).catch((e) => replace(list, h('div', { class: 'empty' }, e.message)));
  search.addEventListener('input', draw);
  ({ close } = openDialog({
    title: 'Add to a case',
    subtitle: what.length ? `Adding ${what.join(', ')}.` : null,
    width: '600px',
    body: [h('div', { class: 'search' }, icon('search'), search), list],
    actions: [{ label: 'Cancel' }, { label: 'New case instead', icon: 'briefcase', run: async () => newCaseDialog({ player, refs, what, title }, onDone) }],
  }));
}

async function setTriage(key, dismissed, onDone) {
  try {
    await api('POST', '/triage', { key, dismissed });
    toast(dismissed ? 'Dismissed: out of the queue.' : 'Back in the queue.');
    onDone?.();
  } catch (e) {
    toast(e.message, 'error');
  }
}

// What can be done with a report or flagged session in the queue.
function triageButtons({ key, refs, player, what, title, priority, triage, onDone }) {
  const inCase = triage.cases?.length > 0;
  return h('div', { class: 'chips item-actions' },
    button('Open case', { small: true, icon: 'briefcase', on: { click: () => newCaseDialog({ player, refs, what, title, priority }, onDone) } }),
    button('Add to case', { small: true, icon: 'plus', on: { click: () => addToCaseDialog({ refs, player, what, title }, onDone) } }),
    !inCase && (triage.dismissed
      ? button('Restore', { small: true, icon: 'undo', title: `Dismissed by ${triage.dismissed.by} ${ago(triage.dismissed.at_ms)}`, on: { click: () => setTriage(key, false, onDone) } })
      : button('Dismiss', { small: true, icon: 'x', title: 'Looked at, nothing to do: takes it out of the queue', on: { click: () => setTriage(key, true, onDone) } })));
}

// The recording of `name` that was going at `atMs`, from a minute before:
// what a report's snapshot covers.
async function watchAt(name, atMs) {
  try {
    const [r] = await api('GET', `/replays?${query({ name, before: atMs + 1 })}`);
    if (!r || (!r.live && r.started_ms + r.duration_ms < atMs - 60000)) {
      toast(`No recording of ${name} covers then. Recordings are kept for a few days.`, 'error');
      return;
    }
    navigate(`/replay?${query({ id: r.id, t: Math.max(0, atMs - r.started_ms - 60000) })}`);
  } catch (e) {
    toast(e.message, 'error');
  }
}

const watchLink = (name, atMs, label) => h('button', { type: 'button', class: 'link-button', title: `${name}'s recorded session, from a minute before`, on: { click: () => watchAt(name, atMs) } }, icon('play'), label);

function dismissedBadge(d) {
  return d && h('span', { class: 'badge', title: `By ${d.by} ${ago(d.at_ms)}` }, 'Dismissed');
}

// `extra`: more under it, such as what a case page can do with it.
function reportItem(r, onDone, extra) {
  const triaged = r.cases?.length || r.dismissed;
  return h('div', { class: 'list-item' },
    h('div', { class: `list-icon ${triaged ? '' : 'warn'}` }, icon('flag')),
    h('div', { class: 'list-main' },
      h('div', { class: 'list-title' }, h('b', {}, playerLink(r.reporter)), ' reported ', h('b', {}, playerLink(r.offender)),
        ' ', h('span', { class: 'badge' }, ruleName(r.reason)),
        r.muted && h('span', { class: 'badge warn', title: 'The reporter, a moderator, muted them for 48 hours with it' }, 'Muted with it'),
        caseLinks(r.cases), dismissedBadge(r.dismissed)),
      h('div', { class: 'list-sub' }, `${r.world ? `World ${r.world}` : `Node ${r.node}`} · ${where(r.x, r.z, r.level)} · `,
        watchLink(r.offender, r.at_ms, 'Their session'), ' ', watchLink(r.reporter, r.at_ms, 'Reporter\'s view')),
      onDone && triageButtons({
        key: `report:${r.id}`, refs: [{ type: 'report', id: r.id }], player: r.offender, what: ['the report'],
        title: REPORT_RULES[r.reason] || 'Abuse report', triage: r, onDone,
      }),
      extra),
    h('div', { class: 'list-side', title: fullTime(r.at_ms) }, ago(r.at_ms)));
}

function flaggedItem(r, onDone) {
  return replayItem(r, triageButtons({
    key: `replay:${r.id}`, refs: [{ type: 'replay', id: r.id }], player: r.player, what: ['the flagged session'],
    title: `Flagged session: ${FLAGS[r.flags[0]]?.[0] || 'anti-cheat'}`, priority: r.score >= 100 ? 'high' : 'normal', triage: r, onDone,
  }));
}

const card = (title, actions, ...body) => h('section', { class: 'card' },
  h('div', { class: 'card-head' }, h('h2', {}, title), actions && h('div', { class: 'actions' }, actions)), body);
const spinnerList = () => h('div', { class: 'list' }, h('div', { class: 'empty' }, h('div', { class: 'spinner' })));
const failed = (el, e) => replace(el, h('div', { class: 'empty' }, e.message));

const CENTER_TABS = [['overview', 'Overview'], ['triage', 'Triage'], ['cases', 'Cases'], ['sanctions', 'In force'], ['history', 'History']];

function moderationPage(ctx) {
  let tab = CENTER_TABS.some(([k]) => k === ctx.params.get('tab')) ? ctx.params.get('tab') : 'overview';
  const tabs = h('div', { class: 'segmented tabs center-tabs', role: 'group', 'aria-label': 'View' });
  const stats = h('div', { class: 'stats' });
  const body = h('div', { class: 'center-body' });
  let queue = null;
  let sanctions = null;
  let params = ctx.params;

  const go = (next, extra = {}) => {
    tab = next;
    params = new URLSearchParams(query(extra));
    const q = query({ tab: next === 'overview' ? null : next, ...extra });
    history.replaceState(null, '', `/moderation${q ? `?${q}` : ''}`);
    draw();
  };

  const statLink = (iconName, label, value, sub, onClick) => h('button', { type: 'button', class: 'card stat stat-link', on: { click: onClick } },
    h('div', { class: 'label' }, icon(iconName), label), h('div', { class: 'value num' }, value), h('div', { class: 'sub' }, sub));

  const drawStats = () => {
    const s = state.caseSummary;
    const queued = queue && queue.reports.length + queue.flagged.length;
    const capped = queue && (queue.reports.length >= 50 || queue.flagged.length >= 50);
    replace(stats,
      statLink('briefcase', 'Open cases', s ? s.open : '-', s ? `${s.urgent} high priority · ${s.waiting} waiting` : '', () => go('cases')),
      statLink('user', 'Your cases', s ? s.yours : '-', s ? (s.unassigned ? `${plural(s.unassigned, 'case')} with nobody on it` : 'Every open case has someone') : '', () => go('cases', { show: 'mine' })),
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
    queue = reports.status === 'fulfilled' && flagged.status === 'fulfilled' ? { reports: reports.value, flagged: flagged.value } : null;
    sanctions = sanctioned.status === 'fulfilled' ? sanctioned.value : null;
    drawStats();
    return { reports, flagged };
  };

  const overview = () => {
    const mine = spinnerList();
    const nobody = spinnerList();
    const triage = spinnerList();
    const actions = spinnerList();
    const closed = spinnerList();
    const take = (c) => button('Take it', { small: true, icon: 'plus', on: { click: async () => {
      try {
        await api('POST', `/cases/${c.id}`, { op: 'join' });
        toast(`You're on case #${c.id}.`);
        loadCaseSummary();
        navigate(caseHref(c.id));
      } catch (e) { toast(e.message, 'error'); }
    } } });
    const load = async () => {
      api('GET', '/cases?mine=true').then((rows) => replace(mine, rows.length ? rows.map((c) => caseItem(c))
        : h('div', { class: 'empty' }, icon('briefcase'), 'You\'re not on any open case.'))).catch((e) => failed(mine, e));
      api('GET', '/cases?unassigned=true').then((rows) => replace(nobody, rows.length ? rows.slice(0, 8).map((c) => caseItem(c, h('div', { class: 'chips item-actions' }, take(c))))
        : h('div', { class: 'empty' }, icon('check'), 'Every open case has someone on it.'))).catch((e) => failed(nobody, e));
      api('GET', '/lookup/moderation').then((rows) => replace(actions, rows.length ? rows.slice(0, 8).map(moderationItem)
        : h('div', { class: 'empty' }, 'No moderation on record.'))).catch((e) => failed(actions, e));
      api('GET', '/cases?status=closed').then((rows) => replace(closed, rows.length ? rows.slice(0, 5).map((c) => caseItem(c))
        : h('div', { class: 'empty' }, 'No closed cases yet.'))).catch((e) => failed(closed, e));
      const { reports, flagged } = await loadCounts();
      if (reports.status === 'rejected' || flagged.status === 'rejected') {
        failed(triage, reports.reason || flagged.reason);
        return;
      }
      const rows = [
        ...reports.value.map((r) => ({ at: r.at_ms, el: () => reportItem(r, load) })),
        ...flagged.value.map((r) => ({ at: r.started_ms, el: () => flaggedItem(r, load) })),
      ].sort((a, b) => b.at - a.at).slice(0, 8);
      replace(triage, rows.length ? rows.map((r) => r.el()) : h('div', { class: 'empty' }, icon('check'), 'Nothing waiting: every report and flagged session is in a case or dismissed.'));
    };
    load();
    const more = (label, next, extra) => button(label, { small: true, kind: 'ghost', on: { click: () => go(next, extra) } });
    return { load, el: h('div', { class: 'split' },
      h('div', { class: 'column' },
        card('Your cases', more('All open', 'cases'), mine),
        card('Nobody on these yet', more('Cases', 'cases', { show: 'unassigned' }), nobody),
        card('Recently closed', more('Closed', 'cases', { show: 'closed' }), closed)),
      h('div', { class: 'column' },
        card('To triage', more('Queue', 'triage'), triage),
        card('Recent actions', more('History', 'history'), actions))) };
  };

  const triageTab = () => {
    let kind = params.get('kind') === 'sessions' ? 'sessions' : 'reports';
    let all = params.get('all') === '1';
    const nameInput = h('input', { class: 'input', type: 'search', placeholder: kind === 'reports' ? 'Against (optional)' : 'Player (optional)', 'aria-label': 'Player', value: params.get('name') || '', maxlength: '12' });
    const kinds = h('div', { class: 'chips' });
    const show = h('div', { class: 'chips' });
    const list = h('div', {});
    const note = h('p', { class: 'muted' });
    const load = () => {
      const name = nameInput.value.trim();
      if (name && !validName(name)) return replace(list, h('div', { class: 'empty' }, 'That isn\'t a valid player name.'));
      const q = query({ tab: 'triage', kind: kind === 'sessions' ? 'sessions' : null, all: all ? 1 : null, name });
      history.replaceState(null, '', `/moderation?${q}`);
      nameInput.placeholder = kind === 'reports' ? 'Against (optional)' : 'Player (optional)';
      note.textContent = kind === 'reports'
        ? 'Abuse reports from the in-game Report Abuse screen. Each carries the minute before it: watch the offender\'s session, or the reporter\'s view.'
        : 'Sessions the anti-cheat checks scored as suspicious. They flag for review only; nobody is banned automatically.';
      if (!all) note.textContent += ' Open a case, add to one, or dismiss what needs nothing, to empty the queue.';
      let paged;
      paged = kind === 'reports'
        ? pagedList((before) => api('GET', `/lookup/reports?${query({ name, before, untriaged: all ? null : true })}`),
          (r) => reportItem(r, () => { paged.reload(); loadCounts(); }), all ? 'No reports.' : 'Nothing to triage.')
        : pagedList((before) => api('GET', `/replays?${query({ name, flagged: true, before, untriaged: all ? null : true })}`),
          (r) => flaggedItem(r, () => { paged.reload(); loadCounts(); }), all ? 'No flagged sessions.' : 'Nothing to triage.', (r) => r.started_ms);
      replace(list, paged.el);
    };
    const chip = (label, pressed, onClick, iconName) => h('button', { type: 'button', class: 'chip', 'aria-pressed': String(pressed), on: { click: onClick } }, iconName && icon(iconName), label);
    const drawChips = () => {
      replace(kinds,
        chip('Reports', kind === 'reports', () => { kind = 'reports'; drawChips(); load(); }, 'flag'),
        chip('Flagged sessions', kind === 'sessions', () => { kind = 'sessions'; drawChips(); load(); }, 'alert'));
      replace(show,
        chip('Needs triage', !all, () => { all = false; drawChips(); load(); }),
        chip('Everything', all, () => { all = true; drawChips(); load(); }));
    };
    nameInput.addEventListener('input', debounce(load, 400));
    drawChips();
    load();
    return { load, el: [
      h('div', { class: 'toolbar' }, h('div', { class: 'search' }, icon('search'), nameInput), kinds, h('span', { class: 'toolbar-gap' }), show),
      note,
      h('section', { class: 'card' }, list)] };
  };

  const casesTab = () => {
    const SHOWS = [['open', 'Open'], ['mine', 'Yours'], ['unassigned', 'Nobody on it'], ['waiting', 'Waiting'], ['closed', 'Closed'], ['all', 'All']];
    let show = SHOWS.some(([k]) => k === params.get('show')) ? params.get('show') : 'open';
    const search = h('input', { class: 'input', type: 'search', placeholder: 'Title, player or #', 'aria-label': 'Search cases', value: params.get('q') || '' });
    const chips = h('div', { class: 'chips' });
    const list = spinnerList();
    const count = h('span', { class: 'muted' });
    let seq = 0;
    const load = async () => {
      const mine = ++seq;
      const q = search.value.trim();
      history.replaceState(null, '', `/moderation?${query({ tab: 'cases', show: show === 'open' ? null : show, q })}`);
      const filter = { mine: { mine: true }, unassigned: { unassigned: true }, waiting: { status: 'waiting' }, closed: { status: 'closed' }, all: { status: 'all' } }[show] || {};
      try {
        const rows = await api('GET', `/cases?${query({ ...filter, q })}`);
        if (mine !== seq) return;
        count.textContent = plural(rows.length, 'case');
        replace(list, rows.length ? rows.map((c) => caseItem(c)) : h('div', { class: 'empty' }, icon('briefcase'), q ? 'No case matches.' : 'No cases here.'));
      } catch (e) {
        if (mine === seq) failed(list, e);
      }
    };
    const drawChips = () => replace(chips, SHOWS.map(([k, label]) =>
      h('button', { type: 'button', class: 'chip', 'aria-pressed': String(show === k), on: { click: () => { show = k; drawChips(); load(); } } }, label)));
    search.addEventListener('input', debounce(load, 250));
    drawChips();
    load();
    return { load, el: [
      h('div', { class: 'toolbar' }, h('div', { class: 'search' }, icon('search'), search), chips),
      h('section', { class: 'card' }, h('div', { class: 'card-head' }, h('h2', {}, 'Cases'), h('div', { class: 'actions' }, count)), list)] };
  };

  const sanctionItem = (s, reload) => h('div', { class: 'list-item' },
    h('div', { class: `list-icon ${s.banned_until_ms ? 'danger' : 'warn'}` }, icon(s.banned_until_ms ? 'ban' : 'mute')),
    h('div', { class: 'list-main' },
      h('div', { class: 'list-title' }, h('b', {}, playerLink(s.name)), ' ',
        s.banned_until_ms && h('span', { class: 'badge danger' }, `Banned · ${until(s.banned_until_ms)}`), ' ',
        s.muted_until_ms && h('span', { class: 'badge warn' }, `Muted · ${until(s.muted_until_ms)}`)),
      h('div', { class: 'list-sub' }, s.moderator
        ? `${s.last_action === 'ban' ? 'Banned' : 'Muted'} by ${s.moderator} ${ago(s.at_ms)}`
        : 'No moderator on record (abuse report or older)')),
    h('div', { class: 'chips' },
      s.muted_until_ms && button('Unmute', { small: true, icon: 'undo', on: { click: () => moderateDialog(s.name, 'unmute', reload) } }),
      s.banned_until_ms && button('Unban', { small: true, icon: 'undo', on: { click: () => moderateDialog(s.name, 'unban', reload) } })));

  const sanctionsTab = () => {
    const list = spinnerList();
    const count = h('span', { class: 'muted' });
    const load = async () => {
      try {
        const rows = await api('GET', '/lookup/sanctions');
        sanctions = rows;
        drawStats();
        count.textContent = plural(rows.length, 'player');
        replace(list, rows.length ? rows.map((s) => sanctionItem(s, load)) : h('div', { class: 'empty' }, icon('shield'), 'Nobody is muted or banned.'));
      } catch (e) {
        failed(list, e);
      }
    };
    load();
    return { load, el: h('section', { class: 'card' }, h('div', { class: 'card-head' }, h('h2', {}, 'In force now'), h('div', { class: 'actions' }, count)), list) };
  };

  const historyTab = () => {
    const paged = pagedList((before) => api('GET', `/lookup/moderation?${query({ before })}`), moderationItem, 'No moderation on record.');
    return { load: paged.reload, el: h('section', { class: 'card' }, h('div', { class: 'card-head' }, h('h2', {}, 'Every action, in game or here, newest first')), paged.el) };
  };

  let current = null;
  const draw = () => {
    replace(tabs, CENTER_TABS.map(([k, label]) =>
      h('button', { type: 'button', 'aria-pressed': String(tab === k), on: { click: () => go(k) } }, label)));
    current = { overview, triage: triageTab, cases: casesTab, sanctions: sanctionsTab, history: historyTab }[tab]();
    replace(body, current.el);
  };
  const reload = () => current?.load();

  drawStats();
  loadCaseSummary();
  if (tab !== 'overview') loadCounts().catch(() => {});
  ctx.cleanup(onCases(drawStats));
  // Kept fresh while it's open, except under a dialog or on paged lists.
  const timer = setInterval(() => {
    if (document.hidden || document.querySelector('dialog[open]')) return;
    if (tab === 'overview') reload();
    else loadCounts().catch(() => {});
  }, 30000);
  ctx.cleanup(() => clearInterval(timer));
  draw();

  return h('div', { class: 'page' },
    pageHead('Moderation center', 'Cases staff work on together, the reports and flagged sessions waiting to be looked at, and every action taken.',
      button('Moderate by name', { icon: 'shield', on: { click: () => moderateDialog('', 'mute', reload) } }),
      button('Open a case', { icon: 'briefcase', kind: 'primary', on: { click: () => newCaseDialog({}, reload) } })),
    stats,
    tabs,
    body);
}

// ---------------------------------------------------------------------------
// A case: its evidence, who's on it, and a timeline of everything done,
// comments included. It refreshes every few seconds, so staff on it together
// see each other's changes, and who else has it open.

function evidenceLabel(e) {
  if (e.type === 'report') return `${e.reporter}'s report of ${e.offender} (${REPORT_RULES[e.reason] || 'unknown rule'})`;
  if (e.type === 'replay') return `${e.player}'s session of ${fullTime(e.started_ms)}`;
  return `${e.from}'s message "${clip(e.message, 60)}"`;
}

function evidenceItem(e, onRemove) {
  const remove = onRemove && button(null, { small: true, kind: 'ghost', icon: 'x', 'aria-label': 'Remove from the case', title: 'Remove from the case', on: { click: onRemove } });
  const row = (iconName, tone, title, sub, side) => h('div', { class: 'list-item' },
    h('div', { class: `list-icon ${tone}` }, icon(iconName)),
    h('div', { class: 'list-main' }, h('div', { class: 'list-title' }, title), h('div', { class: 'list-sub' }, sub)),
    h('div', { class: 'list-side', title: fullTime(side) }, ago(side)),
    remove);
  if (e.type === 'report') {
    return row('flag', 'warn',
      [h('b', {}, playerLink(e.reporter)), ' reported ', h('b', {}, playerLink(e.offender)), ' ', h('span', { class: 'badge' }, ruleName(e.reason))],
      [`${worldOf(e.node)} · ${where(e.x, e.z, e.level)} · `, watchLink(e.offender, e.at_ms, 'Their session'), ' ', watchLink(e.reporter, e.at_ms, 'Reporter\'s view')],
      e.at_ms);
  }
  if (e.type === 'replay') {
    return row(e.score > 0 ? 'alert' : 'play', e.score > 0 ? 'danger' : '',
      [link(`/replay?${query({ id: e.id })}`, { class: 'name-link' }, h('b', {}, `${e.player}'s session`)), ' ', e.flags.map(flagBadge)],
      `${worldOf(e.node)} · ${fullTime(e.started_ms)} · ${clock(e.duration_ms / 1000)}${e.score > 0 ? ` · score ${e.score}` : ''}`,
      e.started_ms);
  }
  return row('chat', '',
    [h('b', {}, playerLink(e.from)), e.to && [h('span', { class: 'muted' }, ' to '), h('b', {}, playerLink(e.to))]],
    [h('span', { class: `chat-text ${e.to ? 'private' : ''}` }, e.message), h('br'), `${worldOf(e.node)} · ${fullTime(e.at_ms)}`],
    e.at_ms);
}

// One timeline entry, in words: [what, icon, tone, body].
function describeEntry(e, timeline) {
  const b = (text) => h('b', {}, text);
  const text = (t) => t && richText(e.html, t);
  switch (e.kind) {
    case 'opened': return [['opened the case on ', b(e.player)], 'briefcase', 'accent'];
    case 'comment': return ['', 'chat', '', text(e.text)];
    case 'assigned': return [e.name === e.by ? 'joined the case' : ['put ', b(e.name), ' on the case'], 'plus', 'accent'];
    case 'unassigned': return [e.name === e.by ? 'left the case' : ['took ', b(e.name), ' off the case'], 'logout', ''];
    case 'status': return [['set the status to ', b(CASE_STATUS[e.status]?.[0] || e.status)], 'activity', ''];
    case 'priority': return [['set the priority to ', b(PRIORITIES.find(([k]) => k === e.priority)?.[1] || e.priority)], 'alert', PRIORITY_TONE[e.priority] || ''];
    case 'retitled': return [['renamed it ', b(`"${e.title}"`)], 'edit', ''];
    case 'evidence_added': return [['added ', b(evidenceLabel(e.evidence))], 'clip', ''];
    case 'evidence_removed': {
      const was = timeline.find((x) => x.kind === 'evidence_added' && evidenceKey(x.evidence) === e.key);
      return [['removed ', b(was ? evidenceLabel(was.evidence) : e.key)], 'trash', ''];
    }
    case 'action': {
      const [verb, iconName, tone] = MOD_VERBS[e.action] || [e.action, 'shield', ''];
      const what = e.ok
        ? [`${verb} `, playerLink(e.name), e.hours ? ` for ${hoursLabel(e.hours)}` : '']
        : [`tried to ${e.action} `, playerLink(e.name), ' ', h('span', { class: 'badge danger' }, 'Failed')];
      // The reason starts with the case it came from, which is this one.
      const why = (e.reason || '').replace(/^Case #\d+:?\s*/, '');
      return [what, iconName, e.ok ? tone : 'danger', h('div', { class: 'list-sub' }, e.message, why && ` Reason: ${why}`)];
    }
    case 'closed': return [['closed it: ', b(RESOLUTIONS[e.resolution] || e.resolution)], 'check', 'ok', text(e.note)];
    case 'reopened': return ['reopened the case', 'undo', 'accent'];
    default: return [e.kind.replace(/_/g, ' '), 'activity', ''];
  }
}

function caseEntry(e, timeline) {
  const [what, iconName, tone, body] = describeEntry(e, timeline);
  const comment = e.kind === 'comment';
  return h('div', { class: `list-item case-entry ${comment ? 'comment' : ''}` },
    comment ? h('div', { class: 'avatar' }, initial(e.by)) : h('div', { class: `list-icon ${tone}` }, icon(iconName)),
    h('div', { class: 'list-main' }, h('div', { class: 'list-title' }, h('b', {}, e.by), ' ', what), body),
    h('div', { class: 'list-side', title: fullTime(e.at_ms) }, ago(e.at_ms)));
}

// Evidence to add to case `c`: reports against a player (the case's, or
// anyone's, such as the other side of a scam), their sessions, or chat.
function evidenceDialog(c, onAdded) {
  let kind = 'reports';
  const have = new Set(c.evidence.map(evidenceKey));
  const nameInput = h('input', { class: 'input', type: 'search', value: c.player, maxlength: '12', placeholder: 'Player name', 'aria-label': 'Player' });
  const kinds = h('div', { class: 'segmented' });
  const body = h('div', { class: 'pick-list' });
  const add = (item) => {
    const key = `${item.type}:${item.id}`;
    const done = have.has(key);
    const btn = button(done ? 'Added' : 'Add', { small: true, icon: done ? 'check' : 'plus', disabled: done });
    btn.addEventListener('click', async () => {
      btn.disabled = true;
      try {
        const detail = await api('POST', `/cases/${c.id}`, { op: 'add_evidence', item });
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
  const pick = (iconName, title, sub, action) => h('div', { class: 'list-item' },
    h('div', { class: 'list-icon' }, icon(iconName)),
    h('div', { class: 'list-main' }, h('div', { class: 'list-title' }, title), h('div', { class: 'list-sub' }, sub)),
    action);
  const load = () => {
    const who = nameInput.value.trim();
    if (!validName(who)) return replace(body, h('div', { class: 'empty' }, 'Enter a player name.'));
    const specs = {
      reports: [(before) => api('GET', `/lookup/reports?${query({ name: who, before })}`),
        (r) => pick('flag', [h('b', {}, r.reporter), ' reported ', h('b', {}, r.offender), ' ', h('span', { class: 'badge' }, ruleName(r.reason))],
          `${worldOf(r.node)} · ${fullTime(r.at_ms)}`, add({ type: 'report', id: r.id })),
        `No reports against ${who}.`, (r) => r.id],
      sessions: [(before) => api('GET', `/replays?${query({ name: who, before })}`),
        (r) => pick(r.score > 0 ? 'alert' : 'play', [h('b', {}, `${r.player}'s session`), ' ', r.flags.map(flagBadge)],
          `${worldOf(r.node)} · ${fullTime(r.started_ms)} · ${clock(r.duration_ms / 1000)}${r.score > 0 ? ` · score ${r.score}` : ''}`, add({ type: 'replay', id: r.id })),
        `No recorded sessions of ${who}.`, (r) => r.started_ms],
      chat: [(before) => api('GET', `/lookup/chat?${query({ name: who, before })}`),
        (m) => pick('chat', [h('b', {}, m.from), m.to && [' to ', h('b', {}, m.to)], ': ', h('span', { class: `chat-text ${m.to ? 'private' : ''}` }, m.message)],
          `${worldOf(m.node)} · ${fullTime(m.at_ms)}`, add({ type: 'chat', id: m.id })),
        `No chat from ${who}.`, (m) => m.id],
    };
    const [fetchPage, item, empty, cursor] = specs[kind];
    replace(body, pagedList(fetchPage, item, empty, cursor).el);
  };
  const drawKinds = () => replace(kinds, [['reports', 'Reports against'], ['sessions', 'Sessions'], ['chat', 'Chat']].map(([k, label]) =>
    h('button', { type: 'button', 'aria-pressed': String(kind === k), on: { click: () => { kind = k; drawKinds(); load(); } } }, label)));
  nameInput.addEventListener('input', debounce(load, 400));
  drawKinds();
  load();
  openDialog({
    title: 'Add evidence',
    subtitle: `To case #${c.id}. Looking at chat here is recorded in the audit log.`,
    width: '680px',
    body: [h('div', { class: 'evidence-tools' }, h('div', { class: 'search' }, icon('search'), nameInput), kinds), body],
    actions: [{ label: 'Done' }],
  });
}

function casePage(ctx) {
  const id = Number(ctx.params.get('id'));
  const page = h('div', { class: 'page' });
  const head = h('div', {}, pageHead(`Case #${ctx.params.get('id') || ''}`, 'Loading...'));
  append(page, head);
  if (!Number.isInteger(id) || id < 1) {
    replace(head, pageHead('Case', 'No such case.'));
    return page;
  }

  const presence = h('div', { class: 'presence hidden', role: 'status' });
  const evidenceList = h('div', { class: 'list' });
  const evidenceCount = h('span', { class: 'muted' });
  const timeline = h('div', { class: 'list case-timeline' });
  const playerBox = h('div', { class: 'card-body' }, h('div', { class: 'spinner' }));
  const caseBox = h('div', { class: 'card-body' });
  const others = spinnerList();
  const reportsBox = h('div', {}, spinnerList());
  const pastActions = spinnerList();
  const layout = h('div', { class: 'case-layout hidden' });
  let c = null;
  let record;

  const draftKey = `caseDraft:${id}`;
  const composer = h('textarea', { class: 'input composer-text', maxlength: '4000', placeholder: 'Write a comment for everyone on the case. Markdown works; paste or drop in images, GIFs and videos.', 'aria-label': 'Comment', value: prefs.get(draftKey, '') });
  const composerMedia = mediaUploader(composer, caseUpload, 'Attach');
  const send = button('Comment', { kind: 'primary', icon: 'chat' });
  const previewBox = h('div', { class: 'composer-preview hidden' });
  const previewButton = button('Preview', { small: true, kind: 'ghost', icon: 'eye', 'aria-pressed': 'false' });
  let previewing = false;
  const setPreview = async (on) => {
    previewing = on;
    previewButton.setAttribute('aria-pressed', String(on));
    composer.classList.toggle('hidden', on);
    previewBox.classList.toggle('hidden', !on);
    if (!on) return composer.focus();
    replace(previewBox, h('div', { class: 'spinner' }));
    try {
      const { html } = await api('POST', '/cases/preview', { markdown: composer.value });
      replace(previewBox, composer.value.trim() ? richText(html, composer.value) : h('p', { class: 'muted' }, 'Nothing to preview yet.'));
    } catch (e) {
      replace(previewBox, h('p', { class: 'error-text' }, e.message));
    }
  };
  previewButton.addEventListener('click', () => setPreview(!previewing));
  const addEvidence = button('Add evidence', { small: true, icon: 'plus' });

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
    const input = h('input', { class: 'input', value: c.title, maxlength: '100', required: true, autofocus: true, autocomplete: 'off' });
    openDialog({
      title: 'Rename case',
      subtitle: `#${c.id}`,
      body: [h('label', { class: 'field' }, h('span', {}, 'Title'), input)],
      actions: [{ label: 'Cancel' }, { label: 'Rename', kind: 'primary', run: () => op({ op: 'retitle', title: input.value.trim() }) }],
    });
  };

  const closeDialog = () => {
    let resolution = c.actions > 0 ? 'actioned' : 'no_action';
    const seg = h('div', { class: 'segmented', role: 'group', 'aria-label': 'Outcome' });
    const hint = h('p', { class: 'hint' });
    const note = h('textarea', { class: 'input', maxlength: '4000', placeholder: 'What was found and what was done. Optional, but the next person to look will thank you.' });
    const noteMedia = mediaUploader(note, caseUpload, 'Attach');
    const draw = () => {
      replace(seg, Object.entries(RESOLUTIONS).map(([k, label]) =>
        h('button', { type: 'button', 'aria-pressed': String(resolution === k), on: { click: () => { resolution = k; draw(); } } }, label)));
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
        h('div', { class: 'field' }, h('span', {}, 'Outcome'), seg, hint),
        h('div', { class: 'field' }, h('span', {}, 'Note'), note,
          h('div', { class: 'news-tools' }, noteMedia.button, noteMedia.status, noteMedia.input), h('span', { class: 'hint' }, MARKDOWN_HINT)),
        c.assignees.length > 1 && h('div', { class: 'callout' }, icon('users'), h('div', {}, `Closes it for everyone on it: ${listNames(c.assignees)}.`)),
      ],
      actions: [{ label: 'Cancel' }, { label: 'Close case', kind: 'primary', icon: 'check', run: async () => {
        if (noteMedia.busy()) throw new Error('Wait for the upload to finish.');
        await op({ op: 'close', resolution, note: note.value });
        toast(`Closed case #${c.id}.`);
      } }],
    });
  };

  const assignDialog = async () => {
    const list = h('div', { class: 'chips' }, h('div', { class: 'spinner' }));
    const { close } = openDialog({ title: 'Put staff on the case', subtitle: 'It shows under Your cases for them.', body: [list], actions: [{ label: 'Done' }] });
    try {
      const staff = await api('GET', '/cases/staff');
      const free = staff.filter((s) => !c.assignees.some((a) => nameKey(a) === nameKey(s.name)));
      replace(list, free.length ? free.map((s) => button(s.name, { small: true, icon: 'plus', title: s.role, on: { click: async () => {
        try {
          await op({ op: 'assign', name: s.name });
          toast(`${s.name} is on the case.`);
          close();
        } catch (e) { toast(e.message, 'error'); }
      } } })) : h('p', { class: 'muted' }, 'Everyone is on it already.'));
    } catch (e) {
      replace(list, h('p', { class: 'error-text' }, e.message));
    }
  };

  const drawHead = () => {
    const on = c.assignees.some(isMe);
    replace(head, pageHead(
      h('span', {}, h('span', { class: 'case-id num' }, `#${c.id}`), c.title, ' ', statusBadge(c), ' ', priorityBadge(c.priority)),
      h('span', {}, 'On ', playerLink(c.player), ` · opened by ${c.created_by} ${ago(c.created_ms)}`,
        c.closed_ms ? ` · closed by ${c.closed_by} ${ago(c.closed_ms)}` : ''),
      open() && button(null, { icon: 'edit', 'aria-label': 'Rename', title: 'Rename', on: { click: retitleDialog } }),
      open() && button(on ? 'Leave' : 'Join', { icon: on ? 'logout' : 'plus', on: { click: run({ op: on ? 'leave' : 'join' }, on ? 'You left the case.' : 'You\'re on the case.') } }),
      open() && button('Moderate', { icon: 'shield', on: { click: () => moderateDialog(c.player, 'mute', afterAction, c.id) } }),
      open()
        ? button('Close case', { icon: 'check', kind: 'primary', on: { click: closeDialog } })
        : button('Reopen', { icon: 'undo', on: { click: run({ op: 'reopen' }, 'Reopened.') } })));
  };

  const drawPresence = () => {
    presence.classList.toggle('hidden', !c.viewing.length);
    replace(presence, h('span', { class: 'dot ok' }), `${listNames(c.viewing)} ${c.viewing.length === 1 ? 'has' : 'have'} this case open too.`);
  };

  const drawEvidence = () => {
    evidenceCount.textContent = c.evidence.length ? plural(c.evidence.length, 'item') : '';
    addEvidence.disabled = !open();
    const remove = (e) => open() && (() => confirmDialog({
      title: 'Remove evidence',
      subtitle: evidenceLabel(e),
      body: [h('p', { class: 'muted' }, 'It comes out of the case. The timeline keeps a note that it was there.')],
      label: 'Remove',
      kind: 'danger',
      run: () => op({ op: 'remove_evidence', key: evidenceKey(e) }),
    }));
    replace(evidenceList, c.evidence.length
      ? c.evidence.map((e) => evidenceItem(e, remove(e)))
      : h('div', { class: 'empty' }, icon('clip'), 'No evidence yet: add reports, sessions or chat.'));
  };

  const drawTimeline = () => {
    const shown = timeline.children.length;
    const atEnd = timeline.getBoundingClientRect().bottom <= window.innerHeight + 40;
    replace(timeline, c.timeline.map((e) => caseEntry(e, c.timeline)));
    // New entries come into view if you were reading the end already.
    if (shown && shown < c.timeline.length && atEnd) timeline.lastElementChild?.scrollIntoView({ block: 'nearest' });
  };

  const drawCase = () => {
    const seg = (options, current, opName) => h('div', { class: 'segmented small' }, options.map(([k, label]) =>
      h('button', { type: 'button', 'aria-pressed': String(current === k), disabled: !open(), on: { click: () => current !== k && run({ op: opName, [opName]: k })() } }, label)));
    const people = h('div', { class: 'assignees' },
      c.assignees.map((n) => h('span', { class: 'assignee' }, h('span', { class: 'avatar' }, initial(n)), h('span', {}, n),
        open() && button(null, { small: true, kind: 'ghost', icon: 'x', 'aria-label': `Take ${n} off`, title: isMe(n) ? 'Leave' : `Take ${n} off`, on: { click: run({ op: 'unassign', name: n }) } }))),
      !c.assignees.length && h('span', { class: 'muted' }, 'Nobody yet.'),
      open() && button('Add', { small: true, icon: 'plus', on: { click: assignDialog } }));
    const facts = [
      ['Opened', `${fullTime(c.created_ms)} by ${c.created_by}`],
      ['Last change', ago(c.updated_ms)],
      c.closed_ms && ['Closed', `${fullTime(c.closed_ms)} by ${c.closed_by}`],
      ['Moderation sent', String(c.actions)],
      ['Comments', String(c.comments)],
    ].filter(Boolean);
    replace(caseBox,
      h('div', { class: 'field' }, h('span', {}, 'Status'), c.status === 'closed'
        ? h('div', {}, statusBadge(c))
        : seg([['open', 'Open'], ['investigating', 'Investigating'], ['waiting', 'Waiting']], c.status, 'status')),
      h('div', { class: 'field' }, h('span', {}, 'Priority'), seg(PRIORITIES, c.priority, 'priority')),
      h('div', { class: 'field' }, h('span', {}, 'On it'), people),
      h('dl', { class: 'facts' }, facts.map(([k, v]) => [h('dt', {}, k), h('dd', {}, v)])));
  };

  const drawPlayer = () => {
    const act = (action) => () => moderateDialog(c.player, action, afterAction, open() ? c.id : undefined);
    const links = h('div', { class: 'chips' },
      link(`/player?${query({ name: c.player })}`, { class: 'btn small ghost' }, icon('user'), 'Profile'),
      link(`/chat?${query({ name: c.player })}`, { class: 'btn small ghost' }, icon('chat'), 'Chat'),
      link(`/replays?${query({ name: c.player })}`, { class: 'btn small ghost' }, icon('play'), 'Sessions'),
      link(`/moderation?${query({ tab: 'triage', all: 1, name: c.player })}`, { class: 'btn small ghost' }, icon('flag'), 'Reports'));
    if (!record) {
      replace(playerBox, h('p', { class: 'muted' }, `No registered player is called ${c.player}. Reports name whoever the reporter typed.`), links);
      return;
    }
    const muted = record.muted_until_ms != null;
    const banned = record.banned_until_ms != null;
    const online = record.online_node != null;
    replace(playerBox,
      h('div', { class: 'list-title' }, h('b', {}, playerLink(record.name)), ' ', playerBadges({ staff_level: record.staff_level }),
        online ? h('span', { class: 'badge ok' }, `Online · ${worldOf(record.online_node)}`) : h('span', { class: 'badge' }, 'Offline'),
        muted && h('span', { class: 'badge warn' }, `Muted · ${until(record.muted_until_ms)}`),
        banned && h('span', { class: 'badge danger' }, `Banned · ${until(record.banned_until_ms)}`)),
      h('dl', { class: 'facts' },
        h('dt', {}, 'Registered'), h('dd', {}, new Date(record.registered_ms).toLocaleDateString()),
        h('dt', {}, 'Last seen'), h('dd', {}, online ? 'Now' : record.last_seen_ms ? ago(record.last_seen_ms) : 'Never')),
      h('div', { class: 'chips' },
        online && button('Kick', { small: true, icon: 'kick', on: { click: act('kick') } }),
        button(muted ? 'Unmute' : 'Mute', { small: true, icon: muted ? 'undo' : 'mute', on: { click: act(muted ? 'unmute' : 'mute') } }),
        button(banned ? 'Unban' : 'Ban', { small: true, icon: banned ? 'undo' : 'ban', kind: banned ? undefined : 'danger', on: { click: act(banned ? 'unban' : 'ban') } })),
      links);
  };

  const loadPlayer = async () => {
    try {
      record = await api('GET', `/lookup/player?${query({ name: c.player })}`);
      drawPlayer();
    } catch (e) {
      replace(playerBox, h('p', { class: 'error-text' }, e.message));
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
      const row = { ...r, cases: elsewhere };
      return reportItem(row, null, h('div', { class: 'chips item-actions' },
        here
          ? h('span', { class: 'badge ok' }, icon('check'), 'In this case')
          : open() && button('Add to case', { small: true, icon: 'plus', on: { click: run({ op: 'add_evidence', item: { type: 'report', id: r.id } }, 'Added to the case.') } })));
    };
    replace(reportsBox, pagedList((before) => api('GET', `/lookup/reports?${query({ name: c.player, before })}`), item, 'No abuse reports against them.').el);
  };
  const loadOthers = () => api('GET', `/cases?${query({ player: c.player, status: 'all' })}`).then((rows) => {
    const rest = rows.filter((x) => x.id !== id);
    replace(others, rest.length ? rest.map((x) => caseItem(x)) : h('div', { class: 'empty' }, 'No other cases on them.'));
  }).catch((e) => failed(others, e));
  const loadHistory = () => api('GET', `/lookup/moderation?${query({ name: c.player })}`).then((rows) => {
    replace(pastActions, rows.length ? rows.slice(0, 6).map(moderationItem) : h('div', { class: 'empty' }, 'No moderation on record.'));
  }).catch((e) => failed(pastActions, e));

  // Redraws what changed; `force` after your own change.
  const show = (detail, force) => {
    const first = !c;
    const changed = force || first || detail.updated_ms !== c.updated_ms || detail.timeline.length !== c.timeline.length;
    const playerChanged = first || detail.player !== c.player;
    c = detail;
    drawPresence();
    if (!changed) return;
    document.title = `#${c.id} ${c.title} · RS Admin`;
    layout.classList.remove('hidden');
    drawHead();
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
      await op({ op: 'comment', text });
      composer.value = '';
      prefs.set(draftKey, '');
      if (previewing) setPreview(false);
      timeline.lastElementChild?.scrollIntoView({ block: 'nearest' });
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
  const timer = setInterval(() => { if (!document.hidden) refresh(); }, 5000);
  ctx.cleanup(() => clearInterval(timer));
  // Evidence names worlds by node until the world list first arrives.
  let named = state.worldsLoaded;
  ctx.cleanup(onWorlds(() => {
    if (named || !c) return;
    named = true;
    drawEvidence();
  }));

  append(layout,
    h('div', { class: 'column' },
      card('Evidence', [evidenceCount, addEvidence], evidenceList),
      card('Reports against them', h('span', { class: 'muted' }, 'All of them, in this case or not'), reportsBox),
      h('section', { class: 'card' },
        h('div', { class: 'card-head' }, h('h2', {}, 'Timeline')),
        timeline,
        h('div', { class: 'composer' }, composer, previewBox,
          h('div', { class: 'composer-foot' },
            h('div', { class: 'news-tools' }, composerMedia.button, composerMedia.status, composerMedia.input),
            h('div', { class: 'composer-send' }, h('span', { class: 'hint' }, 'Ctrl+Enter to send. All staff can read it.'), previewButton, send))))),
    h('div', { class: 'column' },
      card('Player', null, playerBox),
      card('Case', null, caseBox),
      card('Other cases on them', null, others),
      card('Moderation history', null, pastActions)));
  append(page, presence, layout);
  return page;
}

// ---------------------------------------------------------------------------
// Chat logs

function chatPage(ctx) {
  let node = ctx.params.get('node') ? Number(ctx.params.get('node')) : null;
  const nameInput = h('input', { class: 'input', type: 'search', placeholder: 'Player name (optional)', 'aria-label': 'Player', value: ctx.params.get('name') || '', maxlength: '12' });
  const worlds = h('div', { class: 'chips' });
  const body = h('div', { class: 'chat-list' });
  const note = h('p', { class: 'muted' });

  const load = () => {
    const name = nameInput.value.trim();
    if (name && !validName(name)) {
      replace(body, h('div', { class: 'empty' }, 'That isn\'t a valid player name.'));
      return;
    }
    history.replaceState(null, '', `/chat${query({ name, node }) ? `?${query({ name, node })}` : ''}`);
    note.textContent = `${name ? `${name}'s` : 'Everyone\'s'} public chat and private messages${node ? ` on world ${state.worlds.find((w) => w.node === node)?.id ?? node}` : ', on every world'}, newest first. Viewing them is recorded in the audit log.`;
    const paged = pagedList((before) => api('GET', `/lookup/chat?${query({ name, node, before })}`), chatItem, 'No chat matches.');
    replace(body, paged.el);
  };
  const drawWorlds = () => replace(worlds,
    h('button', { type: 'button', class: 'chip', 'aria-pressed': String(node === null), on: { click: () => { node = null; drawWorlds(); load(); } } }, 'All worlds'),
    state.worlds.map((w) => h('button', { type: 'button', class: 'chip', 'aria-pressed': String(node === w.node), on: { click: () => { node = w.node; drawWorlds(); load(); } } }, worldName(w))));
  nameInput.addEventListener('input', debounce(load, 400));
  drawWorlds();
  ctx.cleanup(onWorlds(() => { if (!worlds.children.length || worlds.children.length !== state.worlds.length + 1) drawWorlds(); }));
  load();

  return h('div', { class: 'page' },
    pageHead('Chat logs', null, button('Refresh', { icon: 'refresh', on: { click: load } })),
    h('div', { class: 'toolbar' }, h('div', { class: 'search' }, icon('search'), nameInput), worlds),
    note,
    h('section', { class: 'card' }, body));
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
  cursor_jumps: ['Cursor jumps', 'Most clicks landed far from where the cursor last was: it jumps rather than moves.'],
  robotic_click_timing: ['Clockwork clicks', 'Clicks came at very even intervals.'],
};

const flagBadge = (flag) => h('span', { class: 'badge danger', title: FLAGS[flag]?.[1] || flag }, FLAGS[flag]?.[0] || flag);

function kilobytes(n) {
  if (n < 1024 * 1024) return `${Math.max(1, Math.round(n / 1024))} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

const replayHref = (r) => `/replay?${query({ id: r.id })}`;

function replayItem(r, extra) {
  const flagged = r.score > 0;
  return h('div', { class: 'list-item' },
    h('div', { class: `list-icon ${flagged ? 'danger' : r.live ? 'ok' : ''}` }, icon(flagged ? 'alert' : 'play')),
    h('div', { class: 'list-main' },
      h('div', { class: 'list-title' },
        link(replayHref(r), { class: 'name-link' }, r.player), ' ',
        r.live && h('span', { class: 'badge ok' }, 'Live'), ' ',
        !r.complete && h('span', { class: 'badge warn', title: 'Part of it was lost: it stops early.' }, 'Incomplete'), ' ',
        r.flags.map(flagBadge), caseLinks(r.cases), dismissedBadge(r.dismissed)),
      h('div', { class: 'list-sub' },
        `${r.world ? `World ${r.world}` : `Node ${r.node}`} · ${clock(r.duration_ms / 1000)} · ${kilobytes(r.bytes)}${flagged ? ` · score ${r.score}` : ''}`),
      extra),
    h('div', { class: 'list-side', title: fullTime(r.started_ms) }, ago(r.started_ms)));
}

function replaysPage(ctx) {
  let flagged = ctx.params.get('flagged') === '1';
  const nameInput = h('input', { class: 'input', type: 'search', placeholder: 'Player name (optional)', 'aria-label': 'Player', value: ctx.params.get('name') || '', maxlength: '12' });
  const filter = h('div', { class: 'chips' });
  const body = h('div', {});

  const load = () => {
    const name = nameInput.value.trim();
    if (name && !validName(name)) {
      replace(body, h('div', { class: 'empty' }, 'That isn\'t a valid player name.'));
      return;
    }
    const q = query({ name, flagged: flagged ? 1 : null });
    history.replaceState(null, '', `/replays${q ? `?${q}` : ''}`);
    const paged = pagedList(
      // The client for these starts loading now (see dockWarm).
      (before) => api('GET', `/replays?${query({ name, flagged: flagged || null, before })}`).then((rows) => { if (!before) dockWarm(rows); return rows; }),
      replayItem,
      flagged ? 'No flagged sessions.' : 'No recorded sessions.',
      (r) => r.started_ms);
    replace(body, paged.el);
  };
  const drawFilter = () => replace(filter,
    h('button', { type: 'button', class: 'chip', 'aria-pressed': String(!flagged), on: { click: () => { flagged = false; drawFilter(); load(); } } }, 'All sessions'),
    h('button', { type: 'button', class: 'chip', 'aria-pressed': String(flagged), on: { click: () => { flagged = true; drawFilter(); load(); } } }, icon('alert'), 'Flagged by the checks'));
  nameInput.addEventListener('input', debounce(load, 400));
  drawFilter();
  load();

  return h('div', { class: 'page' },
    pageHead('Replays', 'Every session is recorded and kept for a while. Watch one to see the game as the player did, with their mouse.',
      button('Refresh', { icon: 'refresh', on: { click: load } })),
    h('div', { class: 'toolbar' }, h('div', { class: 'search' }, icon('search'), nameInput), filter),
    h('section', { class: 'card' }, body));
}

// The checks' measurements, in words.
function replayStats(r) {
  let s = {};
  try { s = JSON.parse(r.stats || '{}'); } catch { /* none */ }
  const sum = (a) => (a || []).reduce((x, y) => x + y, 0);
  const rows = [
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
  return h('dl', { class: 'replay-stats' }, rows.map(([k, v]) => [h('dt', {}, k), h('dd', { class: 'num' }, String(v))]));
}

const SPEEDS = [0.5, 1, 2, 4, 8];

// Finished recordings already downloaded this visit, by id: reopening one
// starts at once. Kept in memory only (they hold private messages), up to
// REPLAY_CACHE_BYTES, the oldest dropped first.
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
  ready: { main: false, scout: false }, size: null,
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
    ? { class: 'replay-scout', src: src.href, tabindex: '-1', 'aria-hidden': 'true', title: 'Replay previews', referrerpolicy: 'no-referrer' }
    : { class: 'replay-frame', src: src.href, title: 'Replay', referrerpolicy: 'no-referrer' });
}

// A client loaded for `url` (a world's replay page), and its scout if asked.
function dockEnsure(url, withScout) {
  if (dock.url !== url) dockLetGo();
  if (!dock.el) {
    dock.url = url;
    dock.el = h('div', { class: 'replay-dock parked' });
    dock.main = dockFrame(url, false);
    dock.el.append(dock.main);
    dock.ready = { main: false, scout: false };
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
  Object.assign(dock, { url: null, el: null, main: null, scout: null, ready: { main: false, scout: false }, size: null, page: null, box: null, raf: 0, letGo: null });
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
window.addEventListener('scroll', () => dock.page && dock.place?.(), { passive: true, capture: true });

// Done with it: the replay logs out, and the client waits for the next.
function dockDetach(page) {
  if (!dock.el || dock.page !== page) return;
  cancelAnimationFrame(dock.raf);
  dock.page = null;
  dock.box = null;
  dockPost('main', { type: 'unload' });
  dockPost('scout', { type: 'unload' });
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
  } catch { /* only a head start */ }
}

function replayPage(ctx) {
  const id = ctx.params.get('id') || '';
  const page = h('div', { class: 'page replay-page' });
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
  // The game's own size: 789x532 on 225, 765x503 after.
  let gameW = 765;
  let gameH = 503;
  let sizing = prefs.get('replaySize', 'sharp');

  const stage = h('div', { class: 'replay-stage' }, h('div', { class: 'empty' }, h('div', { class: 'spinner' })));
  const status = h('span', { class: 'muted replay-status' }, 'Loading the recording...');
  const time = h('span', { class: 'num replay-time' }, '0:00 / 0:00');
  const scrub = h('input', { type: 'range', class: 'replay-scrub', min: '0', max: '1', step: '100', value: '0', 'aria-label': 'Position' });
  const marks = h('div', { class: 'replay-marks', 'aria-hidden': 'true' });
  // What's buffered: has frames to preview or jump to straight away.
  const bufferBar = h('div', { class: 'replay-buffer', 'aria-hidden': 'true' });
  const playBtn = button(null, { icon: 'pause', 'aria-label': 'Pause', title: 'Play / pause (space)', disabled: true });
  const backBtn = button('10', { icon: 'back', small: true, 'aria-label': 'Back 10 seconds', title: 'Back 10 seconds' });
  const forwardBtn = button('10', { icon: 'forward', small: true, 'aria-label': 'Forward 10 seconds', title: 'Forward 10 seconds' });
  const lookBtn = button('Free look', { icon: 'eye', small: true, 'aria-pressed': 'false', title: 'Pause and use the client yourself: your mouse and arrow keys work its menus, tabs and camera. Nothing reaches the game, and turning it off puts the client back as it was.' });
  const footHint = h('span', { class: 'muted' }, 'Watching is recorded in the audit log.');
  const speeds = h('div', { class: 'segmented replay-speeds', role: 'group', 'aria-label': 'Speed' });
  const timeline = h('div', { class: 'list replay-notes' });
  const checks = h('div', {});
  const seekText = h('span', {});
  const seekBar = h('span', { class: 'replay-seek-bar' });
  const cover = h('div', { class: 'replay-cover hidden' }, h('div', {}, h('div', { class: 'spinner' }), seekText, h('span', { class: 'replay-seek-track' }, seekBar)));
  const sizes = h('div', { class: 'segmented replay-sizes', role: 'group', 'aria-label': 'Size' });

  const send = (msg, transfer) => { if (dock.page === pageCtl) dockPost('main', msg, transfer); };

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
    on: { click: () => { wantSpeed = s; send({ type: 'speed', value: s }); drawSpeeds(); } },
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
    send({ type: wantPaused ? 'pause' : 'play' });
    drawPlay();
    // Playing ends free look (the page puts the client back).
    if (!wantPaused && freeLook) {
      freeLook = false;
      drawLook();
    }
  };

  const seek = (ms) => {
    seekingTo = ms;
    seekSentAt = performance.now();
    seekHeard = false;
    // The slider goes straight there, not through where the replay passes
    // on its way (a seek back starts it from earlier).
    if (!dragging) scrub.value = String(Math.round(ms));
    time.textContent = `${clock(ms / 1000)} / ${clock(Number(scrub.max) / 1000)}`;
    // The page answers at once with how it's seeking (over a watched frame
    // or not), which sets the cover.
    send({ type: 'seek', ms });
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
    if (dock.page === pageCtl) dock.box = { left: (room - width) / 2, width, height };
    stage.style.height = `${height}px`;
  };
  const drawSizes = () => replace(sizes, [['sharp', 'Sharp'], ['fit', 'Fit']].map(([key, label]) => h('button', {
    type: 'button', 'aria-pressed': String(sizing === key), title: key === 'sharp' ? 'Whole screen pixels per game pixel' : 'Fill the width, smoothed',
    on: { click: () => { sizing = key; prefs.set('replaySize', key); drawSizes(); fit(); } },
  }, label)));
  drawSizes();
  const onDpr = () => fit();
  window.addEventListener('resize', onDpr);
  ctx.cleanup(() => window.removeEventListener('resize', onDpr));
  const resize = new ResizeObserver(fit);
  resize.observe(stage);
  ctx.cleanup(() => resize.disconnect());

  // The dock's client, laid over the stage. `seekTo`: where to start.
  const pageCtl = { stage, onMessage: (which, msg) => (which === 'main' ? onMain(msg) : onScout(msg)) };
  const mount = (seekTo) => {
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
    if (dock.ready.main) onMain({ type: 'ready', width: dock.size?.[0], height: dock.size?.[1] });
    if (scout && dock.ready.scout) onScout({ type: 'ready' });
    fit();
  };
  ctx.cleanup(() => {
    dockDetach(pageCtl);
    cover.remove();
  });

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
    send({ type: 'load', file: copy, seek: loadSeek, speed: wantSpeed, paused: wantPaused, freeLook }, [copy]);
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
      if (msg.live && !scout) requestShot(1000);
      // Hovering somewhere still waiting for a frame: ask again, in case
      // the first ask came before the page (or the scout) could answer.
      const waiting = hoverMs !== null && !(previewFrame !== null && Math.abs(previewFrame - hoverMs) <= previewWithin());
      if (waiting && performance.now() - previewAsked > 400) {
        previewAsked = performance.now();
        send({ type: 'preview', ms: hoverMs });
      }
      const total = msg.durationMs || detail.replay.duration_ms;
      atMs = msg.ms;
      if (msg.seeking) seekHeard = true;
      else if (seekingTo !== null && (seekHeard || performance.now() - seekSentAt > 1000)) seekingTo = null;
      // Seeking: shown at where it's going.
      const shownMs = seekingTo ?? (msg.seeking ? msg.seekTo : msg.ms);
      if (Array.isArray(msg.buffered) && total > 0) {
        replace(bufferBar, msg.buffered.map(([from, to]) => h('span', { style: {
          left: `${Math.min(100, (from / total) * 100)}%`,
          width: `${Math.max(0.2, Math.min(100, ((Math.min(to, total) - from) / total) * 100))}%`,
        } })));
      }
      scrub.max = String(total);
      if (!dragging) scrub.value = String(Math.round(shownMs));
      time.textContent = `${clock(shownMs / 1000)} / ${clock(total / 1000)}`;
      playBtn.disabled = msg.loading;
      if (msg.seeking) {
        showSeek(msg.seekTo, msg.ms, msg.preview);
      } else if (!msg.loading || !loadSeek) {
        showSeek(null);
      }
      if (status.dataset.ended !== '1') {
        status.textContent = msg.seeking ? '' : msg.loading ? 'Starting the client...' : msg.live ? '' : 'Reconnecting...';
      }
      for (const el of timeline.querySelectorAll('[data-ms]')) el.classList.toggle('past', Number(el.dataset.ms) <= shownMs);
    } else if (msg.type === 'ended') {
      wantPaused = true;
      drawPlay();
      status.dataset.ended = '1';
      status.textContent = 'End of the recording.';
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
    if (e.code !== 'Space' || e.target.closest('input, textarea, button, select') || playBtn.disabled) return;
    e.preventDefault();
    togglePlay();
  };
  document.addEventListener('keydown', onKey);
  ctx.cleanup(() => document.removeEventListener('keydown', onKey));

  playBtn.addEventListener('click', togglePlay);
  backBtn.addEventListener('click', () => skip(-10_000));
  forwardBtn.addEventListener('click', () => skip(10_000));
  lookBtn.addEventListener('click', () => {
    freeLook = !freeLook;
    send({ type: 'freeLook', on: freeLook });
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
  const previewImg = h('img', { alt: '', class: 'hidden' });
  const previewTime = h('span', { class: 'num' });
  const previewWaitText = h('span', {}, 'Loading previews...');
  const previewSpin = h('div', { class: 'replay-preview-wait hidden' }, h('div', { class: 'spinner' }), previewWaitText);
  const preview = h('div', { class: 'replay-preview hidden', 'aria-hidden': 'true' }, previewImg, previewSpin, previewTime);
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
  ctx.cleanup(() => { if (previewUrl) URL.revokeObjectURL(previewUrl); });

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
    if (!detail?.player_url || !file) return;
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
      if (dock.ready.scout) onScout({ type: 'ready' });
    } else if (scoutLoaded) {
      dockPost('scout', { type: 'shoot', ms: at });
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
        dockPost('scout', { type: 'load', file: copy, seek: scoutWant || 1, speed: 1, paused: true }, [copy]);
      }
    } else if (msg.type === 'shot' && msg.blob) {
      // Its first frame: it has loaded.
      previewWaitText.remove();
      send({ type: 'addShot', ms: msg.ms, blob: msg.blob, step: msg.step });
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
      send({ type: 'preview', ms: hoverMs });
    } else {
      askTimer = setTimeout(() => {
        if (hoverMs === null) return;
        previewAsked = performance.now();
        send({ type: 'preview', ms: hoverMs });
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
      replace(stage, h('div', { class: 'empty' }, e.message));
      status.textContent = '';
      return;
    }
    const r = detail.replay;
    const refs = [{ type: 'replay', id: r.id }];
    const caseTitle = r.flags.length ? `Flagged session: ${FLAGS[r.flags[0]]?.[0] || r.flags[0]}` : '';
    replace(head, pageHead(
      h('span', {}, playerLink(r.player), '\'s session', ' ', caseLinks(r.cases)),
      `${r.world ? `World ${r.world}` : `Node ${r.node}`} · ${fullTime(r.started_ms)} · ${clock(r.duration_ms / 1000)}${r.live ? ' so far (still playing)' : ''} · revision ${r.revision}`,
      link(`/replays?${query({ name: r.player })}`, { class: 'btn' }, 'Their sessions'),
      button('Add to case', { icon: 'plus', on: { click: () => addToCaseDialog({ refs, player: r.player, what: ['this session'], title: caseTitle }) } }),
      button('Open case', { icon: 'briefcase', on: { click: () => newCaseDialog({ player: r.player, refs, what: ['this session'], title: caseTitle, priority: r.score >= 100 ? 'high' : 'normal' }) } })));

    replace(checks,
      r.score > 0
        ? h('div', { class: 'callout danger' }, icon('alert'), h('div', {},
          h('b', {}, `The checks scored this session ${r.score}.`),
          h('ul', { class: 'flag-list' }, r.flags.map((f) => h('li', {}, h('b', {}, FLAGS[f]?.[0] || f), ': ', FLAGS[f]?.[1] || '')))))
        : h('div', { class: 'callout' }, icon('check'), h('div', {}, 'Nothing suspicious found by the checks.')),
      replayStats(r));

    if (!detail.player_url) {
      replace(stage, h('div', { class: 'empty' }, icon('alert'), detail.why));
      status.textContent = '';
      return;
    }
    origin = new URL(detail.player_url).origin;
    // `t`: where to start, as from a report's "Their session".
    mount(Math.min(Math.max(0, Number(ctx.params.get('t')) || 0), r.duration_ms));
    // A finished recording never changes; a live one has grown since.
    const finished = !r.live && r.ended_ms != null;
    if (finished && replayCache.has(id)) {
      file = replayCache.get(id);
      cacheReplay(id, file);
    } else {
      try {
        const res = await fetch(`/api/replays/${id}/file?${query({ revision: r.revision, player: r.player })}`, { credentials: 'same-origin' });
        if (!res.ok) {
          let message = `Request failed (${res.status})`;
          try { message = (await res.json()).error || message; } catch { /* not JSON */ }
          throw new Error(message);
        }
        file = await res.arrayBuffer();
      } catch (e) {
        dockDetach(pageCtl);
        cover.remove();
        replace(stage, h('div', { class: 'empty' }, e.message));
        status.textContent = '';
        return;
      }
      if (finished) cacheReplay(id, file);
    }

    // The timeline: what they did, from the recording.
    const view = new DataView(file);
    const header = JSON.parse(new TextDecoder().decode(new Uint8Array(file, 9, view.getUint32(5))));
    const total = header.durationMs || r.duration_ms || 1;
    replace(marks, header.notes.map(([ms]) => h('span', { style: { left: `${Math.min(100, (ms / total) * 100)}%` } })));
    replace(timeline, header.notes.length
      ? header.notes.map(([ms, text]) => h('button', { type: 'button', class: 'list-item replay-note', dataset: { ms }, on: { click: () => seek(ms) } },
        h('span', { class: 'num muted' }, clock(ms / 1000)), h('span', {}, text)))
      : h('div', { class: 'empty' }, 'Nothing noted.'));
    sendLoad();
    if (scoutReady && !scoutLoaded) onScout({ type: 'ready' });
  };
  load();

  append(page,
    h('div', { class: 'replay-layout' },
      h('section', { class: 'card replay-card' },
        stage,
        h('div', { class: 'replay-controls' },
          backBtn,
          playBtn,
          forwardBtn,
          h('div', { class: 'replay-track' }, bufferBar, scrub, marks, preview),
          time,
          speeds,
          sizes,
          lookBtn),
        h('div', { class: 'replay-foot' }, status, footHint)),
      h('div', { class: 'replay-side' },
        h('section', { class: 'card' }, h('div', { class: 'card-head' }, h('h2', {}, 'Anti-cheat checks')), h('div', { class: 'card-body' }, checks)),
        h('section', { class: 'card' }, h('div', { class: 'card-head' }, h('h2', {}, 'What they did')), timeline))));
  return page;
}

// ---------------------------------------------------------------------------
// Staff accounts (developers)

const ROLES = [['moderator', 'Moderator'], ['admin', 'Admin'], ['developer', 'Developer']];
const ROLE_HINTS = {
  moderator: 'Kicks, mutes and bans, looks players up and reads chat logs.',
  admin: 'Also reboots, broadcasts and toggles maintenance.',
  developer: 'Everything, including managing these accounts.',
};

// The acting developer's password and code, asked for with every change.
function confirmFields() {
  const password = h('input', { class: 'input', type: 'password', autocomplete: 'current-password', required: true, 'aria-label': 'Your password' });
  const code = h('input', { class: 'input', inputmode: 'numeric', autocomplete: 'one-time-code', maxlength: '6', pattern: '[0-9]{6}', required: true, placeholder: '000000', 'aria-label': 'Your authenticator code', 'data-invalid': 'Authenticator codes are 6 digits.' });
  code.addEventListener('input', () => { code.value = code.value.replace(/\D/g, '').slice(0, 6); });
  return {
    el: h('div', { class: 'confirm-box' },
      h('p', { class: 'hint' }, 'Confirm it\'s you'),
      h('div', { class: 'confirm-grid' },
        h('label', { class: 'field' }, h('span', {}, 'Your password'), password),
        h('label', { class: 'field' }, h('span', {}, 'Authenticator code'), code))),
    value: () => ({ password: password.value, code: code.value }),
  };
}

function roleChips(initial, onChange) {
  let role = initial;
  const chips = h('div', { class: 'segmented' });
  const hint = h('p', { class: 'hint' });
  const draw = () => {
    replace(chips, ROLES.map(([key, label]) => h('button', { type: 'button', 'aria-pressed': String(role === key), on: { click: () => { role = key; draw(); onChange?.(role); } } }, label)));
    hint.textContent = ROLE_HINTS[role];
  };
  draw();
  return { el: [chips, hint], value: () => role };
}

function addStaffDialog(onDone) {
  const name = h('input', { class: 'input', maxlength: '12', pattern: NAME_PATTERN, 'data-invalid': NAME_RULE, required: true, autocomplete: 'off', spellcheck: 'false', autofocus: true, placeholder: 'Their in-game name' });
  const password = h('input', { class: 'input', type: 'password', autocomplete: 'new-password', minlength: '12', required: true, 'data-invalid': 'Passwords are at least 12 characters.', placeholder: 'At least 12 characters' });
  const role = roleChips('moderator');
  const confirm = confirmFields();
  openDialog({
    title: 'Add a staff account',
    subtitle: 'They set up their authenticator app at their first sign-in.',
    body: [
      h('label', { class: 'field' }, h('span', {}, 'Name'), name, h('span', { class: 'hint' }, 'Their in-game name. They must already be staff (level 1+) in the game.')),
      h('div', { class: 'field' }, h('span', {}, 'Role'), role.el),
      h('label', { class: 'field' }, h('span', {}, 'First password'), password, h('span', { class: 'hint' }, 'Give it to them privately. They can change it under Account.')),
      confirm.el,
    ],
    actions: [{ label: 'Cancel' }, {
      label: 'Add account',
      kind: 'primary',
      icon: 'plus',
      run: async () => {
        await api('POST', '/staff', { name: name.value.trim(), role: role.value(), password: password.value, confirm: confirm.value() });
        toast(`Added ${name.value.trim()}.`);
        onDone();
      },
    }],
  });
}

function staffChangeDialog(account, kind, onDone) {
  const confirm = confirmFields();
  const role = roleChips(account.role);
  const password = h('input', { class: 'input', type: 'password', autocomplete: 'new-password', minlength: '12', required: true, 'data-invalid': 'Passwords are at least 12 characters.', placeholder: 'At least 12 characters' });
  const specs = {
    role: ['Change role', 'primary', 'key', [h('div', { class: 'field' }, h('span', {}, 'Role'), role.el)], () => ({ kind: 'role', role: role.value() })],
    password: ['Set password', 'primary', 'lock', [h('label', { class: 'field' }, h('span', {}, 'New password'), password, h('span', { class: 'hint' }, 'Give it to them privately.'))], () => ({ kind: 'password', password: password.value })],
    reset_2fa: ['Reset 2FA', 'primary', 'shield', [h('p', { class: 'muted' }, 'They set up a new authenticator app at their next sign-in. Use this when they lose their phone.')], () => ({ kind: 'reset2fa' })],
    disable: ['Disable', 'danger', 'lock', [h('p', { class: 'muted' }, 'They can\'t sign in until a developer enables the account again.')], () => ({ kind: 'disable' })],
    enable: ['Enable', 'primary', 'unlock', [h('p', { class: 'muted' }, 'They can sign in again.')], () => ({ kind: 'enable' })],
    remove: ['Remove account', 'danger', 'x', [h('div', { class: 'callout danger' }, icon('alert'), h('div', {}, 'The account is deleted. Its audit log entries stay.'))], () => ({ kind: 'remove' })],
  };
  const [label, kindClass, iconName, body, change] = specs[kind];
  openDialog({
    title: `${label}: ${account.name}`,
    subtitle: 'Signs them out everywhere.',
    body: [...body, confirm.el],
    actions: [{ label: 'Cancel' }, {
      label,
      kind: kindClass,
      icon: iconName,
      run: async () => {
        await api('POST', '/staff/update', { name: account.name, change: change(), confirm: confirm.value() });
        toast(`${label}: done for ${account.name}.`);
        onDone();
      },
    }],
  });
}

function staffPage() {
  const list = h('div', { class: 'list' }, h('div', { class: 'empty' }, h('div', { class: 'spinner' })));
  const load = async () => {
    try {
      const accounts = await api('GET', '/staff');
      replace(list, accounts.map((a) => {
        const change = (kind) => () => staffChangeDialog(a, kind, load);
        return h('div', { class: 'list-item' },
          h('div', { class: 'avatar' }, initial(a.name)),
          h('div', { class: 'list-main' },
            h('div', { class: 'list-title' }, h('b', {}, a.name), ' ',
              h('span', { class: 'badge accent' }, a.role), ' ',
              a.you && h('span', { class: 'badge ok' }, 'You'), ' ',
              !a.two_factor && h('span', { class: 'badge warn' }, '2FA not set up'), ' ',
              a.disabled && h('span', { class: 'badge danger' }, 'Disabled')),
            h('div', { class: 'list-sub' }, `${a.sessions ? plural(a.sessions, 'session') + ' open' : 'Signed out'} · added ${a.created_ms ? new Date(a.created_ms).toLocaleDateString() : 'from the command line'}`)),
          a.you
            ? link('/account', { class: 'btn small ghost' }, 'Your account', icon('chevron'))
            : h('div', { class: 'chips' },
              button('Role', { small: true, icon: 'key', on: { click: change('role') } }),
              button('Password', { small: true, icon: 'lock', on: { click: change('password') } }),
              a.two_factor && button('Reset 2FA', { small: true, icon: 'shield', on: { click: change('reset_2fa') } }),
              a.disabled
                ? button('Enable', { small: true, icon: 'unlock', on: { click: change('enable') } })
                : button('Disable', { small: true, icon: 'lock', on: { click: change('disable') } }),
              button(null, { small: true, icon: 'x', 'aria-label': `Remove ${a.name}`, title: 'Remove', on: { click: change('remove') } })));
      }));
    } catch (e) {
      replace(list, h('div', { class: 'empty' }, e.message));
    }
  };
  load();
  return h('div', { class: 'page' },
    pageHead('Staff', 'Console accounts. Every change needs your password and authenticator code, and is audited.',
      button('Add account', { icon: 'plus', kind: 'primary', on: { click: () => addStaffDialog(load) } })),
    h('section', { class: 'card' }, list));
}

// ---------------------------------------------------------------------------
// Command palette (Ctrl/Cmd+K)

function paletteItems(query) {
  const admin = state.canCommand;
  const { online, rebooting } = totals();
  const items = [
    ...navItems().map((n) => ({ group: 'Go to', label: n.label, icon: n.icon, run: () => navigate(n.path) })),
    admin && { group: 'Actions', label: 'Broadcast to all worlds...', icon: 'megaphone', run: () => broadcastDialog(online) },
    admin && { group: 'Actions', label: 'Reboot all worlds...', icon: 'power', run: () => online.length && rebootDialog(online) },
    admin && rebooting.length && { group: 'Actions', label: 'Cancel every scheduled reboot...', icon: 'undo', run: () => cancelRebootDialog(rebooting) },
    { group: 'Actions', label: 'Kick a player...', icon: 'kick', run: () => moderateDialog('', 'kick') },
    { group: 'Actions', label: 'Mute a player...', icon: 'mute', run: () => moderateDialog('', 'mute') },
    { group: 'Actions', label: 'Ban a player...', icon: 'ban', run: () => moderateDialog('', 'ban') },
    { group: 'Actions', label: 'Open a case...', icon: 'briefcase', run: () => newCaseDialog({}) },
    { group: 'Actions', label: 'Triage reports and flagged sessions', icon: 'inbox', run: () => navigate('/moderation?tab=triage') },
    ...state.worlds.map((w) => ({
      group: 'Worlds',
      label: `${worldName(w)}${w.region ? ` · ${w.region}` : ''}`,
      hint: w.online ? `${w.status.players} players` : 'offline',
      icon: 'globe',
      run: () => (w.online ? worldPlayersDialog(w) : navigate('/worlds')),
    })),
  ].filter(Boolean);
  const q = query.trim().toLowerCase();
  const matches = q ? items.filter((i) => i.label.toLowerCase().includes(q)) : items.filter((i) => i.group !== 'Worlds');
  const caseId = /^#?(\d{1,9})$/.exec(query.trim());
  if (caseId) matches.unshift({ group: 'Cases', label: `Go to case #${caseId[1]}`, icon: 'briefcase', run: () => navigate(caseHref(caseId[1])) });
  if (q && /^[a-z0-9 _]{1,12}$/i.test(query.trim())) {
    matches.push({ group: 'Players', label: `Look up ${query.trim()}`, icon: 'user', run: () => navigate(`/player?name=${encodeURIComponent(query.trim())}`) });
    matches.push({ group: 'Players', label: `Find "${query.trim()}" online`, icon: 'search', run: () => navigate(`/players?q=${encodeURIComponent(query.trim())}`) });
    matches.push({ group: 'Players', label: `Chat logs of ${query.trim()}`, icon: 'chat', run: () => navigate(`/chat?name=${encodeURIComponent(query.trim())}`) });
    matches.push({ group: 'Players', label: `Cases on ${query.trim()}`, icon: 'briefcase', run: () => navigate(`/moderation?tab=cases&show=all&q=${encodeURIComponent(query.trim())}`) });
  }
  return matches.slice(0, 40);
}

function openPalette() {
  if (state.session?.stage !== 'signed_in' || document.querySelector('dialog.palette')) return;
  const input = h('input', { placeholder: 'Search pages, actions, worlds, players', 'aria-label': 'Command', autocomplete: 'off', spellcheck: 'false' });
  const list = h('div', { class: 'palette-list', role: 'listbox' });
  const dialog = h('dialog', { class: 'palette', 'aria-label': 'Command palette' },
    h('div', { class: 'palette-input' }, icon('search'), input, h('kbd', {}, 'Esc')),
    list,
    h('div', { class: 'palette-foot hide-mobile' }, h('span', {}, h('kbd', {}, '↑'), ' ', h('kbd', {}, '↓'), ' to move'), h('span', {}, h('kbd', {}, 'Enter'), ' to run')));
  let items = [];
  let active = 0;
  const run = (item) => {
    dialog.close();
    item.run();
  };
  const draw = () => {
    items = paletteItems(input.value);
    active = Math.min(active, Math.max(0, items.length - 1));
    list.replaceChildren();
    let group = null;
    items.forEach((item, i) => {
      if (item.group !== group) {
        group = item.group;
        list.append(h('div', { class: 'palette-group' }, group));
      }
      list.append(h('button', {
        type: 'button', class: 'palette-item', role: 'option', 'aria-selected': String(i === active),
        on: { click: () => run(item), mousemove: () => { if (active !== i) { active = i; draw(); } } },
      }, icon(item.icon), h('span', {}, item.label), item.hint && h('small', {}, item.hint)));
    });
    if (!items.length) list.append(h('div', { class: 'empty' }, 'Nothing matches.'));
    list.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' });
  };
  input.addEventListener('input', () => { active = 0; draw(); });
  input.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown') { e.preventDefault(); active = (active + 1) % Math.max(1, items.length); draw(); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); active = (active - 1 + items.length) % Math.max(1, items.length); draw(); }
    else if (e.key === 'Enter' && items[active]) { e.preventDefault(); run(items[active]); }
  });
  dialog.addEventListener('close', () => dialog.remove());
  dialog.addEventListener('click', (e) => { if (e.target === dialog) dialog.close(); });
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
    state.session = { stage: 'signed_out' };
    toast(e.message, 'error');
  }
  render();
})();
