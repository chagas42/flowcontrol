<!-- PeerCard.svelte -->
<script lang="ts">
  import OSBadge from './OSBadge.svelte';
  import StatusDot from './StatusDot.svelte';

  export let name: string;
  export let sub: string;
  export let os: 'mac' | 'win' = 'mac';
  export let primary = false;
  export let active = false;
  export let latency: number | null = null;
</script>

<div class="card" class:active>
  <OSBadge {os} size={28}/>
  <div class="body">
    <div class="row">
      <span class="name">{name}</span>
      {#if primary}<span class="pill">Primary</span>{/if}
    </div>
    <div class="sub mono">{sub}</div>
  </div>
  <div class="right">
    <div class="row">
      <StatusDot color={active ? 'var(--coral)' : '#22c55e'} pulse={active} size={7}/>
      <span class="state mono">{active ? 'Active' : 'Ready'}</span>
    </div>
    {#if latency != null}<div class="lat mono">{latency}ms</div>{/if}
  </div>
</div>

<style>
  .card {
    padding: 14px 16px; border-radius: var(--radius-md);
    background: var(--paper); border: 1px solid var(--line);
    display: flex; align-items: center; gap: 12px;
    transition: background .15s, border-color .15s;
  }
  .card.active {
    background: var(--coral-soft); border-color: var(--coral);
  }
  .body { flex: 1; min-width: 0; }
  .row { display: flex; align-items: center; gap: 7px; }
  .name { font-size: 13.5px; font-weight: 600; color: var(--ink); }
  .pill {
    font-size: 9.5px; font-weight: 600; padding: 2px 6px; border-radius: 999px;
    background: rgba(0,0,0,0.06); color: var(--ink-3);
    letter-spacing: 0.4px; text-transform: uppercase;
  }
  .sub   { font-size: 10.5px; color: var(--ink-3); margin-top: 2px; }
  .right { text-align: right; display: flex; flex-direction: column; align-items: flex-end; gap: 2px; }
  .state { font-size: 11px; color: var(--ink); font-weight: 500; }
  .lat   { font-size: 10px; color: var(--ink-3); }
</style>
