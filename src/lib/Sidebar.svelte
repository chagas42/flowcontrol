<!-- Sidebar.svelte -->
<script lang="ts">
  import NavIcon from './NavIcon.svelte';
  import StatusDot from './StatusDot.svelte';

  export let active: 'devices' | 'arrange' | 'shortcuts' | 'advanced' | 'about' = 'devices';
  export let status: 'Connected' | 'Remote' | 'Searching' | 'Disconnected' | 'Paused' | 'Stopped' = 'Connected';

  const items = [
    { id: 'devices',   label: 'Devices',      icon: 'grid' },
    { id: 'arrange',   label: 'Arrangement',  icon: 'layout' },
    { id: 'shortcuts', label: 'Shortcuts',    icon: 'keys' },
    { id: 'advanced',  label: 'Advanced',     icon: 'cog' },
    { id: 'about',     label: 'About',        icon: 'info' },
  ] as const;

  $: dotColor = {
    Connected:    'var(--coral)',
    Remote:       'var(--cool)',
    Searching:    '#a68f6d',
    Disconnected: '#b34d3e',
    Paused:       '#d97706',
    Stopped:      '#9e9a93',
  }[status];
</script>

<aside class="sidebar">
  <div class="brand">
    <div class="logo"></div>
    <div class="name">FlowControl</div>
  </div>
  <nav>
    {#each items as it}
      <button class="item" class:active={active === it.id} on:click={() => active = it.id}>
        <NavIcon name={it.icon} active={active === it.id}/>
        <span>{it.label}</span>
      </button>
    {/each}
  </nav>
  <div class="spacer"></div>
  <div class="status-card">
    <div class="mono label">STATUS</div>
    <div class="row">
      <StatusDot color={dotColor} pulse={status === 'Searching'} size={7}/>
      <span>{status}</span>
    </div>
  </div>
</aside>

<style>
  .sidebar {
    width: 200px; background: var(--paper-2);
    border-right: 1px solid var(--line);
    padding: 14px 10px; display: flex; flex-direction: column; gap: 2px;
    box-sizing: border-box;
  }
  .brand {
    display: flex; align-items: center; gap: 8px;
    padding: 6px 10px 12px;
  }
  .logo {
    width: 22px; height: 22px; border-radius: 6px;
    background: linear-gradient(135deg, var(--coral), var(--coral-deep));
    box-shadow: inset 0 1px 0 rgba(255,255,255,0.2);
  }
  .name { font-size: 12.5px; font-weight: 700; letter-spacing: -0.1px; color: var(--ink); }

  nav { display: flex; flex-direction: column; gap: 2px; }
  .item {
    display: flex; align-items: center; gap: 8px;
    padding: 7px 10px; border-radius: 7px; cursor: pointer;
    font-size: 12.5px; font-weight: 500; color: var(--ink-2);
    background: transparent; border: none; width: 100%; text-align: left;
    transition: background .12s;
  }
  .item:hover { background: rgba(0,0,0,0.03); }
  .item.active { color: var(--ink); background: rgba(0,0,0,0.05); }

  .spacer { flex: 1; }

  .status-card {
    padding: 10px 12px; border-radius: 8px;
    background: var(--paper); border: 1px solid var(--line);
    font-size: 11px; color: var(--ink-2);
    display: flex; flex-direction: column; gap: 3px;
  }
  .status-card .label { font-size: 10px; letter-spacing: 0.5px; color: var(--ink-3); }
  .status-card .row { display: flex; align-items: center; gap: 6px; color: var(--ink); font-weight: 500; }
</style>
