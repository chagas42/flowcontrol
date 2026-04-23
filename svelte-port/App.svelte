<!-- App.svelte — Final: full wire-up with pair modal, toasts, pause, radar, screen routing -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import ArrangeDisplays from './lib/ArrangeDisplays.svelte';
  import Sidebar from './lib/Sidebar.svelte';
  import StatusStrip from './lib/StatusStrip.svelte';
  import PeerCard from './lib/PeerCard.svelte';
  import PeerCardSkeleton from './lib/PeerCardSkeleton.svelte';
  import EmptyState from './lib/EmptyState.svelte';
  import PeerRadar from './lib/radar/PeerRadar.svelte';
  import PairRequestDialog from './lib/pair/PairRequestDialog.svelte';
  import PauseBanner from './lib/pause/PauseBanner.svelte';
  import ToastLayer from './lib/toast/ToastLayer.svelte';
  import { pushToast } from './lib/toast/toastStore.svelte';
  import Welcome from './lib/onboarding/Welcome.svelte';
  import Permissions from './lib/onboarding/Permissions.svelte';
  import Shortcuts from './lib/screens/Shortcuts.svelte';
  import About from './lib/screens/About.svelte';
  import Advanced from './lib/screens/Advanced.svelte';

  type Status = 'Connected' | 'Remote' | 'Searching' | 'Disconnected' | 'Paused' | 'Stopped';
  type Nav = 'devices' | 'arrange' | 'shortcuts' | 'advanced' | 'about';

  let status: Status = 'Stopped';
  let mode: 'Server' | 'Client' = 'Server';
  let peers: Array<{ id: string; name: string; os?: 'mac' | 'win' }> = [];
  let activeSide: 'Left' | 'Right' | 'Top' | 'Bottom' = 'Right';
  let nav: Nav = 'devices';

  // Pair request state
  let pairPending: null | { peer_name: string; fingerprint: string; os: 'mac' | 'win' } = null;

  let step: 'welcome' | 'permissions' | 'main' =
    localStorage.getItem('fc_onboarded') ? 'main' : 'welcome';

  onMount(() => {
    const un1 = listen('peers-updated',   (e) => (peers = e.payload as any));
    const un2 = listen('status-changed',  (e) => (status = e.payload as any));
    const un3 = listen('pair-incoming',   (e) => (pairPending = e.payload as any));
    const un4 = listen('pair-resolved',   () => (pairPending = null));
    const un5 = listen('cursor-crossed',  (e: any) => {
      const { direction, peer_name } = e.payload;
      pushToast({
        title: direction === 'to_remote'
          ? `Cursor crossed to ${peer_name}`
          : 'Cursor back on this Mac',
      });
    });
    const un6 = listen('tray-action', (e: any) => {
      if (e.payload === 'pause')  invoke('pause_sharing');
      if (e.payload === 'about')  nav = 'about';
    });
    return () => { [un1, un2, un3, un4, un5, un6].forEach(p => p.then(f => f())); };
  });

  function finishOnboarding() {
    localStorage.setItem('fc_onboarded', 'true');
    step = 'main';
  }

  async function handleLayoutChanged(e: CustomEvent<{ side: typeof activeSide }>) {
    activeSide = e.detail.side;
    if (status !== 'Stopped') {
      await invoke('stop_coordinator');
      await (mode === 'Server' ? startServer() : startClient());
    }
  }

  async function startServer() {
    status = 'Searching';
    try {
      await invoke('start_server', {
        name: 'My Mac',
        width: window.screen.width, height: window.screen.height, side: activeSide,
      });
    } catch { status = 'Stopped'; }
  }
  async function startClient() {
    status = 'Searching';
    try {
      await invoke('start_client', {
        width: window.screen.width, height: window.screen.height, side: activeSide,
      });
    } catch { status = 'Stopped'; }
  }
  async function stop() { await invoke('stop_coordinator'); status = 'Stopped'; peers = []; }

  async function selectRadarPeer(e: CustomEvent<{ id: string; name: string }>) {
    try { await invoke('connect_to_peer', { peerId: e.detail.id }); } catch {}
  }

  $: showEmpty = nav === 'devices' && status === 'Disconnected' && peers.length === 0
                 && !localStorage.getItem('fc_had_peer');
  $: showRadar = nav === 'devices' && status === 'Searching' && peers.length === 0;
  $: if (peers.length > 0) localStorage.setItem('fc_had_peer', '1');
