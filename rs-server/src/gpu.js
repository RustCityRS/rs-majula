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
        ['smooth', 'GPU Smooth'],
        ['sharp', 'GPU Sharp'],
        ['crt', 'GPU CRT'],
    ];
    const MODE_IDS = {sharp: 0, smooth: 1, crt: 2};
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
@group(0) @binding(3) var glow: texture_2d<f32>;

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

// The game as a 2004 PC monitor showed it, the applet at 1:1, one scanline a
// game row. The light is worked out in linear terms, from the gamma the tube
// was driven at, and given back at the screen's.
const CRT_GAMMA = 2.4;
const SCREEN_GAMMA = 2.2;
// How dark the other two stripes of the grille are under each one.
const MASK_DARK = 0.6;

// A game pixel's light; none above or below the picture.
fn crt_texel(x: i32, y: i32) -> vec3f {
    let size = vec2i(view.src);
    if (y < 0 || y >= size.y) {
        return vec3f(0.0);
    }
    let rgb = textureLoad(frame, vec2i(clamp(x, 0, size.x - 1), y), 0).rgb;
    return pow(rgb, vec3f(CRT_GAMMA));
}

// A scanline's light at x (in game pixels): the beam can't change at once,
// so each step between pixels is spread over about 0.4 of one.
fn crt_scanline(x: f32, row: i32) -> vec3f {
    let left = floor(x - 0.5);
    let f = clamp((x - 1.0 - left) * 2.5 + 0.5, 0.0, 1.0);
    return mix(crt_texel(i32(left), row), crt_texel(i32(left) + 1, row), f);
}

fn crt_erf(x: vec3f) -> vec3f {
    let x2 = x * x;
    return sign(x) * sqrt(1.0 - exp(-x2 * (1.2732395 + 0.147 * x2) / (1.0 + 0.147 * x2)));
}

// How much of a scanline lands on a screen pixel d rows from its middle and
// h rows high either side. The spot is a Gaussian, wider the brighter it is
// (so dark rows show gaps between them and bright ones run together), taken
// over the whole screen pixel, so a scale that puts screen rows between
// scanlines doesn't beat against them. It keeps all its light: a flat colour
// comes out as bright as it went in.
fn crt_beam(d: f32, h: f32, rgb: vec3f) -> vec3f {
    let sigma = mix(vec3f(0.20), vec3f(0.36), sqrt(rgb));
    let s = 1.0 / (sigma * 1.4142135);
    return (crt_erf((d + h) * s) - crt_erf((d - h) * s)) / (4.0 * h);
}

// An aperture grille (a Trinitron's): red, green and blue stripes, one screen
// pixel each, brightened back to the light they take. Below about 2x a stripe
// is wide next to a game pixel and only tints it, so it fades in from there.
fn crt_grille(at: vec2f, scale: f32) -> vec3f {
    var stripes = array<vec3f, 3>(
        vec3f(1.0, MASK_DARK, MASK_DARK),
        vec3f(MASK_DARK, 1.0, MASK_DARK),
        vec3f(MASK_DARK, MASK_DARK, 1.0),
    );
    let strength = smoothstep(1.5, 2.5, scale);
    let mask = mix(vec3f(1.0), stripes[u32(at.x) % 3u], strength);
    return mask / mix(1.0, (1.0 + 2.0 * MASK_DARK) / 3.0, strength);
}

// The three scanlines nearest the pixel, red and blue landing a little either
// side of green (more towards the edges, as the beams converge worse there),
// through the grille, a touch dimmer away from the middle of the tube. The
// picture isn't curved: the mouse goes to the client's flat canvas beneath.
fn crt(uv: vec2f, at: vec2f) -> vec3f {
    let pos = uv * view.src;
    let scale = view.dst / view.src;
    let h = 0.5 / scale.y;
    // At an even scale a row's middle falls between two screen rows, which a
    // beam there would light the same; up half a screen row, it's on one.
    let middle = 0.5 - select(0.0, h, (i32(round(scale.y)) & 1) == 0);
    let spread = 0.1 + 0.15 * abs(uv.x * 2.0 - 1.0);
    let nearest = i32(floor(pos.y));
    var light = vec3f(0.0);
    for (var i = -1; i <= 1; i++) {
        let row = nearest + i;
        let rgb = vec3f(
            crt_scanline(pos.x + spread, row).r,
            crt_scanline(pos.x, row).g,
            crt_scanline(pos.x - spread, row).b,
        );
        light += rgb * crt_beam(pos.y - (f32(row) + middle), h, rgb);
    }
    light *= crt_grille(at, scale.x);
    // The glass's glow is over the grille, not through it.
    light = mix(light, textureSampleLevel(glow, pixels, uv, 0.0).rgb, HALATION);
    let d = uv - 0.5;
    light *= 1.0 - dot(d, d) * 0.2;
    return pow(min(light, vec3f(1.0)), vec3f(1.0 / SCREEN_GAMMA));
}

// Halation: some of the phosphors' light scatters in the tube's glass and
// comes out around them, a soft glow off anything bright. The share that
// scatters is taken from the picture and given back as the glow, so it's no
// brighter overall. The glow is the game's light blurred at half its size,
// across and then down (a pass each, into textures crt reads as glow).
const HALATION = 0.12;
// In half-size pixels: about six game pixels.
const GLOW_SIGMA = 3.0;
const GLOW_TAPS = 8;

fn glow_blur(uv: vec2f, step: vec2f, decode: bool) -> vec4f {
    var sum = vec3f(0.0);
    var total = 0.0;
    for (var i = -GLOW_TAPS; i <= GLOW_TAPS; i++) {
        let w = exp(-0.5 * f32(i * i) / (GLOW_SIGMA * GLOW_SIGMA));
        var rgb = textureSampleLevel(frame, pixels, uv + step * f32(i), 0.0).rgb;
        if (decode) {
            rgb = pow(rgb, vec3f(CRT_GAMMA));
        }
        sum += rgb * w;
        total += w;
    }
    return vec4f(sum / total, 1.0);
}

