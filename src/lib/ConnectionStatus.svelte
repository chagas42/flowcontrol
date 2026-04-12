<script lang="ts">
  export let status: "Connected" | "Waiting" | "Stopped" = "Stopped";
</script>

<div class="status-indicator">
  <div class="dot {status.toLowerCase()}"></div>
  <span class="label">
    {#if status === "Connected"}
      Connected
    {:else if status === "Waiting"}
      Waiting for peer...
    {:else}
      Service Stopped
    {/if}
  </span>
</div>

<style>
  .status-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background: #ffffff;
    border: 1px solid var(--mac-border);
    border-radius: 12px;
    box-shadow: 0 1px 2px rgba(0,0,0,0.05);
    font-size: 13px;
    font-weight: 500;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #cfcfcf;
    box-shadow: inset 0 1px 2px rgba(0,0,0,0.2);
  }

  .dot.connected {
    background: #34c759;
  }

  .dot.waiting {
    background: #ffcc00;
    animation: pulse 1.5s infinite alternate;
  }

  .label {
    color: #4a4a4a;
  }

  @keyframes pulse {
    from { opacity: 0.6; }
    to { opacity: 1; }
  }
</style>
