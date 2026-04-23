<!-- toast/ToastLayer.svelte
     Mount once at App root. Stacks toasts bottom-right.
     In App.svelte onMount: listen('cursor-crossed', (e) => pushToast({ title: `Cursor crossed to ${e.payload}` }));
-->
<script lang="ts">
  import { toasts, dismissToast } from './toastStore.svelte';
  import { fly, fade } from 'svelte/transition';
</script>

<div class="layer">
  {#each $toasts as t (t.id)}
    <div
      class="toast tone-{t.tone}"
      role="status"
      in:fly={{ y: 20, duration: 220 }}
      out:fade={{ duration: 180 }}
      on:click={() => dismissToast(t.id)}
    >
      <div class="dot"></div>
      <div class="body">
        <div class="title">{t.title}</div>
        {#if t.desc}<div class="desc">{t.desc}</div>{/if}
      </div>
    </div>
  {/each}
</div>

<style>
  .layer {
    position: fixed; bottom: 20px; right: 20px; z-index: 900;
    display: flex; flex-direction: column; gap: 8px; pointer-events: none;
  }
  .toast {
    pointer-events: auto; cursor: pointer;
    padding: 10px 14px; border-radius: var(--radius-md);
    background: var(--paper); border: 1px solid var(--line);
    box-shadow: var(--shadow-md);
    display: flex; align-items: center; gap: 10px;
    min-width: 240px; max-width: 340px;
  }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--coral); flex-shrink: 0; box-shadow: 0 0 6px var(--coral); }
  .tone-success .dot { background: oklch(0.7 0.14 150); box-shadow: 0 0 6px oklch(0.7 0.14 150); }
  .tone-warn .dot { background: #d97706; box-shadow: 0 0 6px #d97706; }
  .title { font-size: 12.5px; font-weight: 600; color: var(--ink); }
  .desc  { font-size: 11px; color: var(--ink-2); margin-top: 1px; }
</style>
