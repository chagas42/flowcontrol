<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import ArrangeDisplays from './lib/ArrangeDisplays.svelte';
  import ConnectionStatus from './lib/ConnectionStatus.svelte';
  import { onMount } from 'svelte';

  let status: "Stopped" | "Waiting" | "Connected" = "Stopped";
  let permissionRequired = false;
  let permissionGranted = false;
  let activeSide = 'Right';
  
  let mode: "Server" | "Client" = "Server";
  let peers: Array<{ id: string, name: string }> = [];

  onMount(() => {
    listen('peers-updated', (event) => {
      peers = event.payload as any;
    });
    listen('status-changed', (event) => {
      status = event.payload as any;
    });
    listen('permission-required', () => {
      permissionRequired = true;
      permissionGranted = false;
    });
  });

  async function handleLayoutChanged(event: CustomEvent<{ side: string }>) {
    activeSide = event.detail.side;
    if (status !== 'Stopped') {
      try {
        await invoke('stop_coordinator');
        if (mode === 'Server') await startServer();
        else await startClient(); 
      } catch(e) {
        console.error(e);
      }
    }
  }

  async function startServer() {
    try {
      status = "Waiting";
      await invoke('start_server', {
        name: "My Mac",
        width: window.screen.width,
        height: window.screen.height,
        side: activeSide
      });
    } catch (e) {
      console.error(e);
      status = "Stopped";
    }
  }

  async function startClient() {
    try {
      status = "Waiting";
      await invoke('start_client', {
        width: window.screen.width,
        height: window.screen.height,
        side: activeSide
      });
    } catch (e) {
      console.error(e);
      status = "Stopped";
    }
  }

  async function connectToPeer(id: string) {
    try {
      await invoke('connect_to_peer', { peerId: id });
    } catch(e) {
      console.error(e);
    }
  }

  async function stopService() {
    await invoke('stop_coordinator');
    status = "Stopped";
    peers = [];
  }

  async function requestPermission() {
    const granted: boolean = await invoke('request_accessibility_permission');
    if (granted) {
      permissionRequired = false;
      permissionGranted = true;
    } else {
      // System Settings opened — poll every second until granted
      const interval = setInterval(async () => {
        const ok: boolean = await invoke('request_accessibility_permission');
        if (ok) {
          clearInterval(interval);
          permissionRequired = false;
          permissionGranted = true;
          // Re-start coordinator so capture gets retried
          if (status !== 'Stopped') {
            const prevMode = mode;
            await stopService();
            if (prevMode === 'Server') await startServer();
            else await startClient();
          }
        }
      }, 1000);
    }
  }

  function toggleMode(newMode: "Server" | "Client") {
    if (status !== "Stopped") stopService();
    mode = newMode;
  }
</script>

<main class="app-container">
  <div class="top-bar">
    <div class="mode-selector">
      <button class="toggle-btn {mode === 'Server' ? 'active' : ''}" on:click={() => toggleMode('Server')}>Server</button>
      <button class="toggle-btn {mode === 'Client' ? 'active' : ''}" on:click={() => toggleMode('Client')}>Client</button>
    </div>
    
    <ConnectionStatus {status} />
    
    <div class="actions">
      {#if status === "Stopped"}
        <button class="btn primary" on:click={mode === 'Server' ? startServer : startClient}>
          {mode === 'Server' ? 'Start Server' : 'Find Pair'}
        </button>
      {:else}
        <button class="btn secondary" on:click={stopService}>Stop</button>
      {/if}
    </div>
  </div>

  {#if permissionRequired}
    <div class="permission-banner">
      <span>Accessibility permission required — FlowControl needs it to capture mouse events.</span>
      <button class="btn primary small" on:click={requestPermission}>Open System Settings</button>
    </div>
  {/if}
  {#if permissionGranted}
    <div class="permission-ok">Accessibility granted — capture active.</div>
  {/if}

  <ArrangeDisplays on:layoutChanged={handleLayoutChanged} />

  {#if mode === 'Client' && status === 'Waiting'}
    {#if peers.length === 0}
      <p class="search-text">Searching for servers on the local network...</p>
    {:else}
      <div class="peer-list">
        <h3>Available Macs to connect:</h3>
        {#each peers as peer}
          <div class="peer-item">
            <span>{peer.name}</span>
            <button class="btn primary small" on:click={() => connectToPeer(peer.id)}>Connect</button>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</main>

<style>
  .app-container {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--mac-bg);
    overflow-y: auto;
  }

  .top-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 24px 40px 12px 40px;
  }

  .actions {
    display: flex;
    gap: 12px;
  }

  .btn {
    padding: 4px 16px;
    border-radius: 6px;
    font-size: 13px;
    line-height: 20px;
    font-weight: 500;
    transition: all 0.2s ease;
    box-shadow: 0 1px 2px rgba(0,0,0,0.05);
  }

  .btn.primary {
    background: #007aff;
    color: white;
    border: 1px solid #0062cc;
  }

  .btn.primary:hover {
    background: #0062cc;
  }

  .btn.secondary {
    background: #ffffff;
    color: #333;
    border: 1px solid #d1d1d1;
  }

  .btn.secondary:hover {
    background: #f0f0f0;
  }

  .mode-selector {
    display: flex;
    background: #e8e8e8;
    border-radius: 6px;
    padding: 2px;
    border: 1px solid var(--mac-border);
  }

  .toggle-btn {
    padding: 4px 14px;
    font-size: 12px;
    border-radius: 4px;
    color: #4a4a4a;
    font-weight: 500;
  }

  .toggle-btn.active {
    background: #ffffff;
    box-shadow: 0 1px 2px rgba(0,0,0,0.1);
    color: #000;
  }

  .search-text {
    text-align: center;
    font-size: 12px;
    color: #888;
    margin-bottom: 20px;
    animation: pulse 1.5s infinite alternate;
  }

  @keyframes pulse {
    0% { opacity: 0.5; }
    100% { opacity: 1; }
  }

  .peer-list {
    margin: 0 40px 20px 40px;
    background: white;
    border: 1px solid var(--mac-border);
    border-radius: 6px;
    padding: 16px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.05);
  }
  
  .peer-list h3 {
    font-size: 13px;
    margin-bottom: 12px;
    color: #555;
  }
  
  .peer-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: #f8f8f8;
    border-radius: 4px;
    margin-bottom: 8px;
    font-size: 14px;
    border: 1px solid #eaeaea;
  }

  .peer-item:last-child {
    margin-bottom: 0;
  }
  
  .btn.small {
    padding: 4px 10px;
    font-size: 12px;
  }

  .permission-banner {
    margin: 0 40px 12px 40px;
    padding: 10px 16px;
    background: #fff3cd;
    border: 1px solid #ffc107;
    border-radius: 6px;
    font-size: 13px;
    color: #664d03;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  .permission-ok {
    margin: 0 40px 12px 40px;
    padding: 10px 16px;
    background: #d1f5d3;
    border: 1px solid #34c759;
    border-radius: 6px;
    font-size: 13px;
    color: #1a5e2a;
  }
</style>
