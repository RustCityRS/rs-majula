// Session replay for the staff console. Loaded before this world's own
// client.js on /replay, it makes the client play a recording instead of a
// live game:
//
// - The game's WebSocket is faked. The login handshake is answered here, and
//   the recorded packets are fed in at the times they were sent, with their
//   opcodes ciphered as the world did. Math.random is pinned while the client
//   picks its ISAAC seeds, so the seeds are known.
// - Time is virtual: performance.now, Date.now and the timers run at the
//   chosen speed, or stop while paused, which the client's game loop follows.
// - The player's recorded mouse is drawn over the game and replayed into the
//   client, so its menus, tabs and clicks show as they did. The viewer's own
//   input never reaches the client, except in free look, which pauses: then
//   the viewer's mouse and arrow keys work the client (menus, tabs, the
//   camera), and what the client sends is dropped as ever. Ending it puts
//   the client back as it was.
//
// The console frames this page and talks to it with postMessage: it sends
// {type: 'load', file}, then 'play', 'pause', {type: 'speed', value},
// {type: 'seek', ms} and {type: 'freeLook', on}; the page answers with
// 'ready', 'state' and 'ended'.
//
// Watching live, the file's header has `live` ({token, delayMs, ended}) and
// the console adds what's recorded since with {type: 'append', file}, a file
// of only that. Caught up with what it has, the replay waits there (the
// client runs on) until more comes, and only ends once the session has.
(() => {
    'use strict';

    // 225's login handshake starts with the server's seed; later ones with
    // the client's hello.
    const SEED_FIRST = document.documentElement.dataset.rev === '225';

    // A scout (`?scout=1`) is a second, hidden copy the console keeps for
    // previews of what hasn't been watched: asked for a moment, it goes there
    // paused and sends back a frame. It makes no sound.
    const SCOUT = new URLSearchParams(location.search).has('scout');
    if (SCOUT && window.AudioContext) {
        const Real = window.AudioContext;
        window.AudioContext = class extends Real {
            constructor(...args) {
                super(...args);
                this.suspend();
            }

            resume() {
                return Promise.resolve();
            }
        };
        window.webkitAudioContext = window.AudioContext;
    }

    // Music. Every revision's client plays it through its audio library's
    // window._tinyMidiPlay, which renders the whole song to sound as soon as
    // it starts: in 30ms steps, each appending to a growing buffer, a heavy
    // load for many seconds that a replay would run again on its fast clock.
    // A replay is for watching what a player did: it plays no music. (Set
    // here, before the client's scripts define it; sound effects are apart
    // and still play.)
    Object.defineProperty(window, '_tinyMidiPlay', {
        configurable: true,
        get: () => () => Promise.resolve(),
        set: () => {},
    });

    const realNow = performance.now.bind(performance);
    const realSetTimeout = window.setTimeout.bind(window);
    const realClearTimeout = window.clearTimeout.bind(window);
    const realRandom = Math.random;
    const dateOffset = Date.now() - realNow();

    // ---- Virtual time ----

    let speed = 1;
    let paused = false;
    // While seeking, time is driven by the client instead of the wall: it
    // moves on a step each time the game loop sleeps, and stands still while
    // the client is busy (loading a map). The client runs as fast as it can
    // and never falls behind or times out.
    let driven = false;
    // One game tick (20ms) a loop turn: the client works out how many ticks
    // to run a turn from how long its last ten took, so steps of any other
    // length (or a change of length) leave it running too few, and what it
    // simulates (walking, the camera following) falls behind the recording,
    // for good. Its drawing is skipped on the way (see gateDraws), so a turn
    // is cheap.
    let DRIVE_STEP = 20;
    let vBase = realNow();
    let rBase = vBase;
    const vnow = () => (paused || driven ? vBase : vBase + (realNow() - rBase) * speed);

    function setClock(nextSpeed, nextPaused) {
        vBase = vnow();
        rBase = realNow();
        driven = false;
        speed = nextSpeed;
        paused = nextPaused;
        schedule();
    }

    function drive() {
        vBase = vnow();
        driven = true;
        paused = false;
        schedule();
    }

    // The client's timers, by id, and in a heap by when they're due (then
    // id). Some clients leave many waiting (225's socket reader starts a
    // 20-second timeout for every byte it reads and never clears them), so
    // nothing here looks at them all.
    const timers = new Map();
    const heap = [];
    let nextTimer = 1;
    let driver = null;
    // How many short one-off timers wait: the game loop asleep (see addTimer).
    let loopTimers = 0;
    // Recent short timeouts: the client's game loop sleeps ~20ms a turn.
    const loopBeats = [];

    const before = (a, b) => a.due < b.due || (a.due === b.due && a.id < b.id);
    function heapPush(t) {
        heap.push(t);
        let i = heap.length - 1;
        while (i > 0) {
            const up = (i - 1) >> 1;
            if (!before(heap[i], heap[up])) {
                break;
            }
            [heap[i], heap[up]] = [heap[up], heap[i]];
            i = up;
        }
    }
    function heapPop() {
        const top = heap[0];
        const last = heap.pop();
        if (heap.length) {
            heap[0] = last;
            let i = 0;
            for (;;) {
                const l = 2 * i + 1;
                const r = l + 1;
                let m = i;
                if (l < heap.length && before(heap[l], heap[m])) {
                    m = l;
                }
                if (r < heap.length && before(heap[r], heap[m])) {
                    m = r;
                }
                if (m === i) {
                    break;
                }
                [heap[i], heap[m]] = [heap[m], heap[i]];
                i = m;
            }
        }
        return top;
    }
    // Cleared or fired: gone from the map; the heap drops it when it surfaces.
    function dropTimer(t) {
        if (!t.dead) {
            t.dead = true;
            timers.delete(t.id);
            if (t.loop) {
                loopTimers--;
            }
        }
    }

    // The game loop is asleep: a short one-off timer is waiting.
    const loopWaiting = () => loopTimers > 0;

    function addTimer(fn, delay, args, repeat) {
        const id = nextTimer++;
        const wait = Math.max(0, Number(delay) || 0);
        const loop = !repeat && wait > 0 && wait <= 30;
        const timer = { id, fn, args, due: vnow() + wait, every: repeat ? Math.max(1, wait) : 0, loop, dead: false };
        timers.set(id, timer);
        heapPush(timer);
        if (loop) {
            loopTimers++;
        }
        if (repeat && wait === 50 && /\bthis\./.test(Function.prototype.toString.call(fn))) {
            // 254+ start sampling the mouse every 50ms once loading is done
            // (a method of the client's). Not the music player's 50ms fade,
            // which starts with the title music, early in loading.
            clientReady();
        } else if (repeat && wait === 35) {
            // The title screen's flames: it has finished setting up. They
            // only animate where they can be seen: not in the scout, nor
            // fast-forwarding (behind the seek cover, many steps a moment).
            replay.flamesAt = realNow();
            const flames = fn;
            timers.get(id).fn = (...a) => {
                if (!SCOUT && !driven) {
                    flames(...a);
                }
            };
        } else if (loop && !driven) {
            // The game loop sleeping a turn, 40 times a second: it's running.
            const now = realNow();
            loopBeats.push(now);
            while (loopBeats.length && now - loopBeats[0] > 1000) {
                loopBeats.shift();
            }
            if (loopBeats.length >= 40) {
                clientReady();
            }
        }
        schedule();
        return id;
    }

    // Zero-delay turns of the event loop, without the browser's 4ms floor.
    // Fast-forwarding, a turn is one game tick, and many run in one task (as
    // microtasks: the client asks for its next loop sleep at the end of a
    // tick) until the slice is used, then the page yields. A task each would
    // crawl: the browser takes turns between this page's tasks and the
    // other replay's. The scout's slices are short, so the replay being
    // watched stays smooth; the replay's own, behind its seek cover, long.
    const turns = new MessageChannel();
    let turnPending = false;
    let sliceStart = 0;
    const sliceMs = () => (SCOUT ? 6 : 40);
    turns.port1.onmessage = () => {
        turnPending = false;
        sliceStart = realNow();
        runTimers();
    };
    function requestTurn() {
        if (turnPending) {
            return;
        }
        turnPending = true;
        if (realNow() - sliceStart < sliceMs()) {
            queueMicrotask(() => {
                turnPending = false;
                runTimers();
            });
        } else {
            turns.port2.postMessage(0);
        }
    }

    function schedule() {
        if (driver !== null) {
            realClearTimeout(driver);
            driver = null;
        }
        if (paused || timers.size === 0) {
            return;
        }
        if (driven) {
            if (loopWaiting() || earliest() <= vBase) {
                requestTurn();
            } else {
                // The client is busy: look again shortly.
                driver = realSetTimeout(runTimers, 4);
            }
            return;
        }
        driver = realSetTimeout(runTimers, Math.max(0, (earliest() - vnow()) / speed));
    }

    function earliest() {
        while (heap.length && heap[0].dead) {
            heapPop();
        }
        return heap.length ? heap[0].due : Infinity;
    }

    function runTimers() {
        driver = null;
        if (driven && loopWaiting()) {
            // A step on, firing what falls inside it in order.
            const target = vBase + DRIVE_STEP;
            while (driven && earliest() <= target) {
                vBase = Math.max(vBase, earliest());
                fire(vBase);
            }
            if (driven) {
                vBase = target;
            }
        }
        fire(vnow());
        schedule();
    }

    // Fires what's due by `now`, in order. Not what those add: it's due
    // later, or in the next call.
    function fire(now) {
        const ready = [];
        while (heap.length && (heap[0].dead || heap[0].due <= now)) {
            const t = heapPop();
            if (!t.dead) {
                ready.push(t);
            }
        }
        for (const t of ready) {
            if (t.dead) {
                continue;
            }
            if (t.every) {
                t.due = Math.max(t.due + t.every, now + 1);
                heapPush(t);
            } else {
                dropTimer(t);
            }
            try {
                t.fn(...(t.args || []));
            } catch (e) {
                console.error(e);
            }
        }
    }

    const clearTimer = (id) => {
        const t = timers.get(id);
        if (t) {
            dropTimer(t);
        }
    };
    window.setTimeout = (fn, delay, ...args) => addTimer(fn, delay, args, false);
    window.setInterval = (fn, delay, ...args) => addTimer(fn, delay, args, true);
    window.clearTimeout = clearTimer;
    window.clearInterval = clearTimer;
    performance.now = () => vnow();
    Date.now = () => Math.floor(dateOffset + vnow());

    let pinned = false;
    Math.random = () => (pinned ? 0.5 : realRandom());

    // ---- ISAAC, as the client and world use it ----

    class Isaac {
        constructor(seed) {
            this.count = 0;
            this.rsl = new Int32Array(256);
            this.mem = new Int32Array(256);
            this.a = 0;
            this.b = 0;
            this.c = 0;
            this.rsl.set(seed);
            this.init();
        }

        next() {
            if (this.count-- === 0) {
                this.isaac();
                this.count = 255;
            }
            return this.rsl[this.count];
        }

        init() {
            const s = new Int32Array(8).fill(0x9e3779b9 | 0);
            const mix = () => {
                s[0] ^= s[1] << 11; s[3] += s[0]; s[1] += s[2];
                s[1] ^= s[2] >>> 2; s[4] += s[1]; s[2] += s[3];
                s[2] ^= s[3] << 8; s[5] += s[2]; s[3] += s[4];
                s[3] ^= s[4] >>> 16; s[6] += s[3]; s[4] += s[5];
                s[4] ^= s[5] << 10; s[7] += s[4]; s[5] += s[6];
                s[5] ^= s[6] >>> 4; s[0] += s[5]; s[6] += s[7];
                s[6] ^= s[7] << 8; s[1] += s[6]; s[7] += s[0];
                s[7] ^= s[0] >>> 9; s[2] += s[7]; s[0] += s[1];
            };
            for (let i = 0; i < 4; i++) {
                mix();
            }
            for (const from of [this.rsl, this.mem]) {
                for (let i = 0; i < 256; i += 8) {
                    for (let j = 0; j < 8; j++) {
                        s[j] += from[i + j];
                    }
                    mix();
                    this.mem.set(s, i);
                }
            }
            this.isaac();
            this.count = 256;
        }

        isaac() {
            this.c = (this.c + 1) | 0;
            this.b = (this.b + this.c) | 0;
            for (let i = 0; i < 256; i++) {
                const x = this.mem[i];
                switch (i & 3) {
                    case 0: this.a ^= this.a << 13; break;
                    case 1: this.a ^= this.a >>> 6; break;
                    case 2: this.a ^= this.a << 2; break;
                    default: this.a ^= this.a >>> 16;
                }
                this.a = (this.a + this.mem[(i + 128) & 0xff]) | 0;
                const y = (this.mem[(x >>> 2) & 0xff] + this.a + this.b) | 0;
                this.mem[i] = y;
                this.b = (this.mem[((y >>> 8) >>> 2) & 0xff] + x) | 0;
                this.rsl[i] = this.b;
            }
        }
    }

    // The seeds the client picks with Math.random pinned at 0.5 and a server
    // seed of 0; the world's cipher adds 50 to each.
    const SEED = Math.floor(0.5 * 99999999);
    const SERVER_SEED = [SEED + 50, SEED + 50, 50, 50];

    // ---- The recording ----

    const FRAME_SERVER = 0;
    const FRAME_LOGIN = 1;
    const FRAME_RECONNECT = 2;
    // What a client that has just logged in needs to start here; skipped
    // while playing on.
    const FRAME_KEYFRAME = 3;
    // A seek this far past where it is starts from a keyframe rather than
    // playing the way there.
    const KEYFRAME_JUMP_MS = 15_000;

    function parse(buffer) {
        const view = new DataView(buffer);
        const magic = String.fromCharCode(...new Uint8Array(buffer, 0, 4));
        if (magic !== 'RSRP' || view.getUint8(4) !== 1) {
            throw new Error('not a replay file');
        }
        const headerLen = view.getUint32(5);
        const header = JSON.parse(new TextDecoder().decode(new Uint8Array(buffer, 9, headerLen)));
        const frames = [];
        let pos = 9 + headerLen;
        while (pos + 9 <= buffer.byteLength) {
            const ms = view.getUint32(pos);
            const kind = view.getUint8(pos + 4);
            const len = view.getUint32(pos + 5);
            frames.push({ ms, kind, bytes: new Uint8Array(buffer, pos + 9, len) });
            pos += 9 + len;
        }
        return { header, frames };
    }

    const replay = {
        client: null,
        ready: false,
        // Whether this recording's first login has gone through: until then
        // the client is starting, and seeks wait (see 'seek').
        started: false,
        rec: null,
        socket: null,
        cipher: null,
        frame: 0,
        input: 0,
        streaming: false,
        baseMs: 0,
        baseV: 0,
        lastMs: 0,
        userSpeed: 1,
        userPaused: false,
        seekTo: null,
        // While held, the virtual time it was held at: the client runs on but
        // the recording stands still.
        held: null,
        // After a seek that ends paused, held until the client has drawn the
        // spot: {real, to, built}.
        settling: null,
        freeLook: false,
        ended: false,
        loggingIn: false,
        // When the client last closed the game connection itself.
        closedAt: null,
        cursor: null,
        trail: [],
        clicks: [],
        focus: true,
        outside: false,
        keys: [],
        buttons: 0,
        // Camera reports ([ms, pitch, yaw]), the last one passed, and the
        // client's camera field names once found.
        cameras: [],
        camera: -1,
        // The minimap's tilt and zoom as the client sent them ([ms, angle,
        // zoom, yaw or null]), the last one passed, and the client's field
        // names once found.
        minimaps: [],
        minimap: -1,
        minimapFields: undefined,
        // When the title screen last started its flames.
        flamesAt: null,
        // How far apart the scout's sweep frames are (see frameNear).
        scoutStep: 0,
        // The logout packet's opcode, kept past its recording (see unload).
        logoutOp: null,
        // Counts restarts (see restart).
        restartGen: 0,
        // The client's draws: whether they're gated (see gateDraws), how many
        // there have been, and what to do after the next.
        // What happened, for debugging: [wall ms, what].
        log: [],
        drawGated: false,
        drawHooked: false,
        draws: 0,
        // The draw count when the last seek arrived.
        seekDraws: 0,
        afterDraw: null,
        // The keyframe (frame index) the next login starts from, if any.
        startAt: null,
        arrowsHeld: new Set(),
        cameraFields: undefined,
        // Watching live: {delayMs, ended}, and whether it's caught up and
        // waiting for more.
        live: null,
        starved: false,
    };

    const sessionMs = () => (replay.streaming ? replay.baseMs + (vnow() - replay.baseV) : replay.lastMs);

    function note(what) {
        replay.log.push([Math.round(realNow()), what]);
        if (replay.log.length > 200) {
            replay.log.shift();
        }
    }

    // ---- The fake game connection ----

    class ReplaySocket extends EventTarget {
        constructor(url) {
            super();
            this.url = url;
            this.protocol = 'binary';
            this.extensions = '';
            this.bufferedAmount = 0;
            this.binaryType = 'blob';
            this.readyState = 0;
            // Data that came before the client listened (see deliver).
            this.pending = [];
            this.onopen = this.onmessage = this.onclose = this.onerror = null;
            this.stage = 'hello';
            replay.socket = this;
            note('socket made');
            setTimeout(() => {
                if (this.readyState === 0) {
                    this.readyState = 1;
                    note('socket open');
                    this.fire(new Event('open'));
                }
            }, 0);
            if (SEED_FIRST) {
                // 225 sends the server seed as soon as it connects. Once the
                // client is reading, that is.
                setTimeout(() => sendSeed(this, 8), 1);
            }
        }

        fire(event) {
            const handler = this['on' + event.type];
            if (typeof handler === 'function') {
                handler.call(this, event);
            }
            this.dispatchEvent(event);
        }

        send(data) {
            if (this.readyState !== 1) {
                return;
            }
            const bytes = data instanceof ArrayBuffer ? new Uint8Array(data) : new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
            handshake(this, bytes);
        }

        // To the client, once it listens: 225's seed goes out as the
        // connection opens, and can come before the client has set its
        // handler (it does so once it sees the connection open).
        deliver(bytes) {
            if (this.readyState !== 1) {
                return;
            }
            if (!this._onmessage) {
                this.pending.push(bytes);
                return;
            }
            const data = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
            this.fire(new MessageEvent('message', { data }));
        }

        get onmessage() {
            return this._onmessage ?? null;
        }

        set onmessage(fn) {
            this._onmessage = fn;
            if (fn && this.pending?.length) {
                const pending = this.pending;
                this.pending = [];
                queueMicrotask(() => pending.forEach((bytes) => this.deliver(bytes)));
            }
        }

        close() {
            note(`socket closed by client (${this.stage})`);
            if (this.readyState < 2) {
                this.readyState = 3;
            }
            if (replay.socket === this) {
                replay.socket = null;
                replay.streaming = false;
                replay.closedAt = vnow();
            }
        }

        // The recorded connection dropped: the client finds out and
        // reconnects, as it did.
        drop() {
            if (this.readyState === 1) {
                this.readyState = 3;
                this.fire(new CloseEvent('close', { code: 1006 }));
            }
        }
    }
    for (const [name, value] of Object.entries({ CONNECTING: 0, OPEN: 1, CLOSING: 2, CLOSED: 3 })) {
        ReplaySocket[name] = value;
        ReplaySocket.prototype[name] = value;
    }
    window.WebSocket = ReplaySocket;

    // A server seed of 0, after `len - 8` bytes the client reads first. The
    // client picks its seeds right after reading it.
    function sendSeed(socket, len) {
        if (socket.stage !== 'hello') {
            return;
        }
        socket.stage = 'login';
        pinned = true;
        realSetTimeout(() => (pinned = false), 0);
        socket.deliver(new Uint8Array(len));
    }

    function handshake(socket, bytes) {
        note(`client sent ${bytes.length}B at ${socket.stage}`);
        if (socket.stage === 'hello') {
            // From 244: 8 ignored bytes and "go on" (0) come first.
            sendSeed(socket, 17);
        } else if (socket.stage === 'login') {
            pinned = false;
            socket.stage = 'game';
            replay.cipher = new Isaac(SERVER_SEED);
            const frames = replay.rec.frames;
            const next = frames[replay.frame];
            let reply;
            let keyframe = null;
            if (replay.startAt !== null) {
                // Starting from a keyframe: the session's login, then the
                // keyframe in place of everything before it.
                keyframe = frames[replay.startAt];
                reply = frames.find((f) => f.kind === FRAME_LOGIN).bytes;
                replay.baseMs = keyframe.ms;
                replay.lastMs = keyframe.ms;
                replay.frame = replay.startAt + 1;
                const inputs = replay.rec.header.inputs;
                replay.input = inputs.findIndex((i) => i[0] >= keyframe.ms);
                if (replay.input < 0) {
                    replay.input = inputs.length;
                }
                // A line half typed before the keyframe isn't typed at all.
                const typing = replay.input;
                while (replay.input < inputs.length && inputs[replay.input][1] === 11 && inputs[replay.input][2] !== 'Enter') {
                    replay.input++;
                }
                if (replay.input > typing && inputs[replay.input]?.[2] === 'Enter') {
                    replay.input++;
                }
                replay.startAt = null;
            } else if (next && (next.kind === FRAME_LOGIN || next.kind === FRAME_RECONNECT)) {
                reply = next.bytes;
                replay.baseMs = next.ms;
                replay.frame++;
            } else {
                // A reconnect the recording doesn't have: carry on.
                reply = new Uint8Array([15]);
                replay.baseMs = replay.lastMs;
            }
            replay.baseV = vnow();
            replay.streaming = true;
            replay.started = true;
            replay.loggingIn = false;
            replay.closedAt = null;
            socket.deliver(reply);
            if (keyframe) {
                deliverServer(keyframe);
            }
        }
        // Anything else the client sends during the game is ignored.
    }

    // ---- Playing ----

    function deliverServer(frame) {
        const out = new Uint8Array(frame.bytes.length);
        let read = 0;
        let write = 0;
        const src = frame.bytes;
        while (read + 2 <= src.length) {
            const len = (src[read] << 8) | src[read + 1];
            read += 2;
            out.set(src.subarray(read, read + len), write);
            out[write] = (src[read] + replay.cipher.next()) & 0xff;
            read += len;
            write += len;
        }
        replay.socket?.deliver(out.subarray(0, write));
    }

    function step() {
        if (!replay.rec) {
            return;
        }
        if (replay.held !== null) {
            keepAlive();
            if (replay.settling && settled()) {
                replay.settling = null;
                applyClock();
                if (SCOUT && scouting.current !== null) {
                    scouted();
                }
            }
            return;
        }
        const ms = sessionMs();
        if (replay.streaming) {
            replay.lastMs = ms;
            const frames = replay.rec.frames;
            while (replay.frame < frames.length && frames[replay.frame].ms <= ms) {
                const frame = frames[replay.frame];
                if (frame.kind === FRAME_SERVER) {
                    deliverServer(frame);
                    replay.frame++;
                } else if (frame.kind === FRAME_RECONNECT) {
                    replay.streaming = false;
                    replay.socket?.drop();
                    break;
                } else {
                    replay.frame++;
                }
            }
        }
        playInputs(replay.lastMs);
        playCamera(replay.lastMs);
        playMinimap(replay.lastMs);
        if (replay.seekTo !== null && replay.lastMs >= replay.seekTo) {
            const to = replay.seekTo;
            replay.seekTo = null;
            replay.seekDraws = replay.draws;
            if (SCOUT && scouting.sweeping && scouting.current !== null && sceneBuilt()) {
                // A sweep frame on a map already built: no settling, just
                // one draw (it skips them on the way) and the picture.
                replay.afterDraw = () => {
                    applyClock();
                    scouted();
                };
                hold();
                setClock(1, false);
            } else if (replay.userPaused) {
                // Stopping here: let the client finish loading and draw it
                // first, or it shows a loading screen until played.
                replay.settling = { real: realNow(), to, built: null };
                hold();
                setClock(1, false);
            } else {
                applyClock();
            }
        }
        // Live, and caught up with what's come: wait there for more.
        const duration = replay.rec.header.durationMs;
        if (replay.live && !replay.live.ended && replay.streaming && replay.seekTo === null && replay.lastMs >= duration) {
            replay.starved = true;
            hold();
            return;
        }
        // Over once everything was sent and its time is up, or once the
        // client closed the connection itself (it logged out) with no
        // reconnect to come.
        const sent = replay.frame >= replay.rec.frames.length;
        const loggedOut = !replay.streaming && replay.closedAt !== null && vnow() - replay.closedAt > 5000
            && !(markedFrames().reconnects.at(-1) >= replay.frame);
        if (!replay.ended && !replay.loggingIn && ((sent && replay.lastMs >= duration) || loggedOut)) {
            replay.ended = true;
            replay.userPaused = true;
            applyClock();
            post({ type: 'ended' });
        }
    }

    // ---- Holding and settling ----

    // The longest a seek waits for the client to draw before stopping.
    const SETTLE_MAX_MS = 8000;

    function hold() {
        if (replay.held === null) {
            replay.held = vnow();
        }
    }

    function unhold() {
        if (replay.held !== null) {
            // The time it was held doesn't count towards the recording.
            replay.baseV += vnow() - replay.held;
            replay.held = null;
        }
    }

    // Sets the clock for what the viewer chose. Paused in free look, the
    // client runs on (it answers the viewer) while the recording is held.
    function applyClock() {
        if (replay.seekTo !== null || replay.settling) {
            return;
        }
        if (replay.userPaused && replay.freeLook) {
            hold();
            setClock(1, false);
        } else {
            unhold();
            setClock(replay.userSpeed, replay.userPaused);
        }
    }

    // The client's scene state field (2 once the map is built), found by the
    // code that draws "Loading - please wait." when it isn't.
    function sceneField() {
        if (replay.sceneField !== undefined || !replay.client) {
            return replay.sceneField ?? null;
        }
        replay.sceneField = null;
        const built = /2\s*===\s*this\.([\w$]+)|this\.([\w$]+)\s*===\s*2\b/;
        let best = null;
        for (let proto = Object.getPrototypeOf(replay.client); proto && proto !== Object.prototype; proto = Object.getPrototypeOf(proto)) {
            for (const name of Object.getOwnPropertyNames(proto)) {
                const fn = name === 'constructor' ? null : Object.getOwnPropertyDescriptor(proto, name)?.value;
                if (typeof fn !== 'function') {
                    continue;
                }
                const source = Function.prototype.toString.call(fn);
                const match = source.includes('Loading - please wait.') ? built.exec(source) : null;
                // The shortest such method: the one that only checks it.
                if (match && (!best || source.length < best.length)) {
                    best = { field: match[1] || match[2], length: source.length };
                }
            }
        }
        if (best && typeof replay.client[best.field] === 'number') {
            replay.sceneField = best.field;
        }
        return replay.sceneField;
    }

    // How the client notices the connection has gone quiet, found in its
    // game loop: from 254 and on 225, the time of the last packet, which more
    // than 15 seconds ago drops the connection; 244 and 245.2 count loop turns
    // since one (250 drops it).
    function idleField() {
        if (replay.idleField !== undefined || !replay.client) {
            return replay.idleField ?? null;
        }
        replay.idleField = null;
        const since = /[\w$]+\s*-\s*this\.([\w$]+)\s*>\s*(?:15e3|15000|15_000)\b/;
        // Counted turns, then a reconnect: not the cache's own idle counters.
        const turns = /this\.([\w$]+)\+\+\s*[,;]\s*(?:if\s*\(\s*)?this\.\1\s*>\s*(?:250|750)\s*\)?\s*(?:&&\s*)?(?:\{\s*)?(?:\/\/[^\n]*\n\s*)*await this\./;
        const sources = [];
        for (let proto = Object.getPrototypeOf(replay.client); proto && proto !== Object.prototype; proto = Object.getPrototypeOf(proto)) {
            for (const name of Object.getOwnPropertyNames(proto)) {
                const fn = name === 'constructor' ? null : Object.getOwnPropertyDescriptor(proto, name)?.value;
                if (typeof fn === 'function') {
                    sources.push(Function.prototype.toString.call(fn));
                }
            }
        }
        for (const [pattern, time] of [[since, true], [turns, false]]) {
            for (const source of sources) {
                const match = pattern.exec(source);
                if (match) {
                    replay.idleField = { field: match[1], time };
                    return replay.idleField;
                }
            }
        }
        return null;
    }

    // While held, nothing arrives: the client mustn't decide the connection
    // is lost (it would log out, back to its loading screen).
    function keepAlive() {
        const idle = replay.streaming ? idleField() : null;
        if (idle && typeof replay.client[idle.field] === 'number') {
            replay.client[idle.field] = idle.time ? vnow() : 0;
        }
    }

    // The client has built the map and drawn it a few times.
    function settled() {
        const s = replay.settling;
        const now = realNow();
        if (now - s.real > SETTLE_MAX_MS) {
            return true;
        }
        const field = sceneField();
        const built = field ? replay.client[field] === 2 : now - s.real > 1000;
        if (!built) {
            s.built = null;
            return false;
        }
        s.built ??= now;
        return now - s.built > 150;
    }

    // ---- The player's input ----

    const canvas = () => document.getElementById('canvas');

    function pointer(type, x, y, button) {
        const c = canvas();
        // In free look the viewer has the mouse; the player's is only drawn.
        if (!c || replay.freeLook) {
            return;
        }
        const rect = c.getBoundingClientRect();
        const init = {
            bubbles: true,
            cancelable: true,
            clientX: rect.left + ((x + 0.5) * rect.width) / c.width,
            clientY: rect.top + ((y + 0.5) * rect.height) / c.height,
            button: button ?? 0,
            buttons: replay.buttons,
            pointerType: 'mouse',
            pointerId: 1,
            isPrimary: true,
        };
        const event = type.startsWith('pointer') ? new PointerEvent(type, init) : new MouseEvent(type, init);
        c.dispatchEvent(event);
    }

    function playInputs(ms) {
        const inputs = replay.rec.header.inputs;
        while (replay.input < inputs.length && inputs[replay.input][0] <= ms) {
            const [at, code, a, b, c] = inputs[replay.input++];
            switch (code) {
                case 0: // move
                    if (replay.outside) {
                        replay.outside = false;
                        pointer('pointerenter', a, b);
                    }
                    replay.cursor = [a, b];
                    replay.trail.push([at, a, b]);
                    pointer('pointermove', a, b);
                    break;
                case 1: // left the frame
                    replay.outside = true;
                    pointer('pointerleave', -1, -1);
                    break;
                case 2: // press
                case 4: // click (254+): press, then release
                    replay.cursor = [a, b];
                    replay.clicks.push([at, a, b, c]);
                    replay.buttons = c ? 2 : 1;
                    pointer('mousedown', a, b, c ? 2 : 0);
                    if (code === 4) {
                        setTimeout(() => {
                            replay.buttons = 0;
                            pointer('mouseup', a, b, c ? 2 : 0);
                        }, 60);
                    }
                    break;
                case 3: // release
                    replay.buttons = 0;
                    if (replay.cursor) {
                        pointer('mouseup', replay.cursor[0], replay.cursor[1], a ? 2 : 0);
                    }
                    break;
                case 5: // key down
                    replay.keys.push([at, a]);
                    arrowKey('keydown', a);
                    break;
                case 6: // key up
                    arrowKey('keyup', a);
                    break;
                case 7:
                    replay.focus = a === 1;
                    break;
                case 8:
                    replay.outside = false;
                    break;
                case 9:
                    replay.outside = true;
                    break;
                case 11: // a key of a line the player sent
                    typeKey(a);
                    break;
                default:
                    break;
            }
        }
    }

    // ---- The camera ----
    //
    // From 254 (and on 225) the client reports its camera's pitch and yaw
    // while the arrow keys turn it, and the replay sets the client's own
    // camera to match, eased between reports. 244 and 245.2 don't report it,
    // but their input report has the arrow keys, which are pressed again.

    const ARROWS = { 1: 'ArrowLeft', 2: 'ArrowRight', 3: 'ArrowUp', 4: 'ArrowDown' };
    const ARROW_CODES = { 1: 37, 2: 39, 3: 38, 4: 40 };
    const YAW_TURN = 2048;
    // Reports this far apart are separate turns, not one to ease along.
    const CAMERA_EASE_MS = 1500;

    function arrowKey(type, key) {
        if (typeof replay.rec.header.cameraOp === 'number' || !ARROWS[key]) {
            return;
        }
        if (type === 'keydown') {
            replay.arrowsHeld.add(key);
        } else {
            replay.arrowsHeld.delete(key);
        }
        if (replay.freeLook) {
            return;
        }
        canvas()?.dispatchEvent(new KeyboardEvent(type, { key: ARROWS[key], code: ARROWS[key], keyCode: ARROW_CODES[key], bubbles: true }));
    }

    // Types a key of a chat line, private message or command. The client
    // ignores untrusted key events without a code.
    function typeKey(key) {
        const init = { key, code: key === ' ' ? 'Space' : key, bubbles: true };
        canvas()?.dispatchEvent(new KeyboardEvent('keydown', init));
        canvas()?.dispatchEvent(new KeyboardEvent('keyup', init));
    }

    function releaseArrows() {
        for (const key of [1, 2, 3, 4]) {
            arrowKey('keyup', key);
        }
    }

    // The client's camera fields, by name: the two it writes into its camera
    // report, right after the report's opcode.
    function cameraFields() {
        if (replay.cameraFields !== undefined) {
            return replay.cameraFields;
        }
        replay.cameraFields = null;
        const op = replay.rec.header.cameraOp;
        if (typeof op !== 'number' || !replay.client) {
            return null;
        }
        const report = new RegExp(`\\(${op}\\),this\\.[\\w$]+\\.p2\\(this\\.([\\w$]+)\\),this\\.[\\w$]+\\.p2\\(this\\.([\\w$]+)\\)`);
        for (let proto = Object.getPrototypeOf(replay.client); proto && proto !== Object.prototype; proto = Object.getPrototypeOf(proto)) {
            for (const name of Object.getOwnPropertyNames(proto)) {
                const fn = name === 'constructor' ? null : Object.getOwnPropertyDescriptor(proto, name)?.value;
                const match = typeof fn === 'function' && report.exec(Function.prototype.toString.call(fn));
                if (match) {
                    replay.cameraFields = { pitch: match[1], yaw: match[2] };
                    return replay.cameraFields;
                }
            }
        }
        return null;
    }

    function playCamera(ms) {
        const cams = replay.cameras;
        const fields = cams.length && replay.streaming && !replay.freeLook ? cameraFields() : null;
        if (!fields) {
            return;
        }
        while (replay.camera + 1 < cams.length && cams[replay.camera + 1][0] <= ms) {
            replay.camera++;
        }
        // Before the first report, the camera is where that report found it.
        const [at, pitch, yaw] = cams[Math.max(0, replay.camera)];
        let p = pitch;
        let y = yaw;
        const next = cams[replay.camera + 1];
        if (replay.camera >= 0 && next && next[0] - at <= CAMERA_EASE_MS) {
            const f = (ms - at) / (next[0] - at);
            p = pitch + (next[1] - pitch) * f;
            const turn = ((next[2] - yaw + YAW_TURN * 1.5) % YAW_TURN) - YAW_TURN / 2;
            y = (yaw + turn * f + YAW_TURN) % YAW_TURN;
        }
        replay.client[fields.pitch] = Math.round(p);
        replay.client[fields.yaw] = Math.round(y) & (YAW_TURN - 1);
    }

    // ---- The minimap ----
    //
    // The client tilts and zooms its minimap a little at random (drifting a
    // step every 10 seconds or so) against bots that read the screen, and a
    // minimap click sends what they were, with the camera's yaw; 225's camera
    // report sends them too. The replay sets the client's to match, so its
    // minimap shows as the player's did and a recorded minimap click puts the
    // flag where theirs went. On 244 and 245.2, which don't report the
    // camera, a minimap click's yaw also puts right the camera that the
    // recorded arrow keys turn (and that a login starts at random).

    function minimapsIn(inputs) {
        return inputs.filter((i) => i[1] === 12).map(([at, , angle, zoom, yaw]) => [at, angle, zoom, yaw ?? null]);
    }

    // The client's fields, by name: those it writes into a minimap click
    // around its fixed bytes 57 and 89.
    function minimapFields() {
        if (replay.minimapFields !== undefined) {
            return replay.minimapFields;
        }
        replay.minimapFields = null;
        if (!replay.client) {
            return null;
        }
        const trailer = /\.p2\(this\.([\w$]+)\),this\.[\w$]+\.p1\(57\),this\.[\w$]+\.p1\(this\.([\w$]+)\),this\.[\w$]+\.p1\(this\.([\w$]+)\),this\.[\w$]+\.p1\(89\)/;
        for (let proto = Object.getPrototypeOf(replay.client); proto && proto !== Object.prototype; proto = Object.getPrototypeOf(proto)) {
            for (const name of Object.getOwnPropertyNames(proto)) {
                const fn = name === 'constructor' ? null : Object.getOwnPropertyDescriptor(proto, name)?.value;
                const match = typeof fn === 'function' && trailer.exec(Function.prototype.toString.call(fn));
                if (match) {
                    replay.minimapFields = { yaw: match[1], angle: match[2], zoom: match[3] };
                    return replay.minimapFields;
                }
            }
        }
        return null;
    }

    function playMinimap(ms) {
        const marks = replay.minimaps;
        const fields = marks.length && replay.streaming && !replay.freeLook ? minimapFields() : null;
        if (!fields) {
            return;
        }
        const was = replay.minimap;
        let yaw = null;
        while (replay.minimap + 1 < marks.length && marks[replay.minimap + 1][0] <= ms) {
            replay.minimap++;
            yaw = marks[replay.minimap][3] ?? yaw;
        }
        // Before the first, the minimap is as that one found it.
        const [, angle, zoom] = marks[Math.max(0, replay.minimap)];
        replay.client[fields.angle] = angle;
        replay.client[fields.zoom] = zoom;
        if (replay.minimap !== was && yaw !== null && typeof replay.rec.header.cameraOp !== 'number') {
            replay.client[fields.yaw] = yaw & (YAW_TURN - 1);
        }
    }

    // ---- The overlay: the player's cursor over the game ----

    let overlay = null;

    function makeOverlay() {
        const c = canvas();
        if (!c || overlay) {
            return;
        }
        overlay = document.createElement('canvas');
        overlay.id = 'replay-overlay';
        overlay.width = c.width;
        overlay.height = c.height;
        overlay.style.cssText = 'position:absolute;left:0;top:0;width:100%;height:100%;cursor:default;';
        curtain = document.createElement('canvas');
        curtain.id = 'replay-curtain';
        curtain.width = c.width;
        curtain.height = c.height;
        curtain.style.cssText = 'position:absolute;left:0;top:0;width:100%;height:100%;display:none;pointer-events:none;';
        c.parentElement.style.position = 'relative';
        c.parentElement.appendChild(curtain);
        c.parentElement.appendChild(overlay);
        fitPixels();
        overlay.style.pointerEvents = replay.freeLook ? 'none' : 'auto';
        // The viewer only watches: none of their input reaches the client
        // (in free look the overlay lets it through).
        for (const type of ['mousedown', 'mouseup', 'mousemove', 'click', 'dblclick', 'contextmenu', 'wheel', 'pointerdown', 'pointerup', 'pointermove', 'pointerenter', 'pointerleave', 'touchstart', 'touchend', 'touchmove']) {
            overlay.addEventListener(type, (e) => {
                e.stopPropagation();
                e.preventDefault();
            }, { passive: false });
        }
        requestAnimationFrame(draw);
    }

    // Free look pauses. Ending it puts the client back as it was before:
    // if the viewer clicked or turned anything, it starts again from the
    // nearest keyframe to the same spot (unless `reset` is false, when it's
    // going somewhere else anyway).
    function setFreeLook(on, reset = true) {
        const was = replay.freeLook;
        replay.freeLook = on;
        if (overlay) {
            overlay.style.pointerEvents = on ? 'none' : 'auto';
        }
        if (on) {
            replay.userPaused = true;
            replay.touched = false;
            // What it looks like before the viewer changes anything, to show
            // while it's put back.
            replay.lookShot = null;
            canvas()?.toBlob((blob) => (replay.lookShot = blob), 'image/webp', 0.8);
            canvas()?.focus?.();
            applyClock();
            return;
        }
        if (!was) {
            return;
        }
        // Let go of any arrow key the viewer was holding.
        for (const key of Object.values(ARROWS)) {
            canvas()?.dispatchEvent(new KeyboardEvent('keyup', { key, code: key, bubbles: true }));
        }
        if (reset && replay.touched && replay.rec && typeof replay.rec.header.logout === 'number') {
            replay.settling = null;
            unhold();
            if (replay.lookShot) {
                showCurtain({ blob: replay.lookShot });
            }
            restart(replay.lastMs);
            return;
        }
        if (replay.cursor && !replay.outside) {
            // The player's mouse again, where it is.
            pointer('pointermove', replay.cursor[0], replay.cursor[1]);
        }
        applyClock();
    }

    // The viewer's keyboard: only the arrow keys, and only in free look. No
    // typing: the client would show it as said.
    for (const type of ['keydown', 'keyup', 'keypress']) {
        window.addEventListener(type, (e) => {
            if (!e.isTrusted) {
                return;
            }
            if (replay.freeLook && e.key.startsWith('Arrow')) {
                replay.touched = true;
            } else {
                e.stopImmediatePropagation();
                e.preventDefault();
            }
        }, true);
    }
    // A click changes the client (a tab, a window); hovering doesn't.
    for (const type of ['pointerdown', 'mousedown', 'wheel']) {
        window.addEventListener(type, (e) => {
            if (e.isTrusted && replay.freeLook) {
                replay.touched = true;
            }
        }, true);
    }

    // What the overlay shows, and the time what's faded was dropped at: it's
    // drawn again only when that changes, not every frame paused.
    let drawn = null;
    let fadedAt = null;
    function draw() {
        requestAnimationFrame(draw);
        const showing = replay.rec && (replay.streaming || replay.frame !== 0);
        const ms = replay.lastMs;
        if (showing && ms !== fadedAt) {
            fadedAt = ms;
            replay.trail = replay.trail.filter(([t]) => ms - t < 1000);
            replay.clicks = replay.clicks.filter(([t]) => ms - t < 600);
            replay.keys = replay.keys.filter(([t]) => ms - t < 1500);
        }
        const now = showing
            ? [ms, replay.trail.length, replay.clicks.length, replay.keys.length, replay.cursor?.[0],
                replay.cursor?.[1], replay.outside, replay.focus, overlay.width, overlay.height].join()
            : '';
        if (now === drawn) {
            return;
        }
        drawn = now;
        const ctx = overlay.getContext('2d');
        ctx.clearRect(0, 0, overlay.width, overlay.height);
        if (!showing) {
            return;
        }

        if (replay.trail.length > 1) {
            ctx.strokeStyle = 'rgba(255, 255, 255, 0.35)';
            ctx.lineWidth = 1;
            ctx.beginPath();
            replay.trail.forEach(([, x, y], i) => (i ? ctx.lineTo(x + 0.5, y + 0.5) : ctx.moveTo(x + 0.5, y + 0.5)));
            ctx.stroke();
        }
        for (const [t, x, y, right] of replay.clicks) {
            const age = (ms - t) / 600;
            ctx.strokeStyle = right ? `rgba(255, 64, 64, ${1 - age})` : `rgba(255, 225, 57, ${1 - age})`;
            ctx.lineWidth = 2;
            ctx.beginPath();
            ctx.arc(x + 0.5, y + 0.5, 4 + age * 14, 0, Math.PI * 2);
            ctx.stroke();
        }
        if (replay.cursor && !replay.outside) {
            const [x, y] = replay.cursor;
            ctx.save();
            ctx.translate(x, y);
            ctx.fillStyle = '#fff';
            ctx.strokeStyle = '#000';
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(0, 0);
            ctx.lineTo(0, 15);
            ctx.lineTo(4, 11);
            ctx.lineTo(7, 17);
            ctx.lineTo(9, 16);
            ctx.lineTo(6, 10);
            ctx.lineTo(11, 10);
            ctx.closePath();
            ctx.fill();
            ctx.stroke();
            ctx.restore();
        }
        const badges = [];
        if (!replay.focus) {
            badges.push('Game window not focused');
        }
        if (replay.outside) {
            badges.push('Cursor outside the game');
        }
        if (replay.keys.length) {
            badges.push('Keys: ' + replay.keys.map(([, k]) => k).join(' '));
        }
        ctx.font = 'bold 12px Arial, sans-serif';
        badges.forEach((text, i) => {
            const w = ctx.measureText(text).width + 12;
            ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
            ctx.fillRect(6, 6 + i * 22, w, 18);
            ctx.fillStyle = '#ffe139';
            ctx.fillText(text, 12, 19 + i * 22);
        });
    }

    // ---- Starting up ----

    // Finds the client's login method: the one that picks the ISAAC seeds.
    function loginMethod(client) {
        const seeds = /99999999\s*\*\s*Math\.random\(\)|Math\.random\(\)\s*\*\s*99999999/;
        for (let proto = Object.getPrototypeOf(client); proto && proto !== Object.prototype; proto = Object.getPrototypeOf(proto)) {
            for (const name of Object.getOwnPropertyNames(proto)) {
                if (name === 'constructor') {
                    continue;
                }
                const fn = Object.getOwnPropertyDescriptor(proto, name)?.value;
                if (typeof fn !== 'function') {
                    continue;
                }
                const source = Function.prototype.toString.call(fn);
                if (!source.startsWith('class') && seeds.test(source)) {
                    return fn;
                }
            }
        }
        return null;
    }

    function clientReady() {
        if (!replay.ready) {
            replay.ready = true;
            // Not from inside the client's own code (this runs from its
            // timers): it finishes loading first, and sets up its title
            // screen (see restart).
            const started = realNow();
            const wait = () => {
                if (replay.flamesAt === null && realNow() - started < TITLE_SETTLE_MS) {
                    realSetTimeout(wait, 20);
                } else {
                    maybeLogin();
                }
            };
            realSetTimeout(wait, 0);
        }
    }

    function maybeLogin() {
        if (!replay.ready || !replay.client || !replay.rec || replay.loggingIn || replay.frame > 0) {
            return;
        }
        const login = loginMethod(replay.client);
        if (!login) {
            post({ type: 'error', message: "Couldn't find the client's login." });
            return;
        }
        replay.loggingIn = true;
        gateDraws();
        if (replay.seekTo !== null) {
            drive();
        } else {
            setClock(replay.userSpeed, false);
        }
        note('login called');
        login.call(replay.client, replay.rec.header.player || 'replay', 'replay', false);
    }

    // ---- Drawing only what's seen ----
    //
    // Fast-forwarding, the client draws the scene after every step, which
    // nobody sees (it's behind the seek cover, or it's the scout) and costs
    // most of the time. The client's draw step is found in its main loop:
    // the first method it awaits after its logic catch-up (`count &= 255`).
    // While unseen, it's skipped: as a slow machine dropping frames, the next
    // draw shows things as they are then.

    function drawMethod() {
        const after = /&=\s*(?:255|0xff)\b[\s\S]{0,240}?await this\.([\w$]+)\(\)/;
        for (let proto = Object.getPrototypeOf(replay.client); proto && proto !== Object.prototype; proto = Object.getPrototypeOf(proto)) {
            for (const name of Object.getOwnPropertyNames(proto)) {
                const fn = name === 'constructor' ? null : Object.getOwnPropertyDescriptor(proto, name)?.value;
                const match = typeof fn === 'function' ? after.exec(Function.prototype.toString.call(fn)) : null;
                if (match && typeof replay.client[match[1]] === 'function') {
                    return match[1];
                }
            }
        }
        return null;
    }

    function gateDraws() {
        if (replay.drawGated || !replay.client) {
            return;
        }
        replay.drawGated = true;
        const name = drawMethod();
        if (!name) {
            return;
        }
        const draw = replay.client[name];
        replay.drawHooked = true;
        replay.client[name] = async function (...args) {
            if (unseen()) {
                return undefined;
            }
            const result = await draw.apply(this, args);
            replay.draws++;
            const then = replay.afterDraw;
            if (then) {
                replay.afterDraw = null;
                then();
            }
            return result;
        };
    }

    // Nothing would see this draw: the scout between pictures, or the replay
    // fast-forwarding behind the seek cover. The title screen always draws.
    function unseen() {
        if (!replay.streaming || replay.settling || replay.afterDraw || clickComing()) {
            return false;
        }
        return SCOUT || replay.seekTo !== null;
    }

    // A recorded click is about to be played. The client works out what's
    // under the cursor, and the menu a click picks from, while it draws: it
    // draws on the way to one, so the click does what it did.
    function clickComing() {
        const inputs = replay.rec?.header.inputs;
        if (!inputs) {
            return false;
        }
        // The last draw or two before it: the menu it picks from is built
        // then.
        const until = replay.lastMs + 2 * DRIVE_STEP;
        for (let i = replay.input; i < inputs.length && inputs[i][0] <= until; i++) {
            if (inputs[i][1] === 2 || inputs[i][1] === 4) {
                return true;
            }
        }
        return false;
    }

    // Going back: the client can't rewind, so it's logged out (the game's
    // own logout packet, which resets it as a real logout does) and logged
    // in again from the start of the recording, then fast-forwarded. No
    // reload: the client keeps its loaded cache.
    // The keyframe to start from to reach `ms`: the last one at or before it.
    function keyframeFor(ms) {
        if (replay.noKeyframes) {
            return null;
        }
        const { keyframes } = markedFrames();
        const frames = replay.rec.frames;
        let lo = 0;
        let hi = keyframes.length;
        while (lo < hi) {
            const mid = (lo + hi) >> 1;
            if (frames[keyframes[mid]].ms <= ms) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        return lo ? keyframes[lo - 1] : null;
    }

    // Where the recording's keyframes and reconnects are (frame indexes,
    // in order), found once: frames only ever come after those there.
    let marks = null;
    function markedFrames() {
        const frames = replay.rec.frames;
        if (marks?.of !== frames) {
            marks = { of: frames, seen: 0, keyframes: [], reconnects: [] };
        }
        for (; marks.seen < frames.length; marks.seen++) {
            const kind = frames[marks.seen].kind;
            if (kind === FRAME_KEYFRAME) {
                marks.keyframes.push(marks.seen);
            } else if (kind === FRAME_RECONNECT) {
                marks.reconnects.push(marks.seen);
            }
        }
        return marks;
    }

    // How long after a logout the client has to set up its title screen.
    const TITLE_SETTLE_MS = 700;

    // What belonged to the last recording: its frames, where it had got to,
    // what the scout was doing. (The client and what was found in it stay.)
    function forgetRecording() {
        replay.logoutOp = replay.rec?.header.logout ?? replay.logoutOp;
        if (replay.freeLook) {
            setFreeLook(false, false);
        }
        replay.settling = null;
        unhold();
        shots.list = [];
        shots.bytes = 0;
        shots.gap = 500;
        bufferedFor = -1;
        curtainUp = false;
        if (curtain) {
            curtain.style.display = 'none';
        }
        scouting.current = null;
        scouting.pending = null;
        scouting.sweeping = false;
        Object.assign(sweep, { fine: 0, step: 0, from: 0, done: false, sent: [] });
        replay.scoutStep = 0;
        replay.live = null;
        replay.starved = false;
        replay.ended = false;
        replay.seekTo = null;
        replay.startAt = null;
        replay.afterDraw = null;
    }

    function restart(to) {
        replay.seekTo = to;
        replay.startAt = keyframeFor(to);
        const socket = replay.socket;
        releaseArrows();
        Object.assign(replay, {
            streaming: false,
            frame: 0,
            input: 0,
            camera: -1,
            minimap: -1,
            lastMs: 0,
            ended: false,
            closedAt: null,
            cursor: null,
            trail: [],
            clicks: [],
            keys: [],
            focus: true,
            outside: false,
            buttons: 0,
        });
        drive();
        // A later restart (another seek while this one logs in again) takes
        // over: this one's waiting stops.
        const gen = ++replay.restartGen;
        const started = realNow();
        if (socket && socket.readyState === 1 && socket.stage === 'game' && replay.cipher) {
            // In a session: the game's own logout, which resets the client as
            // a real one does.
            socket.deliver(new Uint8Array([(replay.rec.header.logout + replay.cipher.next()) & 0xff]));
        } else if (socket) {
            // Mid-login (an earlier restart's): the connection just ends.
            socket.drop();
            replay.socket = null;
        } else {
            replay.loggingIn = false;
            maybeLogin();
            return;
        }
        replay.loggingIn = true;
        // Log in again once the client has dropped the connection and
        // finished setting up its title screen: that runs on after the
        // logout (it decodes the title image), and finishing during the
        // login it would take the game's drawing areas away for good,
        // leaving its loading screen up.
        let dropped = null;
        const wait = () => {
            if (gen !== replay.restartGen) {
                return;
            }
            const now = realNow();
            if (replay.socket && now - started < 5000) {
                realSetTimeout(wait, 20);
                return;
            }
            dropped ??= now;
            // Set up once its flames start (every revision runs them on a
            // 35ms timer); if they never do, after a while anyway.
            const titled = replay.flamesAt !== null && replay.flamesAt >= started && now - replay.flamesAt >= 30;
            if (!titled && now - dropped < TITLE_SETTLE_MS) {
                realSetTimeout(wait, 20);
                return;
            }
            replay.socket?.drop();
            replay.socket = null;
            replay.loggingIn = false;
            maybeLogin();
        };
        wait();
    }

    function post(message) {
        if (window.parent !== window) {
            window.parent.postMessage({ source: 'rs-replay', ...message }, '*');
        }
    }

    window.addEventListener('message', (e) => {
        if (e.source !== window.parent || !e.data || typeof e.data !== 'object') {
            return;
        }
        const msg = e.data;
        if (msg.type === 'load' && msg.file instanceof ArrayBuffer) {
            // For tests: seek the long way, to compare; and other step sizes.
            replay.noKeyframes = msg.keyframes === false;
            if (typeof msg.driveStep === 'number' && msg.driveStep >= 10) {
                DRIVE_STEP = msg.driveStep;
            }
            let rec;
            try {
                rec = parse(msg.file);
            } catch (err) {
                post({ type: 'error', message: String(err.message || err) });
                return;
            }
            // A page already playing one (the console keeps its client
            // loaded between recordings): that one's session ends here.
            const another = replay.rec !== null || replay.frame > 0 || replay.socket !== null;
            if (another) {
                forgetRecording();
            }
            replay.rec = rec;
            replay.started = false;
            replay.cameras = rec.header.inputs.filter((i) => i[1] === 10).map(([at, , pitch, yaw]) => [at, pitch, yaw]);
            replay.camera = -1;
            replay.minimaps = minimapsIn(rec.header.inputs);
            replay.minimap = -1;
            replay.live = rec.header.live ? { delayMs: rec.header.live.delayMs || 0, ended: !!rec.header.live.ended } : null;
            replay.userSpeed = Math.min(8, Math.max(0.25, Number(msg.speed) || 1));
            replay.userPaused = !!msg.paused;
            const seekTo = typeof msg.seek === 'number' && msg.seek > 0 ? msg.seek : null;
            if (SCOUT && seekTo !== null) {
                scouting.current = seekTo;
            }
            if (msg.freeLook) {
                replay.freeLook = true;
                if (overlay) {
                    overlay.style.pointerEvents = 'none';
                }
            }
            if (another) {
                restart(seekTo ?? 0);
                return;
            }
            if (seekTo !== null) {
                replay.seekTo = seekTo;
                replay.startAt = keyframeFor(seekTo);
            }
            maybeLogin();
        } else if (msg.type === 'append' && msg.file instanceof ArrayBuffer && replay.rec && replay.live) {
            let more;
            try {
                more = parse(msg.file);
            } catch (err) {
                post({ type: 'error', message: String(err.message || err) });
                return;
            }
            append(more);
        } else if (msg.type === 'unload') {
            // Not watching any more: the session ends, and the client waits
            // at its title screen (still) for the next.
            forgetRecording();
            if (replay.socket && replay.socket.readyState === 1 && replay.cipher && replay.logoutOp !== null) {
                replay.socket.deliver(new Uint8Array([(replay.logoutOp + replay.cipher.next()) & 0xff]));
            }
            replay.rec = null;
            replay.started = false;
            replay.userPaused = true;
            unhold();
            setClock(1, false);
            realSetTimeout(() => {
                if (!replay.rec) {
                    setClock(1, true);
                }
            }, 2000);
        } else if (msg.type === 'play') {
            replay.userPaused = false;
            replay.settling = null;
            if (replay.ended && replay.rec) {
                // Played again at the end: from the start.
                setFreeLook(false, false);
                seek(0);
                return;
            }
            // Playing ends free look, back where it was.
            if (replay.freeLook) {
                setFreeLook(false);
                return;
            }
            applyClock();
        } else if (msg.type === 'pause') {
            replay.userPaused = true;
            applyClock();
        } else if (msg.type === 'speed' && typeof msg.value === 'number') {
            replay.userSpeed = Math.min(8, Math.max(0.25, msg.value));
            applyClock();
        } else if (msg.type === 'freeLook') {
            setFreeLook(!!msg.on);
        } else if (msg.type === 'seek' && typeof msg.ms === 'number') {
            // Not while the client starts: logging in again over its first
            // login stalls it. It starts where the load said.
            if (!replay.started) {
                report();
                return;
            }
            // Going somewhere else: no need to put this spot back first.
            if (replay.freeLook) {
                setFreeLook(false, false);
            }
            seek(msg.ms);
            report();
        } else if (msg.type === 'shoot' && typeof msg.ms === 'number' && SCOUT) {
            shoot(msg.ms);
        } else if (msg.type === 'addShot' && typeof msg.ms === 'number' && msg.blob instanceof Blob) {
            // A frame the scout fetched: seeking there shows it at once too.
            replay.scoutStep = Math.max(replay.scoutStep, Number(msg.step) || 0);
            addShot(msg.ms, msg.blob);
        } else if (msg.type === 'preview' && typeof msg.ms === 'number') {
            // Hovering the timeline: the nearest watched frame, if any.
            const f = frameNear(msg.ms);
            post({ type: 'preview', ms: msg.ms, frameMs: f ? f.ms : null, blob: f ? f.blob : null });
        }
    });

    // `b` (sorted by time) merged into `a` (the same) from index `from`: what
    // comes before that has been played.
    // Only what comes after `b`'s first is touched: it's mostly all after.
    function mergeFrom(a, from, b) {
        if (!b.length) {
            return a;
        }
        let lo = from;
        let hi = a.length;
        while (lo < hi) {
            const mid = (lo + hi) >> 1;
            if (a[mid][0] <= b[0][0]) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        const rest = a.splice(lo);
        let i = 0;
        let j = 0;
        while (i < rest.length || j < b.length) {
            if (j >= b.length || (i < rest.length && rest[i][0] <= b[j][0])) {
                a.push(rest[i++]);
            } else {
                a.push(b[j++]);
            }
        }
        return a;
    }

    // What's been recorded since, watching live. Input reports come in
    // batches, so some of it may be earlier than what's been played already:
    // that plays straight away.
    function append(more) {
        const rec = replay.rec;
        rec.frames.push(...more.frames);
        const inputs = more.header.inputs || [];
        rec.header.inputs = mergeFrom(rec.header.inputs, replay.input, inputs);
        const cams = inputs.filter((i) => i[1] === 10).map(([at, , pitch, yaw]) => [at, pitch, yaw]);
        replay.cameras = mergeFrom(replay.cameras, Math.max(0, replay.camera + 1), cams);
        replay.minimaps = mergeFrom(replay.minimaps, Math.max(0, replay.minimap + 1), minimapsIn(inputs));
        rec.header.notes.push(...(more.header.notes || []));
        rec.header.durationMs = Math.max(rec.header.durationMs, more.header.durationMs || 0);
        rec.header.frames += more.frames.length;
        if (more.header.live?.ended) {
            replay.live.ended = true;
        }
        if (replay.starved) {
            replay.starved = false;
            unhold();
            applyClock();
        }
    }

    function seek(ms) {
        replay.settling = null;
        replay.starved = false;
        unhold();
        // Somewhere already watched shows at once; the client catches up
        // behind it.
        const shown = frameNear(ms);
        if (shown) {
            showCurtain(shown);
        }
        const keyframe = keyframeFor(ms);
        const far = keyframe !== null && replay.rec.frames[keyframe].ms > replay.lastMs + KEYFRAME_JUMP_MS;
        if (ms < replay.lastMs || far) {
            if (typeof replay.rec.header.logout === 'number') {
                restart(ms);
            } else {
                // A file from an older world: the console reloads the page.
                post({ type: 'reload', ms });
            }
            return;
        }
        replay.seekTo = ms;
        drive();
    }

    // ---- Frames already watched ----
    //
    // What the game showed, a frame every `gap` ms of the recording, so going
    // back somewhere already watched (or hovering the timeline) shows it at
    // once. Kept under FRAME_BUDGET bytes: past it, every other frame goes and
    // the gap doubles, so even hours of play stay covered end to end, just
    // more coarsely.

    const FRAME_BUDGET = 160 * 1024 * 1024;
    const shots = { list: [], bytes: 0, gap: 500, busy: false };
    let curtain = null;
    let curtainUp = false;

    // The index of the first frame at or after `ms`.
    function shotIndex(ms) {
        let lo = 0;
        let hi = shots.list.length;
        while (lo < hi) {
            const mid = (lo + hi) >> 1;
            if (shots.list[mid].ms < ms) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        return lo;
    }

    // The frame nearest `ms`, if one is close enough to stand in for it.
    function frameNear(ms) {
        const i = shotIndex(ms);
        let best = null;
        for (const f of [shots.list[i - 1], shots.list[i]]) {
            if (f && (!best || Math.abs(f.ms - ms) < Math.abs(best.ms - ms))) {
                best = f;
            }
        }
        // The scout's sweep spaces frames `scoutStep` apart: one of those
        // stands in for anywhere between them.
        const within = Math.max(2000, shots.gap * 2, replay.scoutStep * 0.6);
        return best && Math.abs(best.ms - ms) <= within ? best : null;
    }

    function sceneBuilt() {
        const field = sceneField();
        return !field || replay.client[field] === 2;
    }

    // Keeps a frame of what's showing now, if it's playing normally and
    // there's none close by.
    function capture() {
        const c = canvas();
        if (SCOUT || !c || shots.busy || !replay.rec || !replay.streaming || replay.seekTo !== null || replay.settling
            || replay.held !== null || curtainUp || !sceneBuilt()) {
            return;
        }
        const ms = Math.round(replay.lastMs);
        const i = shotIndex(ms);
        const near = [shots.list[i - 1], shots.list[i]].some((f) => f && Math.abs(f.ms - ms) < shots.gap);
        if (near) {
            return;
        }
        shots.busy = true;
        c.toBlob((blob) => {
            shots.busy = false;
            if (blob) {
                addShot(ms, blob);
            }
        }, 'image/webp', 0.8);
    }

    function addShot(ms, blob) {
        const i = shotIndex(ms);
        if ([shots.list[i - 1], shots.list[i]].some((f) => f && Math.abs(f.ms - ms) < 100)) {
            return;
        }
        shots.list.splice(i, 0, { ms, blob });
        shots.bytes += blob.size;
        while (shots.bytes > FRAME_BUDGET && shots.list.length > 1) {
            shots.gap *= 2;
            const kept = [];
            let bytes = 0;
            for (const f of shots.list) {
                if (!kept.length || f.ms - kept[kept.length - 1].ms >= shots.gap) {
                    kept.push(f);
                    bytes += f.blob.size;
                }
            }
            shots.list = kept;
            shots.bytes = bytes;
        }
    }

    // ---- Scouting ----

    // The moment being fetched (`sweeping` when it's the sweep's, not asked
    // for), and the next one asked for.
    const scouting = { current: null, pending: null, sweeping: false };

    // With nothing asked for, the scout sweeps the recording: a frame every
    // `step` ms, coarse first (about a twelfth of it apart), then halving
    // the gaps down to `fine` (5 seconds, or wider so it's at most ~600
    // frames), each pass forward from the start. What's asked for comes
    // first.
    const sweep = { fine: 0, step: 0, from: 0, done: false, sent: [] };

    function sweepStart() {
        const duration = replay.rec.header.durationMs || 0;
        sweep.fine = Math.max(5000, Math.ceil(duration / 600 / 1000) * 1000);
        sweep.step = sweep.fine;
        while (sweep.step * 12 < duration) {
            sweep.step *= 2;
        }
        sweep.from = sweep.step;
    }

    // A frame was sent within half a fine step of `ms`.
    function swept(ms) {
        const list = sweep.sent;
        let lo = 0;
        let hi = list.length;
        while (lo < hi) {
            const mid = (lo + hi) >> 1;
            if (list[mid] < ms) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        return [list[lo - 1], list[lo]].some((t) => t !== undefined && Math.abs(t - ms) <= sweep.fine / 2);
    }

    // The next moment the sweep still needs, or null when it's done.
    function sweepNext() {
        const duration = replay.rec.header.durationMs || 0;
        if (!sweep.fine) {
            sweepStart();
        }
        while (!sweep.done) {
            for (let t = sweep.from; t < duration; t += sweep.step) {
                if (!swept(t)) {
                    sweep.from = t + sweep.step;
                    return t;
                }
            }
            if (sweep.step <= sweep.fine) {
                sweep.done = true;
            } else {
                sweep.step /= 2;
                sweep.from = sweep.step;
            }
        }
        return null;
    }

    function shoot(ms) {
        if (scouting.current !== null && replay.seekTo !== null && replay.streaming && !replay.loggingIn) {
            // On its way somewhere else: go here instead. (Mid-login or
            // drawing the last spot, it finishes first.)
            scouting.current = ms;
            scouting.sweeping = false;
            seek(ms);
            return;
        }
        scouting.pending = ms;
        scoutNext();
    }

    function scoutNext() {
        if (!replay.rec || scouting.current !== null) {
            return;
        }
        let ms = scouting.pending;
        scouting.sweeping = ms === null;
        if (ms === null) {
            ms = sweepNext();
            if (ms === null) {
                return;
            }
        }
        scouting.pending = null;
        scouting.current = ms;
        replay.userPaused = true;
        seek(ms);
    }

    // There and drawn: send the frame back, then go on to the next.
    function scouted() {
        scouting.current = null;
        const c = canvas();
        if (!c || !replay.streaming || !sceneBuilt()) {
            scoutNext();
            return;
        }
        const ms = Math.round(replay.lastMs);
        c.toBlob((blob) => {
            if (blob) {
                sweep.sent.splice(sweep.sent.findIndex((t) => t > ms) >>> 0, 0, ms);
                post({ type: 'shot', ms, blob, step: sweep.fine });
            }
            scoutNext();
        }, 'image/webp', 0.8);
    }

    function showCurtain(frame) {
        if (!curtain) {
            return;
        }
        curtainUp = true;
        createImageBitmap(frame.blob).then((bitmap) => {
            if (curtainUp) {
                curtain.getContext('2d').drawImage(bitmap, 0, 0, curtain.width, curtain.height);
                curtain.style.display = 'block';
            }
            bitmap.close();
        }, () => {});
    }

    // Down once the client has caught up and drawn the spot.
    function checkCurtain() {
        // (Drawn since the seek: draws are skipped on the way.)
        const drawn = !replay.drawHooked || replay.draws > replay.seekDraws;
        if (curtainUp && replay.seekTo === null && !replay.settling && replay.streaming && sceneBuilt() && drawn) {
            curtainUp = false;
            curtain.style.display = 'none';
        }
    }

    // Steps the replay on real time, so a paused one still reports.
    function report() {
        step();
        checkCurtain();
        capture();
        post({
            type: 'state',
            ms: replay.lastMs,
            durationMs: replay.rec?.header.durationMs ?? 0,
            playing: !paused && !replay.userPaused,
            speed: replay.userSpeed,
            seeking: replay.seekTo !== null || replay.settling !== null,
            seekTo: replay.seekTo ?? replay.settling?.to ?? null,
            live: replay.streaming,
            loading: !replay.streaming && replay.frame === 0,
            // The client is starting: no seeking yet.
            starting: !replay.started,
            // Watching live: caught up and waiting for more, and whether the
            // session has ended.
            starved: replay.starved,
            sessionEnded: replay.live ? replay.live.ended : null,
            // A watched frame stands in while it seeks.
            preview: curtainUp,
            // What has frames (to preview or jump to), when that changed.
            buffered: buffered(),
        });
    }

    // The stretches of the recording with a kept frame near enough to stand
    // in (see frameNear), as [from, to] ms. Only when they've changed.
    let bufferedFor = -1;
    function buffered() {
        if (SCOUT || shots.list.length === bufferedFor) {
            return undefined;
        }
        bufferedFor = shots.list.length;
        const within = Math.max(2000, shots.gap * 2, replay.scoutStep * 0.6);
        const ranges = [];
        for (const f of shots.list) {
            const from = Math.max(0, f.ms - within);
            const to = f.ms + within;
            const last = ranges[ranges.length - 1];
            if (last && from <= last[1]) {
                last[1] = Math.max(last[1], to);
            } else {
                ranges.push([from, to]);
            }
        }
        return ranges;
    }
    // Packets go out on virtual time; the report on real time.
    window.setInterval(step, 10);
    (function tick() {
        report();
        realSetTimeout(tick, 200);
    })();

    window.__replay = {
        // For tests and the curious: the replay's state.
        state: replay,
        // What a benchmark wants to know, on the wall's clock.
        metrics: () => ({
            now: realNow(),
            ready: replay.ready,
            live: replay.streaming,
            built: replay.streaming && sceneBuilt(),
            ms: replay.lastMs,
            seeking: replay.seekTo !== null || replay.settling !== null,
            curtain: curtainUp,
            shots: shots.list.length,
            shotBytes: shots.bytes,
            draws: replay.draws,
            drawHooked: replay.drawHooked,
            loggingIn: replay.loggingIn,
            log: replay.log.slice(-25),
            camera: (() => {
                const f = cameraFields();
                const cams = replay.cameras || [];
                let i = -1;
                while (i + 1 < cams.length && cams[i + 1][0] <= replay.lastMs) i++;
                return {
                    fields: f,
                    reports: cams.length,
                    pitch: f ? replay.client[f.pitch] : null,
                    yaw: f ? replay.client[f.yaw] : null,
                    expected: i >= 0 ? cams[i] : cams[0] ?? null,
                };
            })(),
            minimap: (() => {
                const f = minimapFields();
                return {
                    fields: f,
                    marks: (replay.minimaps || []).length,
                    angle: f ? replay.client[f.angle] : null,
                    zoom: f ? replay.client[f.zoom] : null,
                    yaw: f ? replay.client[f.yaw] : null,
                };
            })(),
            socket: replay.socket?.stage ?? null,
            frame: replay.frame,
            driven,
            paused,
            timers: timers.size,
            loopWaiting: loopWaiting(),
            shotTimes: shots.list.map((f) => f.ms),
            scoutStep: replay.scoutStep,
            gap: shots.gap,
        }),
        realNow,
        // For tests: the kept frame nearest `ms`.
        frameAt: (ms) => {
            const i = shotIndex(ms);
            const near = [shots.list[i - 1], shots.list[i]].filter(Boolean);
            near.sort((a, b) => Math.abs(a.ms - ms) - Math.abs(b.ms - ms));
            return near[0] ?? null;
        },
        start(client) {
            replay.client = client;
            if (document.readyState === 'loading') {
                document.addEventListener('DOMContentLoaded', makeOverlay);
            } else {
                makeOverlay();
            }
            maybeLogin();
        },
    };
    // Whole-pixel scales stay sharp; anything else is smoothed rather than
    // dropping or doubling uneven rows of pixels.
    function fitPixels() {
        const c = canvas();
        if (!c) {
            return;
        }
        const scale = (c.getBoundingClientRect().width * devicePixelRatio) / c.width;
        const sharp = scale >= 1 && Math.abs(scale - Math.round(scale)) < 0.02;
        for (const el of [c, curtain, overlay]) {
            if (el) {
                el.style.imageRendering = sharp ? 'pixelated' : 'auto';
            }
        }
    }
    window.addEventListener('resize', fitPixels);

    // A fallback for a client that never shows it's done loading.
    realSetTimeout(clientReady, 20000);
    const ready = () => {
        const game = canvas();
        post({ type: 'ready', width: game?.width ?? 765, height: game?.height ?? 503 });
    };
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', ready);
    } else {
        ready();
    }
})();
