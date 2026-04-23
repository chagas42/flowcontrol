<!-- pause/PauseBanner.svelte
     Shows when status === 'Paused'. Sits above StatusStrip.
     Consumes:   invoke('resume_sharing')
-->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  export let peerName = 'Studio PC';

  async function resume() {
    await invoke('resume_sharing');
  }
</script>

<div class="banner">
  <div class="icon" aria-hidden="true">
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <rect x="3" y="2" width="2.5" height="10" rx="1" fill="currentColor"/>
      <rect x="8.5" y="2" width="2.5" height="10" rx="1" fill="currentColor"/>
    </svg>
  </div>
  <div class="body">
    <div class="title">Sharing paused</div>
    <div class="desc">{peerName} is still connected. Resume to hand off the cursor again.</div>
  </div>
  <button class="btn solid" on:click={resume}>Resume</button>
</div>

<style>
  .banner {
    padding: 12px 16px; border-radius: var(--radius-md);
    background: oklch(0.96 0.03 80); border: 1px solid oklch(0.82 0.06 80);
    display: flex; align-items: center; gap: 12px;
  }
  .icon {
    width: 32px; height: 32px; border-radius: 8px;
    background: oklch(0.88 0.08 80); color: oklch(0.42 0.12 70);
    display: grid; place-items: center; flex-shrink: 0;
  }
  .body { flex: 1; }
  .title { font-size: 13px; font-weight: 600; color: var(--ink); }
  .desc  { font-size: 11.5px; color: var(--ink-2); margin-top: 1px; }
  .btn {
    padding: 7px 14px; border-radius: 999px; border: none; cursor: pointer;
    font-family: inherit; font-size: 12.5px; font-weight: 600;
    background: linear-gradient(180deg, var(--coral), var(--coral-deep)); color: #fff;
  }
</style>