</script>

{#if step === 'welcome'}
  <Welcome on:continue={() => (step = 'permissions')}/>
{:else if step === 'permissions'}
  <Permissions on:continue={finishOnboarding} on:back={() => (step = 'welcome')}/>
{:else}
  <main class="layout">
    <Sidebar bind:active={nav} {status}/>

    {#if nav === 'devices' || nav === 'arrange'}
      <section class="content">
        <header>
          <div class="title">{nav === 'arrange' ? 'Arrangement' : 'Devices'}</div>
          <div class="count mono">{peers.length} {peers.length === 1 ? 'peer' : 'peers'}</div>
        </header>
        <p class="subtitle">
          {nav === 'arrange'
            ? 'Position each display where it physically sits on your desk.'
            : 'Devices on your network, ready to share a mouse.'}
        </p>

        {#if status === 'Paused'}
          <PauseBanner peerName={peers[0]?.name ?? 'peer'}/>
        {:else}
          <StatusStrip {status} peerName={peers[0]?.name ?? 'Studio PC'}/>
        {/if}

        {#if showRadar}
          <div class="radar-slot">
            <PeerRadar {peers} selfName="This Mac" on:select={selectRadarPeer}/>
          </div>
        {:else if showEmpty}
          <EmptyState on:start={mode === 'Server' ? startServer : startClient}/>
        {:else}
          <ArrangeDisplays on:layoutChanged={handleLayoutChanged}/>

          <div class="peer-grid">
            <PeerCard
              name="This Mac"
              sub="Marta's MBP · 192.168.1.17"
              os="mac"
              primary
              active={status === 'Connected'}/>
            {#if peers.length > 0}
              <PeerCard
                name={peers[0].name}
                sub="Windows 11 · paired"
                os={peers[0].os ?? 'win'}
                active={status === 'Remote'}
                latency={status === 'Connected' || status === 'Remote' ? 4 : null}/>
            {:else if status === 'Searching'}
              <PeerCardSkeleton/>
            {:else}
              <PeerCard name="Waiting for peer" sub="Run FlowControl on another device" os="win"/>
            {/if}
          </div>
        {/if}

        <footer class="actions">
          {#if status === 'Stopped'}
            <button class="btn solid" on:click={mode === 'Server' ? startServer : startClient}>
              {mode === 'Server' ? 'Start server' : 'Find pair'}
            </button>
          {:else}
            <button class="btn ghost" on:click={stop}>Stop</button>
          {/if}
        </footer>
      </section>
    {:else if nav === 'shortcuts'}
      <Shortcuts/>
    {:else if nav === 'about'}
      <About/>
    {:else if nav === 'advanced'}
      <Advanced/>
    {/if}
  </main>

  {#if pairPending}
    <PairRequestDialog
      peerName={pairPending.peer_name}
      fingerprint={pairPending.fingerprint}
      os={pairPending.os}
      on:accepted={() => (pairPending = null)}
      on:declined={() => (pairPending = null)}/>
  {/if}

  <ToastLayer/>
{/if}

<style>
  .layout { display: flex; width: 100%; height: 100vh; background: var(--paper); }
  .content { flex: 1; display: flex; flex-direction: column; padding: 24px 30px; gap: 14px; overflow-y: auto; }
  header { display: flex; align-items: baseline; justify-content: space-between; }
  .title { font-size: 22px; font-weight: 700; letter-spacing: -0.4px; }
  .count { font-size: 11px; color: var(--ink-3); }
  .subtitle { font-size: 13px; color: var(--ink-2); margin-top: -8px; }
  .radar-slot { padding: 32px 20px 80px; }
  .peer-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .actions { display: flex; justify-content: flex-end; margin-top: auto; }
  .btn { padding: 8px 18px; border-radius: 999px; font-size: 13px; font-weight: 600; cursor: pointer; border: none; }
  .btn.solid { background: linear-gradient(180deg, var(--coral), var(--coral-deep)); color: #fff; box-shadow: 0 1px 2px rgba(0,0,0,0.15), inset 0 0.5px 0 rgba(255,255,255,0.3); }
  .btn.ghost { background: rgba(0,0,0,0.05); color: var(--ink); }
</style>