// Across, from the game's frame: each half-size pixel sits between four game
// pixels, which the sampler averages.
@fragment
fn fs_glow_x(in: Out) -> @location(0) vec4f {
    return glow_blur(in.uv, vec2f(2.0 / f32(textureDimensions(frame).x), 0.0), true);
}

// Down, from the pass across (bound as frame), already linear.
@fragment
fn fs_glow_y(in: Out) -> @location(0) vec4f {
    return glow_blur(in.uv, vec2f(0.0, 1.0 / f32(textureDimensions(frame).y)), false);
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
        glowXPipeline: null,
        glowYPipeline: null,
        glowX: null,
        glowY: null,
        glowXBind: null,
        glowYBind: null,
        mode: 'off',
        frame: 0,
    };
    let stored = 'off';
    try {
        stored = localStorage.getItem(STORE) || 'off';
    } catch { /* storage blocked */
    }
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
        #game:fullscreen #canvas.stretch { width: 100vw !important; height: 100vh !important; }
    `;
    document.head.append(style);
    source.after(layer);

    function store(mode) {
        try {
            localStorage.setItem(STORE, mode);
        } catch { /* storage blocked */
        }
    }

    function frameTexture() {
        const {width, height} = source;
        if (gpu.texture && gpu.texture.width === width && gpu.texture.height === height) {
            return;
        }
        const {device} = gpu;
        gpu.texture?.destroy();
        gpu.glowX?.destroy();
        gpu.glowY?.destroy();
        gpu.texture = device.createTexture({
            size: [width, height],
            format: 'rgba8unorm',
            usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT,
        });
        const glow = () => device.createTexture({
            size: [(width + 1) >> 1, (height + 1) >> 1],
            format: 'rgba16float',
            usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.RENDER_ATTACHMENT,
        });
        gpu.glowX = glow();
        gpu.glowY = glow();
        gpu.bind = device.createBindGroup({
            layout: gpu.pipeline.getBindGroupLayout(0),
            entries: [
                {binding: 0, resource: gpu.texture.createView()},
                {binding: 1, resource: gpu.sampler},
                {binding: 2, resource: {buffer: gpu.uniforms}},
                {binding: 3, resource: gpu.glowY.createView()},
            ],
        });
        const blurBind = (pipeline, from) => device.createBindGroup({
            layout: pipeline.getBindGroupLayout(0),
            entries: [
                {binding: 0, resource: from.createView()},
                {binding: 1, resource: gpu.sampler},
            ],
        });
        gpu.glowXBind = blurBind(gpu.glowXPipeline, gpu.texture);
        gpu.glowYBind = blurBind(gpu.glowYPipeline, gpu.glowX);
    }

    function drawPass(encoder, target, pipeline, bind) {
        const pass = encoder.beginRenderPass({
            colorAttachments: [{
                view: target.createView(),
                loadOp: 'clear',
                storeOp: 'store',
                clearValue: [0, 0, 0, 1],
            }],
        });
        pass.setPipeline(pipeline);
        pass.setBindGroup(0, bind);
        pass.draw(3);
        pass.end();
    }

    // The screen pixels the layer covers, as the browser lays it out: a box
    // starting part way into a pixel (a centred one, scaled 150%) is snapped
    // a pixel wider or narrower than its size times the scale, and a canvas
    // with any other number of pixels would be stretched over it.
    const covers = {width: 0, height: 0};
    try {
        new ResizeObserver(([entry]) => {
            const size = entry.devicePixelContentBoxSize?.[0];
            if (size) {
                covers.width = size.inlineSize;
                covers.height = size.blockSize;
            }
        }).observe(layer, {box: 'device-pixel-content-box'});
    } catch { /* not told: worked out from the scale below */
    }

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
        const {device} = gpu;
        device.queue.copyExternalImageToTexture({source}, {texture: gpu.texture}, [source.width, source.height]);
        const view = new ArrayBuffer(32);
        new Float32Array(view, 0, 4).set([source.width, source.height, layer.width, layer.height]);
        new Uint32Array(view, 16, 1)[0] = MODE_IDS[gpu.mode];
        device.queue.writeBuffer(gpu.uniforms, 0, view);
        const encoder = device.createCommandEncoder();
        if (gpu.mode === 'crt') {
            drawPass(encoder, gpu.glowX, gpu.glowXPipeline, gpu.glowXBind);
            drawPass(encoder, gpu.glowY, gpu.glowYPipeline, gpu.glowYBind);
        }
        drawPass(encoder, gpu.context.getCurrentTexture(), gpu.pipeline, gpu.bind);
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
        context.configure({device, format, alphaMode: 'opaque'});
        const module = device.createShaderModule({code: SHADER});
        const pipeline = (entryPoint, target) => device.createRenderPipelineAsync({
            layout: 'auto',
            vertex: {module, entryPoint: 'vs'},
            fragment: {module, entryPoint, targets: [{format: target}]},
            primitive: {topology: 'triangle-list'},
        });
        [gpu.pipeline, gpu.glowXPipeline, gpu.glowYPipeline] = await Promise.all([
            pipeline('fs', format),
            pipeline('fs_glow_x', 'rgba16float'),
            pipeline('fs_glow_y', 'rgba16float'),
        ]);
        gpu.sampler = device.createSampler({magFilter: 'linear', minFilter: 'linear'});
        gpu.uniforms = device.createBuffer({size: 32, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST});
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
