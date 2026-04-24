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
  // Radar only makes sense for guests (clients) — the host doesn't browse.
  $: showRadar = nav === 'devices' && status === 'Searching' && mode === 'Client' && peers.length === 0;
  $: showHostWaiting = nav === 'devices' && status === 'Searching' && mode === 'Server';
  $: if (peers.length > 0) localStorage.setItem('fc_had_peer', '1');
</script>

{#if step === 'welcome'}
  <Welcome on:continue={() => (step = 'permissions')}/>
{:else if step === 'permissions'}
  <Permissions on:continue={finishOnboarding} on:back={() => (step = 'welcome')}/>
{:else}
  <main class="layout">
    <Sidebar bind:active={nav} {status}/>

    {#if nav === 'arrange'}
      <section class="content">
        <header>
          <div class="title">Arrangement</div>
          <div class="count mono">side: {activeSide.toUpperCase()}</div>
        </header>
        <p class="subtitle">
          Position the other display where it physically sits relative to this Mac.
          Drop anywhere — it snaps to the nearest edge. Configure this on each Mac so the sides are <em>opposite</em>.
        </p>

        <ArrangeDisplays
          bind:side={activeSide}
          localName="This Mac"
          localOs="mac"
          remoteName={peers[0]?.name ?? 'Other Mac'}
          remoteOs={peers[0]?.os ?? 'mac'}
          on:layoutChanged={handleLayoutChanged}/>

        <footer class="actions">
          {#if status !== 'Stopped'}
            <div class="mode-pill">{mode === 'Server' ? 'Host' : 'Guest'}</div>
          {/if}
        </footer>
      </section>
    {:else if nav === 'devices'}
      <section class="content">
        <header>
          <div class="title">Devices</div>
          <div class="count mono">{peers.length} {peers.length === 1 ? 'peer' : 'peers'}</div>
        </header>
        <p class="subtitle">Devices on your network, ready to share a mouse.</p>

        {#if status === 'Paused'}
          <PauseBanner peerName={peers[0]?.name ?? 'peer'}/>
        {:else}
          <StatusStrip {status} peerName={peers[0]?.name ?? 'Studio PC'}/>
        {/if}

        {#if status === 'Stopped'}
          <p class="arrange-hint mono">
            Neighbor side: <b>{activeSide.toUpperCase()}</b> · configure under Arrangement before starting
          </p>
          <div class="role-picker">
            <button class="role-card" on:click={() => { mode = 'Server'; startServer(); }}>
              <div class="role-title">Share this mouse</div>
              <div class="role-desc">
                This Mac keeps the cursor. When you move past the configured edge,
                the cursor jumps to the other Mac. Pick this on the machine whose
                keyboard &amp; trackpad you want to use.
              </div>
              <div class="role-cta">Host · Start server</div>
            </button>

            <button class="role-card" on:click={() => { mode = 'Client'; startClient(); }}>
              <div class="role-title">Receive cursor</div>
              <div class="role-desc">
                This Mac waits for a cursor to arrive. Pick this on the machine
                you want to <em>drive into</em> from the other side.
              </div>
              <div class="role-cta">Join · Find pair</div>
            </button>
          </div>
        {:else if showRadar}
          <div class="radar-slot">
            <PeerRadar {peers} selfName="This Mac" on:select={selectRadarPeer}/>
          </div>
          <p class="hint">You are a guest. Start FlowControl on the other Mac and pick <em>Share this mouse</em> there.</p>
        {:else if showHostWaiting}
          <div class="radar-slot">
            <div class="host-waiting">
              <div class="pulse"></div>
              <div class="host-title">Waiting for a guest…</div>
              <div class="host-sub">
                On the other Mac, open FlowControl and pick <em>Receive cursor</em>.
                It should show up under its peer list.
              </div>
            </div>
          </div>
        {:else if showEmpty}
          <EmptyState on:start={mode === 'Server' ? startServer : startClient}/>
        {:else}
          <div class="peer-grid">
            <PeerCard
              name="This Mac"
              sub={mode === 'Server' ? 'Host · owns the cursor' : 'Guest · receives the cursor'}
              os="mac"
              primary
              active={status === 'Connected'}/>
            {#if peers.length > 0}
              <PeerCard
                name={peers[0].name}
                sub={peers[0].os === 'win' ? 'Windows · paired' : 'Mac · paired'}
                os={peers[0].os ?? 'mac'}
                active={status === 'Remote'}
                latency={status === 'Connected' || status === 'Remote' ? 4 : null}/>
            {:else if status === 'Searching' && mode === 'Client'}
              <PeerCardSkeleton/>
            {:else if mode === 'Server'}
              <PeerCard name="Waiting for guest" sub="Another Mac will appear once paired" os="mac"/>
            {:else}
              <PeerCard name="Waiting for host" sub="No peers discovered on the network yet" os="mac"/>
            {/if}
          </div>
        {/if}

        <footer class="actions">
          {#if status !== 'Stopped'}
            <div class="mode-pill">{mode === 'Server' ? 'Host' : 'Guest'}</div>
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
  .actions { display: flex; justify-content: flex-end; align-items: center; gap: 10px; margin-top: auto; }
  .btn { padding: 8px 18px; border-radius: 999px; font-size: 13px; font-weight: 600; cursor: pointer; border: none; }
  .btn.ghost { background: rgba(0,0,0,0.05); color: var(--ink); }

  .role-picker { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-top: 8px; }
  .role-card {
    display: flex; flex-direction: column; gap: 8px;
    padding: 18px; border-radius: var(--radius-md);
    background: var(--paper-2); border: 1px solid var(--line);
    text-align: left; cursor: pointer; transition: border-color 0.15s, transform 0.15s;
  }
  .role-card:hover { border-color: var(--coral); transform: translateY(-1px); }
  .role-title { font-size: 15px; font-weight: 700; color: var(--ink); }
  .role-desc { font-size: 12px; color: var(--ink-2); line-height: 1.45; }
  .role-cta { font-size: 11px; font-weight: 600; color: var(--coral); margin-top: auto; letter-spacing: 0.2px; }

  .mode-pill {
    font-size: 11px; font-weight: 600; letter-spacing: 0.3px;
    padding: 3px 10px; border-radius: 999px;
    background: rgba(0,0,0,0.06); color: var(--ink-2);
  }
  .hint { font-size: 12px; color: var(--ink-2); text-align: center; margin: -16px 0 0; }
  .arrange-hint { font-size: 11px; color: var(--ink-3); letter-spacing: 0.2px; margin: -6px 0 -4px; }

  .host-waiting {
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: 10px; padding: 24px;
  }
  .pulse {
    width: 48px; height: 48px; border-radius: 50%;
    background: radial-gradient(circle, var(--coral) 0%, transparent 70%);
    animation: pulse 1.8s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { transform: scale(0.9); opacity: 0.6; }
    50%      { transform: scale(1.1); opacity: 1; }
  }
  .host-title { font-size: 14px; font-weight: 700; color: var(--ink); }
  .host-sub { font-size: 12px; color: var(--ink-2); text-align: center; max-width: 360px; line-height: 1.45; }
</style>
