<!-- NavIcon.svelte -->
<script lang="ts">
  export let name: 'grid' | 'layout' | 'keys' | 'cog' | 'info';
  export let active = false;
  $: c = active ? 'var(--ink)' : 'var(--ink-3)';
</script>

<svg width="14" height="14" viewBox="0 0 16 16" fill="none">
  {#if name === 'grid'}
    <rect x="2" y="2" width="5" height="5" rx="1" stroke={c} stroke-width="1.5"/>
    <rect x="9" y="2" width="5" height="5" rx="1" stroke={c} stroke-width="1.5"/>
    <rect x="2" y="9" width="5" height="5" rx="1" stroke={c} stroke-width="1.5"/>
    <rect x="9" y="9" width="5" height="5" rx="1" stroke={c} stroke-width="1.5"/>
  {:else if name === 'layout'}
    <rect x="2" y="3" width="5" height="10" rx="1" stroke={c} stroke-width="1.5"/>
    <rect x="9" y="3" width="5" height="10" rx="1" stroke={c} stroke-width="1.5"/>
  {:else if name === 'keys'}
    <rect x="2" y="5" width="12" height="7" rx="1.5" stroke={c} stroke-width="1.5"/>
    <path d="M5 9 h6" stroke={c} stroke-width="1.5" stroke-linecap="round"/>
  {:else if name === 'cog'}
    <circle cx="8" cy="8" r="2" stroke={c} stroke-width="1.5"/>
    <path d="M8 2 v2 M8 12 v2 M2 8 h2 M12 8 h2" stroke={c} stroke-width="1.5" stroke-linecap="round"/>
  {:else if name === 'info'}
    <circle cx="8" cy="8" r="6" stroke={c} stroke-width="1.5"/>
    <path d="M8 7 v4 M8 5 v0.1" stroke={c} stroke-width="1.5" stroke-linecap="round"/>
  {/if}
</svg>
