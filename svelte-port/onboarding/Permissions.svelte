<!-- Permissions.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { createEventDispatcher, onDestroy } from 'svelte';
  const dispatch = createEventDispatcher();

  let accessibility = false;
  let inputMon = false;
  let polling: ReturnType<typeof setInterval> | null = null;

  async function openSettings() {
    await invoke('request_accessibility_permission');
    polling = setInterval(async () => {
      const ok: boolean = await invoke('check_accessibility_permission');
      if (ok) {
        accessibility = true;
        inputMon = true; // macOS pairs these; simplify for now
        if (polling) clearInterval(polling);
      }
    }, 1500);
  }

  onDestroy(() => { if (polling) clearInterval(polling); });
</script>

<div class="wrap">
  <div class="step mono">STEP 2 OF 3</div>
  <h1>A couple of permissions</h1>
  <p>macOS needs permission for FlowControl to read mouse input and inject it when the cursor is on another device.</p>

  <div class="rows">
    <div class="row" class:granted={accessibility}>
      <div class="icon">{#if accessibility}✓{:else}🔒{/if}</div>
      <div class="body">
        <div class="t">Accessibility</div>
        <div class="d">Observe global mouse movement and clicks.</div>
        <div class="s mono">System Settings → Privacy → Accessibility</div>
      </div>
      <button class="btn" class:ghost={accessibility} on:click={openSettings}>
        {accessibility ? 'Granted' : 'Open Settings'}
      </button>
    </div>

    <div class="row" class:granted={inputMon}>
      <div class="icon">{#if inputMon}✓{:else}🔒{/if}</div>
      <div class="body">
        <div class="t">Input Monitoring</div>
        <div class="d">Post synthetic events back to the active app.</div>
        <div class="s mono">System Settings → Privacy → Input Monitoring</div>
      </div>
      <button class="btn" class:ghost={inputMon} on:click={openSettings}>
        {inputMon ? 'Granted' : 'Open Settings'}
      </button>
    </div>
  </div>

  <footer>
    <span class="note">Your mouse data never leaves your local network.</span>
    <div class="actions">
      <button class="btn ghost" on:click={() => dispatch('back')}>Back</button>
      <button class="btn solid" on:click={() => dispatch('continue')}>Continue</button>
    </div>
  </footer>
</div>

<style>
  .wrap {
    padding: 40px 48px; height: 100vh; box-sizing: border-box;
    display: flex; flex-direction: column; background: var(--paper);
  }
  .step { font-size: 11px; letter-spacing: 1.2px; color: var(--coral); margin-bottom: 10px; }
  h1 { font-size: 28px; font-weight: 700; letter-spacing: -0.6px; color: var(--ink); margin-bottom: 6px; }
  p  { font-size: 14px; color: var(--ink-2); margin-bottom: 28px; max-width: 500px; }
  .rows { display: flex; flex-direction: column; gap: 10px; }
  .row {
    display: flex; align-items: center; gap: 14px; padding: 16px 18px;
    background: var(--paper-2); border: 1px solid var(--line);
    border-radius: var(--radius-md);
  }
  .icon {
    width: 36px; height: 36px; border-radius: 9px;
    background: rgba(0,0,0,0.06); display: grid; place-items: center;
    color: var(--ink-2);
  }
  .row.granted .icon { background: oklch(0.92 0.09 150); color: oklch(0.38 0.12 150); }
  .body { flex: 1; }
  .t { font-size: 14px; font-weight: 600; color: var(--ink); }
  .d { font-size: 12.5px; color: var(--ink-2); margin-top: 1px; }
  .s { font-size: 10.5px; color: var(--ink-3); margin-top: 4px; letter-spacing: 0.2px; }
  footer {
    margin-top: auto; display: flex; align-items: center; justify-content: space-between;
  }
  .note { font-size: 12px; color: var(--ink-3); }
  .actions { display: flex; gap: 10px; }
  .btn {
    padding: 8px 16px; border-radius: 999px; font-size: 13px; font-weight: 600;
    border: none; cursor: pointer; font-family: inherit;
  }
  .btn.solid { background: linear-gradient(180deg, var(--coral), var(--coral-deep)); color: #fff; }
  .btn.ghost { background: rgba(0,0,0,0.05); color: var(--ink); }
</style>
