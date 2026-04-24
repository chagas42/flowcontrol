<!--
  ArrangeDisplays.svelte — v3

  Drag the secondary card to the side where the peer physically sits.
  Dropping anywhere snaps to the NEAREST edge (no dead zone where the
  drop is silently lost).

  Accepts props so the card labels & OS icons reflect the real peer.
-->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let side: 'Left' | 'Right' | 'Top' | 'Bottom' = 'Right';
  export let localName: string = 'This Mac';
  export let localOs: 'mac' | 'win' = 'mac';
  export let remoteName: string = 'Peer';
  export let remoteOs: 'mac' | 'win' = 'mac';

  const dispatch = createEventDispatcher<{ layoutChanged: { side: 'Left' | 'Right' | 'Top' | 'Bottom' } }>();

  // Canvas sized to fit all 4 snap positions without clipping.
  const CANVAS_W = 640;
  const CANVAS_H = 360;
  const LOCAL = { w: 180, h: 110 };
  const REMOTE = { w: 160, h: 100 };

  // Center LOCAL so every snap position (Left / Right / Top / Bottom) is
  // fully visible inside the canvas.
  const localOrigin = {
    x: (CANVAS_W - LOCAL.w) / 2,
    y: (CANVAS_H - LOCAL.h) / 2,
  };

  // Secondary position relative to the local top-left.
  // `posForSide` keeps the initial position in sync with the `side` prop.
  function posForSide(s: 'Left' | 'Right' | 'Top' | 'Bottom') {
    switch (s) {
      case 'Right':  return { x: LOCAL.w, y: (LOCAL.h - REMOTE.h) / 2 };
      case 'Left':   return { x: -REMOTE.w, y: (LOCAL.h - REMOTE.h) / 2 };
      case 'Bottom': return { x: (LOCAL.w - REMOTE.w) / 2, y: LOCAL.h };
      case 'Top':    return { x: (LOCAL.w - REMOTE.w) / 2, y: -REMOTE.h };
    }
  }
  let pos = posForSide(side);
  // Re-seed when parent flips `side` externally (e.g. on a restart).
  $: pos = posForSide(side);

  let dragging = false;
  let dragStart: { mx: number; my: number; px: number; py: number } | null = null;
  let snapEdge: 'Left' | 'Right' | 'Top' | 'Bottom' = side;
  $: snapEdge = nearestEdge(pos);

  /**
   * Always returns the single edge this position is *closest to*.
   * No threshold, no null — drop anywhere on the canvas and the secondary
   * commits to one of the 4 sides.
   */
  function nearestEdge(p: { x: number; y: number }): 'Left' | 'Right' | 'Top' | 'Bottom' {
    const cx = p.x + REMOTE.w / 2;      // secondary center x, relative to LOCAL origin
    const cy = p.y + REMOTE.h / 2;
    const lcx = LOCAL.w / 2;            // LOCAL center
    const lcy = LOCAL.h / 2;
    const dx = cx - lcx;                // vector from LOCAL to secondary
    const dy = cy - lcy;
    if (Math.abs(dx) >= Math.abs(dy)) {
      return dx >= 0 ? 'Right' : 'Left';
    } else {
      return dy >= 0 ? 'Bottom' : 'Top';
    }
  }

  function snapPos(edge: 'Left' | 'Right' | 'Top' | 'Bottom', fallback: { x: number; y: number }) {
    const base = posForSide(edge);
    // Preserve the perpendicular axis from the drop so the handoff line
    // lines up where the user aimed.
    switch (edge) {
      case 'Right':
      case 'Left':
        return { x: base.x, y: clamp(fallback.y, -REMOTE.h + 20, LOCAL.h - 20) };
      case 'Top':
      case 'Bottom':
        return { x: clamp(fallback.x, -REMOTE.w + 20, LOCAL.w - 20), y: base.y };
    }
  }

  function onPointerDown(e: PointerEvent) {
    dragging = true;
    dragStart = { mx: e.clientX, my: e.clientY, px: pos.x, py: pos.y };
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging || !dragStart) return;
    pos = {
      x: dragStart.px + (e.clientX - dragStart.mx),
      y: dragStart.py + (e.clientY - dragStart.my),
    };
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);

    const edge = nearestEdge(pos);
    pos = snapPos(edge, pos);
    if (edge !== side) {
      side = edge;
    }
    dispatch('layoutChanged', { side: edge });
  }

  function clamp(v: number, lo: number, hi: number) { return Math.min(hi, Math.max(lo, v)); }

  $: travelKf = buildTravel(snapEdge, pos);
  function buildTravel(edge: 'Left' | 'Right' | 'Top' | 'Bottom', p: { x: number; y: number }) {
    if (edge === 'Right') return {
      from: { left: 30, top: LOCAL.h / 2 },
      to:   { left: p.x + 40, top: p.y + REMOTE.h / 2 },
    };
    if (edge === 'Left') return {
      from: { left: LOCAL.w - 30, top: LOCAL.h / 2 },
      to:   { left: p.x + REMOTE.w - 40, top: p.y + REMOTE.h / 2 },
    };
    if (edge === 'Bottom') return {
      from: { left: LOCAL.w / 2, top: 20 },
      to:   { left: p.x + REMOTE.w / 2, top: p.y + 30 },
    };
    return {
      from: { left: LOCAL.w / 2, top: LOCAL.h - 20 },
      to:   { left: p.x + REMOTE.w / 2, top: p.y + REMOTE.h - 20 },
    };
  }
