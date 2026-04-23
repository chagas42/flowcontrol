<!-- StatusStrip.svelte -->
<script lang="ts">
  import Kbd from './Kbd.svelte';
  export let status: 'Connected' | 'Remote' | 'Searching' | 'Disconnected' | 'PairRequest' | 'Paused' | 'Stopped' = 'Connected';
  export let peerName = 'Studio PC';

  $: blurb = {
    Connected:    { title: 'Cursor is here', desc: `You're driving this machine. Move past the configured edge to cross over.`, color: 'var(--coral)' },
    Remote:       { title: `Cursor is on ${peerName}`, desc: 'Keyboard and mouse are driving the remote machine.', color: 'var(--cool)' },
    Searching:    { title: 'Looking for peers…', desc: 'Scanning the local network over mDNS.', color: '#a68f6d' },
    Disconnected: { title: 'Not connected', desc: `${peerName} is offline or on a different network.`, color: '#b34d3e' },
    PairRequest:  { title: `Pair request from ${peerName}`, desc: 'Approve the request below to start sharing a mouse.', color: 'var(--coral)' },
    Paused:       { title: 'Sharing paused', desc: 'Resume to let the cursor cross again.', color: '#d97706' },
    Stopped:      { title: 'Idle', desc: 'Start the server or find a pair to begin.', color: '#9e9a93' },
  }[status];
</script>

<div class="strip">
  <div class="icon" style="background:{blurb.color}22; color:{blurb.color}">
    <div class="d" style="background:{blurb.color}; box-shadow: 0 0 10px {blurb.color}"></div>
  </div>
  <div class="body">
    <div class="title">{blurb.title}</div>
    <div class="desc">{blurb.desc}</div>
  </div>
  <div class="actions">
    {#if status === 'PairRequest'}
      <slot name="pair-actions"/>
    {:else}
      <div class="kbd-row mono">
        <Kbd>⌃</Kbd><Kbd>⌥</Kbd><Kbd>F</Kbd>
        <span class="hint">force local</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .strip {
    padding: 14px 18px; border-radius: var(--radius-md);
    background: var(--paper-2); border: 1px solid var(--line);
    display: flex; align-items: center; gap: 14px;
  }
  .icon {
    width: 38px; height: 38px; border-radius: var(--radius-md);
    display: grid; place-items: center;
  }
  .d { width: 10px; height: 10px; border-radius: 50%; }
  .body { flex: 1; }
  .title { font-size: 14px; font-weight: 600; color: var(--ink); }
  .desc  { font-size: 12px; color: var(--ink-2); }
  .actions { display: flex; align-items: center; gap: 6px; }
  .kbd-row { display: flex; align-items: center; gap: 6px; font-size: 11px; }
  .hint { color: var(--ink-3); margin-left: 4px; }
</style>
