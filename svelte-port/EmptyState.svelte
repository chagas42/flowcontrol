<!-- EmptyState.svelte — shown when never paired yet -->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher();
</script>

<div class="empty">
  <div class="illo" aria-hidden="true">
    <svg width="72" height="72" viewBox="0 0 72 72" fill="none">
      <rect x="6" y="18" width="28" height="20" rx="3" stroke="var(--coral)" stroke-width="1.5" fill="var(--coral-soft)"/>
      <rect x="38" y="14" width="28" height="24" rx="3" stroke="var(--ink-3)" stroke-width="1.5" stroke-dasharray="3 3"/>
      <path d="M34 28 H38" stroke="var(--coral)" stroke-width="2" stroke-dasharray="2 2"/>
      <circle cx="20" cy="50" r="2" fill="var(--coral)"/>
      <circle cx="52" cy="50" r="2" fill="var(--ink-3)"/>
      <path d="M20 50 H52" stroke="var(--line)" stroke-width="1" stroke-dasharray="2 3"/>
    </svg>
  </div>
  <div class="title">No peers yet</div>
  <div class="desc">Install FlowControl on another device and make sure both are on the same Wi-Fi.</div>
  <button class="btn" on:click={() => dispatch('start')}>Start looking</button>
</div>

<style>
  .empty {
    padding: 48px 24px; text-align: center;
    background: var(--paper-2); border: 1px dashed var(--line-2);
    border-radius: var(--radius-md);
    display: flex; flex-direction: column; align-items: center; gap: 8px;
  }
  .illo { margin-bottom: 6px; }
  .title { font-size: 15px; font-weight: 600; color: var(--ink); }
  .desc  { font-size: 12.5px; color: var(--ink-2); max-width: 320px; line-height: 1.45; }
  .btn {
    margin-top: 10px; padding: 7px 16px; border-radius: 999px;
    background: linear-gradient(180deg, var(--coral), var(--coral-deep)); color: #fff;
    border: none; cursor: pointer; font-size: 12.5px; font-weight: 600; font-family: inherit;
  }
</style>
