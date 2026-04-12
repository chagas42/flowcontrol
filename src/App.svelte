<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import ArrangeDisplays from './lib/ArrangeDisplays.svelte';
  import ConnectionStatus from './lib/ConnectionStatus.svelte';

  let status: "Stopped" | "Waiting" | "Connected" = "Stopped";
  let activeSide = 'Right';

  async function handleLayoutChanged(event: CustomEvent<{ side: string }>) {
    activeSide = event.detail.side;
    if (status !== 'Stopped') {
      try {
        await invoke('stop_coordinator');
        await startServer();
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
        width: 1920, 
        height: 1080, 
        side: activeSide 
      });
      status = "Connected";
    } catch (e) {
      console.error(e);
      status = "Stopped";
    }
  }

  async function stopService() {
    await invoke('stop_coordinator');
    status = "Stopped";
  }
</script>

<main class="app-container">
  <div class="top-bar">
    <ConnectionStatus {status} />
    
    <div class="actions">
      {#if status === "Stopped"}
        <button class="btn primary" on:click={startServer}>Start Server</button>
      {:else}
        <button class="btn secondary" on:click={stopService}>Stop</button>
      {/if}
    </div>
  </div>

  <ArrangeDisplays on:layoutChanged={handleLayoutChanged} />
</main>

<style>
  .app-container {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--mac-bg);
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
</style>
