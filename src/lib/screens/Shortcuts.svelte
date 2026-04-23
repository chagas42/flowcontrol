<!-- screens/Shortcuts.svelte -->
<script lang="ts">
  import Kbd from '../Kbd.svelte';

  const shortcuts = [
    { id: 'force-local',  label: 'Force cursor local',     keys: ['⌃', '⌥', 'F'], desc: 'Pull the cursor back to this machine immediately.' },
    { id: 'pause',        label: 'Pause sharing',          keys: ['⌃', '⌥', 'P'], desc: 'Temporarily stop handing off without disconnecting.' },
    { id: 'switch-peer',  label: 'Cycle to next peer',     keys: ['⌃', '⌥', 'S'], desc: 'When more than two devices are paired.' },
    { id: 'open-window',  label: 'Open FlowControl',       keys: ['⌃', '⌥', 'O'], desc: 'Show this window from anywhere.' },
    { id: 'quit',         label: 'Quit FlowControl',       keys: ['⌃', '⌥', 'Q'], desc: 'Stop the daemon and exit.' },
  ];
</script>

<section class="page">
  <header>
    <div class="title">Shortcuts</div>
    <div class="sub">Global hotkeys work anywhere, even when FlowControl isn't focused.</div>
  </header>

  <div class="list">
    {#each shortcuts as s}
      <div class="row">
        <div class="body">
          <div class="label">{s.label}</div>
          <div class="desc">{s.desc}</div>
        </div>
        <div class="keys">
          {#each s.keys as k, i}
            <Kbd>{k}</Kbd>
            {#if i < s.keys.length - 1}<span class="plus">+</span>{/if}
          {/each}
        </div>
      </div>
    {/each}
  </div>

  <footer class="note">Shortcut customization lands in a future release.</footer>
</section>

<style>
  .page { padding: 24px 30px; display: flex; flex-direction: column; gap: 14px; flex: 1; overflow-y: auto; }
  .title { font-size: 22px; font-weight: 700; letter-spacing: -0.4px; color: var(--ink); }
  .sub { font-size: 13px; color: var(--ink-2); margin-top: 2px; }
  .list { display: flex; flex-direction: column; gap: 1px; background: var(--line); border-radius: var(--radius-md); overflow: hidden; border: 1px solid var(--line); }
  .row {
    padding: 14px 16px; background: var(--paper);
    display: flex; align-items: center; gap: 16px;
  }
  .body { flex: 1; }
  .label { font-size: 13px; font-weight: 500; color: var(--ink); }
  .desc  { font-size: 11.5px; color: var(--ink-3); margin-top: 2px; }
  .keys  { display: flex; align-items: center; gap: 4px; }
  .plus  { font-size: 10px; color: var(--ink-3); }
  .note  { font-size: 11.5px; color: var(--ink-3); text-align: center; padding-top: 8px; }
</style>
