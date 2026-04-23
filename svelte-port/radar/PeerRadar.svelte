<!-- radar/PeerRadar.svelte
     Concentric pulsing rings with peers plotted on a radial grid.
     Consumes the live `peers` prop (already flowing from listen('peers-updated')).
-->
<script lang="ts">
  import OSBadge from '../OSBadge.svelte';
  import { createEventDispatcher } from 'svelte';

  export let peers: Array<{ id: string; name: string; os?: 'mac' | 'win' }> = [];
  export let selfName = 'This Mac';

  const dispatch = createEventDispatcher();

  // Deterministic angle per peer id so UI is stable between re-renders
  function angle(id: string, i: number): number {
    let h = 0;
    for (let k = 0; k < id.length; k++) h = (h * 31 + id.charCodeAt(k)) >>> 0;
    return ((h % 360) + i * 47) % 360;
  }
  // Ring index based on peer order (nearest = first found)
  function ring(i: number): number {
    return Math.min(i, 2);  // 0, 1, 2 = inner, mid, outer
  }
</script>

<div class="radar-wrap">
  <svg viewBox="-150 -150 300 300" class="radar">
    <!-- Sweeping conic gradient simulated with rotated gradient -->
    <defs>
      <radialGradient id="sweep">
        <stop offset="0" stop-color="var(--coral)" stop-opacity="0.0"/>
        <stop offset="1" stop-color="var(--coral)" stop-opacity="0.12"/>
      </radialGradient>
    </defs>

    <!-- Pulse rings -->
    {#each [0, 1, 2] as n}
      <circle cx="0" cy="0" r="40" fill="none"
              stroke="var(--coral)" stroke-width="1" opacity="0.35"
              class="pulse" style="animation-delay: {n * 0.9}s"/>
    {/each}

    <!-- Static grid rings -->
    <circle cx="0" cy="0" r="45" fill="none" stroke="var(--line)" stroke-width="1" stroke-dasharray="2 4"/>
    <circle cx="0" cy="0" r="85" fill="none" stroke="var(--line)" stroke-width="1" stroke-dasharray="2 4"/>
    <circle cx="0" cy="0" r="125" fill="none" stroke="var(--line)" stroke-width="1" stroke-dasharray="2 4"/>

    <!-- Center (self) -->
    <g>
      <circle cx="0" cy="0" r="22" fill="var(--paper)" stroke="var(--coral)" stroke-width="2"/>
      <circle cx="0" cy="0" r="6" fill="var(--coral)"/>
    </g>
  </svg>

  <!-- Peer markers as HTML so we can mount OSBadge + click -->
  <div class="peers">
    {#each peers as p, i (p.id)}
      {@const a = angle(p.id, i) * Math.PI / 180}
      {@const r = 45 + ring(i) * 40}
      {@const x = Math.cos(a) * r}
      {@const y = Math.sin(a) * r}
      <button
        class="peer"
        style="transform: translate({x}px, {y}px);"
        on:click={() => dispatch('select', p)}
        title={p.name}
      >
        <div class="peer-inner">
          <OSBadge os={p.os ?? 'win'} size={24}/>
        </div>
        <span class="peer-label mono">{p.name}</span>
      </button>
    {/each}
  </div>

  <!-- Self label -->
  <div class="self-label mono">{selfName}</div>

  <!-- Status copy -->
  <div class="copy">
    {#if peers.length === 0}
      <div class="headline">Scanning the network…</div>
      <div class="subline">Make sure FlowControl is running on the other device.</div>
    {:else}
      <div class="headline">{peers.length} device{peers.length === 1 ? '' : 's'} found</div>
      <div class="subline">Click a device to request pairing.</div>
    {/if}
  </div>
</div>

<style>
  .radar-wrap {
    position: relative; width: 100%; aspect-ratio: 1;
    max-width: 340px; margin: 0 auto;
    display: flex; align-items: center; justify-content: center;
  }
  .radar { width: 100%; height: 100%; overflow: visible; }

  .pulse {
    transform-origin: center;
    animation: radarPulse 2.7s ease-out infinite;
  }
  @keyframes radarPulse {
    0%   { r: 6;   opacity: 0.6; }
    100% { r: 140; opacity: 0;   }
  }

  .peers {
    position: absolute; inset: 0;
    display: flex; align-items: center; justify-content: center;
    pointer-events: none;
  }
  .peer {
    position: absolute; pointer-events: auto;
    background: none; border: none; cursor: pointer; padding: 0;
    display: flex; flex-direction: column; align-items: center; gap: 4px;
    animation: peerIn .4s cubic-bezier(.2,.8,.3,1.2) both;
  }
  @keyframes peerIn {
    from { opacity: 0; transform: translate(var(--x, 0), var(--y, 0)) scale(0.5); }
    to   { opacity: 1; }
  }
  .peer-inner {
    padding: 6px; border-radius: 10px; background: var(--paper);
    border: 1.5px solid var(--coral);
    box-shadow: 0 0 0 4px rgba(201,100,66,0.12), var(--shadow-sm);
    transition: transform .15s, box-shadow .15s;
  }
  .peer:hover .peer-inner {
    transform: scale(1.08);
    box-shadow: 0 0 0 6px rgba(201,100,66,0.2), var(--shadow-md);
  }
  .peer-label {
    font-size: 10px; color: var(--ink-2); white-space: nowrap;
    background: var(--paper); padding: 1px 6px; border-radius: 4px;
    border: 1px solid var(--line);
  }

  .self-label {
    position: absolute; top: 50%; left: 50%;
    transform: translate(-50%, 28px);
    font-size: 10px; color: var(--ink-3); white-space: nowrap;
    pointer-events: none;
  }

  .copy { position: absolute; bottom: -68px; left: 0; right: 0; text-align: center; }
  .headline { font-size: 14px; font-weight: 600; color: var(--ink); }
  .subline  { font-size: 12px; color: var(--ink-2); margin-top: 3px; }
</style>
