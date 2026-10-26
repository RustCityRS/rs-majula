// GPU scaling for the game's page. Loaded before the client, it adds GPU
// options to the page's scaling dropdown; picked, the game is drawn again by
// WebGPU, scaled by a shader, onto a canvas laid over the client's own.
//
// The client is left as it is: it keeps drawing into its 2D canvas, which
// keeps the mouse and keys (the GPU canvas lets them through) and still makes
// the screenshots. Each frame is copied from it into a texture on the GPU, so
// nothing here reads the pixels back. Without WebGPU the options never show.
(() => {
    'use strict';

    const MODES = [
        ['sharp', 'GPU Sharp'],
        ['smooth', 'GPU Smooth'],
        ['crt', 'GPU CRT'],
    ];
    const MODE_IDS = { sharp: 0, smooth: 1, crt: 2 };
    const STORE = 'gpuFilter';

    const SHADER = /* wgsl */ `
struct View {
    src: vec2f,
    dst: vec2f,
    mode: u32,
}

@group(0) @binding(0) var frame: texture_2d<f32>;
@group(0) @binding(1) var pixels: sampler;
@group(0) @binding(2) var<uniform> view: View;

struct Out {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
}

// One triangle over the whole canvas.
@vertex
fn vs(@builtin(vertex_index) i: u32) -> Out {
    let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
    var out: Out;
    out.pos = vec4f(p * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2f(p.x, 1.0 - p.y);
    return out;
}

// Square pixels at any scale: each source pixel is a flat block, blended
// only across the one screen pixel where two meet, so none come out wider
// than their neighbours (as nearest does) and none blur (as bilinear does).
fn sharp(uv: vec2f) -> vec3f {
    let texel = uv * view.src;
    let scale = view.dst / view.src;
    let edge = max(vec2f(0.0), 0.5 - 0.5 / scale);
    let off = fract(texel) - 0.5;
    let f = (off - clamp(off, -edge, edge)) * scale + 0.5;
    return textureSampleLevel(frame, pixels, (floor(texel) + f) / view.src, 0.0).rgb;
}

// Catmull-Rom: smooth, but keeping edges (nine bilinear taps).
fn catmull_rom(uv: vec2f) -> vec3f {
    let pos = uv * view.src;
    let c = floor(pos - 0.5) + 0.5;
    let f = pos - c;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    let w12 = w1 + w2;
    let p0 = (c - 1.0) / view.src;
    let p3 = (c + 2.0) / view.src;
    let p12 = (c + w2 / w12) / view.src;
    var rgb = vec3f(0.0);
    rgb += textureSampleLevel(frame, pixels, vec2f(p0.x, p0.y), 0.0).rgb * w0.x * w0.y;
    rgb += textureSampleLevel(frame, pixels, vec2f(p12.x, p0.y), 0.0).rgb * w12.x * w0.y;
    rgb += textureSampleLevel(frame, pixels, vec2f(p3.x, p0.y), 0.0).rgb * w3.x * w0.y;
    rgb += textureSampleLevel(frame, pixels, vec2f(p0.x, p12.y), 0.0).rgb * w0.x * w12.y;
    rgb += textureSampleLevel(frame, pixels, vec2f(p12.x, p12.y), 0.0).rgb * w12.x * w12.y;
    rgb += textureSampleLevel(frame, pixels, vec2f(p3.x, p12.y), 0.0).rgb * w3.x * w12.y;
    rgb += textureSampleLevel(frame, pixels, vec2f(p0.x, p3.y), 0.0).rgb * w0.x * w3.y;
    rgb += textureSampleLevel(frame, pixels, vec2f(p12.x, p3.y), 0.0).rgb * w12.x * w3.y;
    rgb += textureSampleLevel(frame, pixels, vec2f(p3.x, p3.y), 0.0).rgb * w3.x * w3.y;
    return clamp(rgb, vec3f(0.0), vec3f(1.0));
}

// Sharp pixels as an old monitor showed them: a dark gap under each row, a
// faint aperture grille, a little brighter to make up for both, and dimmer
// at the corners. The gap is at the bottom of a row, not around its middle,
// so at 2x (two screen rows a row) the second one is the dark one. It fades
// out towards 1x, where every screen row is a whole game row.
fn crt(uv: vec2f, at: vec2f) -> vec3f {
    var rgb = sharp(uv);
    let scale = view.dst.y / view.src.y;
    let gap = smoothstep(1.1, 1.6, scale) * 0.5;
    rgb *= 1.0 - gap * smoothstep(0.45, 1.0, fract(uv.y * view.src.y));
    var grille = array<vec3f, 3>(vec3f(1.0, 0.9, 0.9), vec3f(0.9, 1.0, 0.9), vec3f(0.9, 0.9, 1.0));
    rgb *= grille[u32(at.x) % 3u];
    let d = uv - 0.5;
    rgb *= 1.0 - dot(d, d) * 0.35;
    return min(rgb * 1.25, vec3f(1.0));
}

@fragment
fn fs(in: Out) -> @location(0) vec4f {
    var rgb: vec3f;
    switch view.mode {
        case 1u: { rgb = catmull_rom(in.uv); }
        case 2u: { rgb = crt(in.uv, in.pos.xy); }
        default: { rgb = sharp(in.uv); }
    }
    return vec4f(rgb, 1.0);
}
`;

    const source = document.getElementById('canvas');
    const select = document.getElementById('filtering');
    if (!source || !select || !navigator.gpu) {
        return;
    }

    const gpu = {
        device: null,
        context: null,
        pipeline: null,
        sampler: null,
        uniforms: null,
        texture: null,
        bind: null,
        mode: 'off',
        frame: 0,
    };
    let stored = 'off';
    try {
        stored = localStorage.getItem(STORE) || 'off';
    } catch { /* storage blocked */ }
    if (!(stored in MODE_IDS)) {
        stored = 'off';
    }

    // Over the client's canvas, the same size, letting the mouse through.
    const layer = document.createElement('canvas');
    layer.id = 'gpu-canvas';
    layer.setAttribute('aria-hidden', 'true');
    const style = document.createElement('style');
    style.textContent = `
        #game { position: relative; }
        /* Above the client's canvas: the page gives every canvas z-index -1. */
        #gpu-canvas { position: absolute; z-index: 1; display: none; pointer-events: none; touch-action: none; }
        #game:fullscreen { display: flex; align-items: center; justify-content: center; background: #000; width: 100vw; height: 100vh; }
        #game:fullscreen #canvas {
            width: min(100vw, calc(100vh * 765 / 503)) !important; height: min(100vh, calc(100vw * 503 / 765)) !important;
            max-width: none !important; max-height: none !important;
        }
    `;
    document.head.append(style);
    source.after(layer);

    function store(mode) {
        try {
            localStorage.setItem(STORE, mode);
        } catch { /* storage blocked */ }
    }

    function frameTexture() {
        const { width, height } = source;
        if (gpu.texture && gpu.texture.width === width && gpu.texture.height === height) {
            return;
        }
        gpu.texture?.destroy();
        gpu.texture = gpu.device.createTexture({
            size: [width, height],
            format: 'rgba8unorm',
            usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT,
        });
        gpu.bind = gpu.device.createBindGroup({
            layout: gpu.pipeline.getBindGroupLayout(0),
            entries: [
                { binding: 0, resource: gpu.texture.createView() },
                { binding: 1, resource: gpu.sampler },
                { binding: 2, resource: { buffer: gpu.uniforms } },
            ],
        });
    }

    // The screen pixels the layer covers, as the browser lays it out: a box
    // starting part way into a pixel (a centred one, scaled 150%) is snapped
    // a pixel wider or narrower than its size times the scale, and a canvas
    // with any other number of pixels would be stretched over it.
    const covers = { width: 0, height: 0 };
    try {
        new ResizeObserver(([entry]) => {
            const size = entry.devicePixelContentBoxSize?.[0];
            if (size) {
                covers.width = size.inlineSize;
                covers.height = size.blockSize;
            }
        }).observe(layer, { box: 'device-pixel-content-box' });
    } catch { /* not told: worked out from the scale below */ }

    // Kept on the client's canvas wherever the page puts and sizes it: the
    // same box exactly (from the exact boxes, not offsetLeft, which rounds),
    // so the browser snaps the two to the same screen pixels.
    const px = (n) => `${n}px`;
    function place() {
        const box = source.getBoundingClientRect();
        const parent = source.offsetParent;
        if (!box.width || !box.height || !parent) {
            return false;
        }
        const origin = parent.getBoundingClientRect();
        const left = px(box.left - origin.left - parent.clientLeft);
        const top = px(box.top - origin.top - parent.clientTop);
        if (layer.style.left !== left || layer.style.top !== top) {
            layer.style.left = left;
            layer.style.top = top;
        }
        if (layer.style.width !== px(box.width) || layer.style.height !== px(box.height)) {
            layer.style.width = px(box.width);
            layer.style.height = px(box.height);
        }
        // What the browser says it covers, when that's the snapping and not
        // some other scale (an emulated one goes unreported).
        const dpr = window.devicePixelRatio || 1;
        const near = (told, size) => (Math.abs(told - size) <= 2 ? told : Math.max(1, Math.round(size)));
        const w = near(covers.width, box.width * dpr);
        const h = near(covers.height, box.height * dpr);
        if (layer.width !== w || layer.height !== h) {
            layer.width = w;
            layer.height = h;
        }
        return true;
    }

    function draw() {
        gpu.frame = 0;
        if (gpu.mode === 'off' || !gpu.device) {
            return;
        }
        gpu.frame = requestAnimationFrame(draw);
        if (document.hidden || !place()) {
            return;
        }
        frameTexture();
        const { device } = gpu;
        device.queue.copyExternalImageToTexture({ source }, { texture: gpu.texture }, [source.width, source.height]);
        const view = new ArrayBuffer(32);
        new Float32Array(view, 0, 4).set([source.width, source.height, layer.width, layer.height]);
        new Uint32Array(view, 16, 1)[0] = MODE_IDS[gpu.mode];
        device.queue.writeBuffer(gpu.uniforms, 0, view);
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({
            colorAttachments: [{
                view: gpu.context.getCurrentTexture().createView(),
                loadOp: 'clear',
                storeOp: 'store',
                clearValue: [0, 0, 0, 1],
            }],
        });
        pass.setPipeline(gpu.pipeline);
        pass.setBindGroup(0, gpu.bind);
        pass.draw(3);
        pass.end();
        device.queue.submit([encoder.finish()]);
    }

    function apply(mode) {
        gpu.mode = gpu.device ? mode : 'off';
        layer.style.display = gpu.mode === 'off' ? 'none' : 'block';
        if (gpu.mode !== 'off' && !gpu.frame) {
            gpu.frame = requestAnimationFrame(draw);
        }
    }

    // The dropdown's own choices keep working as they did; a GPU one is
    // remembered apart from them (the page keeps only pixel or auto).
    const pageSetFilter = window.setFilter;
    window.setFilter = (type) => {
        if (type === 'dropdown') {
            const picked = select.value.startsWith('gpu-') ? select.value.slice(4) : 'off';
            stored = picked in MODE_IDS ? picked : 'off';
            store(stored);
            if (stored !== 'off') {
                apply(stored);
                return;
            }
        }
        pageSetFilter?.(type);
        if (gpu.device && stored !== 'off') {
            select.value = `gpu-${stored}`;
        }
        apply(gpu.device ? stored : 'off');
    };

    // Fullscreen takes the canvas alone, which would leave this layer out:
    // with the GPU on, the game's box goes fullscreen instead.
    const pageToggleFullscreen = window.toggleFullscreen;
    window.toggleFullscreen = () => {
        const game = document.getElementById('game');
        if (gpu.mode !== 'off' && !document.fullscreenElement && game?.requestFullscreen) {
            game.requestFullscreen();
        } else {
            pageToggleFullscreen?.();
        }
    };

    async function start() {
        const adapter = await navigator.gpu.requestAdapter();
        if (!adapter) {
            return;
        }
        const device = await adapter.requestDevice();
        const context = layer.getContext('webgpu');
        if (!context) {
            return;
        }
        const format = navigator.gpu.getPreferredCanvasFormat();
        context.configure({ device, format, alphaMode: 'opaque' });
        const module = device.createShaderModule({ code: SHADER });
        gpu.pipeline = await device.createRenderPipelineAsync({
            layout: 'auto',
            vertex: { module, entryPoint: 'vs' },
            fragment: { module, entryPoint: 'fs', targets: [{ format }] },
            primitive: { topology: 'triangle-list' },
        });
        gpu.sampler = device.createSampler({ magFilter: 'linear', minFilter: 'linear' });
        gpu.uniforms = device.createBuffer({ size: 32, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
        gpu.context = context;
        gpu.device = device;
        // A lost GPU (a driver reset) falls back to the client's own canvas.
        device.lost.then(() => {
            gpu.device = null;
            gpu.texture = null;
            apply('off');
        });

        const group = document.createElement('optgroup');
        group.label = 'GPU';
        for (const [mode, label] of MODES) {
            group.append(new Option(label, `gpu-${mode}`));
        }
        select.append(group);
        if (stored !== 'off') {
            select.value = `gpu-${stored}`;
            apply(stored);
        }
    }

    start().catch((e) => console.warn('GPU scaling unavailable:', e));
})();