</script>

<div class="arrange" on:pointerdown={onPointerDown} on:pointermove={onPointerMove} on:pointerup={onPointerUp}>
  <div class="header">
    <p>Drag the other display to match its physical position relative to this one.</p>
  </div>

  <div class="canvas" style="width: {CANVAS_W}px; height: {CANVAS_H}px;">
    <div class="origin" style="left: {localOrigin.x}px; top: {localOrigin.y}px;">
      <!-- Primary (anchored) -->
      <div class="display primary"
           class:edge-right={snapEdge === 'Right'} class:edge-left={snapEdge === 'Left'}
           class:edge-top={snapEdge === 'Top'} class:edge-bottom={snapEdge === 'Bottom'}
           style="width: {LOCAL.w}px; height: {LOCAL.h}px;">
        <div class="chrome" class:mac={localOs === 'mac'}></div>
        <div class="label">
          <div class="osbadge" class:mac={localOs === 'mac'} class:win={localOs === 'win'}>
            {#if localOs === 'mac'}
              <svg width="14" height="14" viewBox="0 0 16 16" fill="#fff"><path d="M10.8 2.3c-.6.7-1.5 1.2-2.4 1.1-.1-.9.3-1.8.8-2.4.6-.7 1.6-1.2 2.3-1.2.1.9-.2 1.8-.7 2.5zM11.5 4c-1.3-.1-2.4.7-3 .7-.7 0-1.6-.7-2.7-.7-1.4 0-2.7.8-3.4 2.1-1.5 2.5-.4 6.3 1 8.3.7 1 1.6 2.1 2.7 2.1 1.1 0 1.5-.7 2.8-.7s1.6.7 2.8.7c1.1 0 1.9-1 2.6-2 .8-1.1 1.2-2.3 1.2-2.3s-2.3-.9-2.3-3.5c0-2.2 1.8-3.2 1.9-3.3-1-1.5-2.6-1.7-3.2-1.8z"/></svg>
            {:else}
              <div class="wingrid"><span></span><span></span><span></span><span></span></div>
            {/if}
          </div>
          <div class="name">{localName}</div>
          <div class="pill">Primary</div>
        </div>
      </div>

      <!-- Secondary (draggable) -->
      <div class="display secondary" class:dragging
           style="left: {pos.x}px; top: {pos.y}px; width: {REMOTE.w}px; height: {REMOTE.h}px;">
        <div class="chrome" class:mac={remoteOs === 'mac'}></div>
        <div class="label">
          <div class="osbadge" class:mac={remoteOs === 'mac'} class:win={remoteOs === 'win'}>
            {#if remoteOs === 'mac'}
              <svg width="14" height="14" viewBox="0 0 16 16" fill="#fff"><path d="M10.8 2.3c-.6.7-1.5 1.2-2.4 1.1-.1-.9.3-1.8.8-2.4.6-.7 1.6-1.2 2.3-1.2.1.9-.2 1.8-.7 2.5zM11.5 4c-1.3-.1-2.4.7-3 .7-.7 0-1.6-.7-2.7-.7-1.4 0-2.7.8-3.4 2.1-1.5 2.5-.4 6.3 1 8.3.7 1 1.6 2.1 2.7 2.1 1.1 0 1.5-.7 2.8-.7s1.6.7 2.8.7c1.1 0 1.9-1 2.6-2 .8-1.1 1.2-2.3 1.2-2.3s-2.3-.9-2.3-3.5c0-2.2 1.8-3.2 1.9-3.3-1-1.5-2.6-1.7-3.2-1.8z"/></svg>
            {:else}
              <div class="wingrid"><span></span><span></span><span></span><span></span></div>
            {/if}
          </div>
          <div class="name">{remoteName}</div>
        </div>
      </div>

      <!-- Handoff line -->
      {#if snapEdge === 'Right'}
        <div class="handoff" style="left: {LOCAL.w - 1}px; top: {Math.max(0, pos.y)}px; width: 2px; height: {Math.max(4, Math.min(LOCAL.h, pos.y + REMOTE.h) - Math.max(0, pos.y))}px;"></div>
      {:else if snapEdge === 'Left'}
        <div class="handoff" style="left: -1px; top: {Math.max(0, pos.y)}px; width: 2px; height: {Math.max(4, Math.min(LOCAL.h, pos.y + REMOTE.h) - Math.max(0, pos.y))}px;"></div>
      {:else if snapEdge === 'Bottom'}
        <div class="handoff" style="top: {LOCAL.h - 1}px; left: {Math.max(0, pos.x)}px; height: 2px; width: {Math.max(4, Math.min(LOCAL.w, pos.x + REMOTE.w) - Math.max(0, pos.x))}px;"></div>
      {:else}
        <div class="handoff" style="top: -1px; left: {Math.max(0, pos.x)}px; height: 2px; width: {Math.max(4, Math.min(LOCAL.w, pos.x + REMOTE.w) - Math.max(0, pos.x))}px;"></div>
      {/if}

      <!-- Traveling cursor preview -->
      <div class="cursor" style="--from-l: {travelKf.from.left}px; --from-t: {travelKf.from.top}px; --to-l: {travelKf.to.left}px; --to-t: {travelKf.to.top}px;">
        <svg width="18" height="25" viewBox="0 0 16 22">
          <path d="M1.5 1.2 L1.5 16.8 L5.2 13.5 L7.8 19.5 L10.2 18.5 L7.6 12.5 L12.5 12.5 Z"
                fill="#1a1a1a" stroke="#fff" stroke-width="1.2" stroke-linejoin="round"/>
        </svg>
      </div>
    </div>

    <div class="hint mono">SNAP TO NEAREST EDGE · DRAG TO ARRANGE</div>
    <div class="side-label mono">Peer on your <b>{snapEdge.toUpperCase()}</b></div>
  </div>
</div>

<style>
  .arrange { padding: 20px 32px; display: flex; flex-direction: column; flex: 1; }
  .header p { font-size: 13px; color: var(--ink-2, #555); margin-bottom: 14px; }

  .canvas {
    position: relative; overflow: hidden;
    border-radius: var(--radius-lg, 14px);
    background: var(--paper-2, #f6f4ef);
    border: 1px solid var(--line, #e3dfd6);
    background-image: radial-gradient(circle at 0 0, rgba(0,0,0,0.06) 1px, transparent 1px);
    background-size: 20px 20px;
    background-position: 4px 4px;
    margin: 0 auto;
  }

  .origin { position: absolute; }

  .display {
    position: absolute;
    border-radius: 10px;
    background: linear-gradient(180deg, #fbfaf7 0%, #eeece6 100%);
    border: 1px solid rgba(0,0,0,0.08);
    box-shadow: 0 1px 2px rgba(0,0,0,0.06);
    overflow: hidden;
    user-select: none;
  }
  .display.secondary { cursor: grab; transition: box-shadow .12s; }
  .display.secondary.dragging { cursor: grabbing; box-shadow: 0 12px 30px rgba(0,0,0,0.2); transition: none; }

  .chrome { height: 16px; background: rgba(0,0,0,0.04); border-bottom: 1px solid rgba(0,0,0,0.04); }
  .chrome:not(.mac) {
    height: auto; position: absolute; left: 0; right: 0; bottom: 0; top: auto;
    height: 18px; border-top: 1px solid rgba(0,0,0,0.06); border-bottom: none;
    background: rgba(255,255,255,0.85);
  }

  .label {
    position: absolute; inset: 0;
    display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 4px;
  }
  .name { font-size: 12px; font-weight: 600; letter-spacing: -0.1px; color: #2a2724; }
  .mono { font-family: 'JetBrains Mono', ui-monospace, monospace; }
  .pill {
    font-size: 9px; font-weight: 600; padding: 2px 6px; border-radius: 999px;
    background: rgba(0,0,0,0.06); color: rgba(0,0,0,0.55);
    text-transform: uppercase; letter-spacing: 0.5px;
  }

  .osbadge {
    width: 22px; height: 22px; border-radius: 5px;
    display: grid; place-items: center; color: #fff;
  }
  .osbadge.mac { background: #1e1e22; }
  .osbadge.win { background: #0078d4; }
  .wingrid {
    display: grid; grid-template-columns: 1fr 1fr; gap: 2px; width: 13px; height: 13px;
  }
  .wingrid span { background: #fff; border-radius: 1px; }

  .display.primary { position: relative; }
  .display.primary::after {
    content: ''; position: absolute; pointer-events: none;
    background: var(--coral, #e07a5f); box-shadow: 0 0 14px var(--coral, #e07a5f);
    opacity: 0; transition: opacity .15s;
  }
  .display.primary.edge-right::after  { opacity: 1; right: 0; top: 0; bottom: 0; width: 3px; }
  .display.primary.edge-left::after   { opacity: 1; left: 0; top: 0; bottom: 0; width: 3px; }
  .display.primary.edge-top::after    { opacity: 1; top: 0; left: 0; right: 0; height: 3px; }
  .display.primary.edge-bottom::after { opacity: 1; bottom: 0; left: 0; right: 0; height: 3px; }

  .handoff {
    position: absolute;
    background: var(--coral, #e07a5f);
    box-shadow: 0 0 12px var(--coral, #e07a5f);
    border-radius: 1px;
    pointer-events: none;
  }

  .cursor {
    position: absolute; pointer-events: none;
    animation: travel 4.5s cubic-bezier(.5,.05,.5,.95) infinite;
    filter: drop-shadow(0 1px 2px rgba(0,0,0,0.3));
  }
  @keyframes travel {
    0%, 40%   { left: var(--from-l); top: var(--from-t); }
    50%, 90%  { left: var(--to-l);   top: var(--to-t); }
    100%      { left: var(--from-l); top: var(--from-t); }
  }

  .hint {
    position: absolute; right: 12px; bottom: 10px;
    font-size: 10.5px; letter-spacing: 0.3px;
    color: rgba(0,0,0,0.4);
  }
  .side-label {
    position: absolute; left: 12px; bottom: 10px;
    font-size: 11px; letter-spacing: 0.3px;
    color: rgba(0,0,0,0.55);
  }
</style>
