<!-- screens/Advanced.svelte -->
<script lang="ts">
  // All flags are UI-only for now; wire to tauri-plugin-store in Phase 6.
  let launchOnLogin = false;
  let autoReconnect = true;
  let telemetry = false;
  let darkOverride: 'system' | 'light' | 'dark' = 'system';
  let mdnsPort = 7070;

  function applyTheme() {
    if (darkOverride === 'system') {
      document.documentElement.removeAttribute('data-theme');
    } else {
      document.documentElement.setAttribute('data-theme', darkOverride);
    }
  }
  $: darkOverride, applyTheme();
</script>

<section class="page">
  <header>
    <div class="title">Advanced</div>
    <div class="sub">Power-user settings. Defaults are safe for everyone.</div>
  </header>

  <div class="group">
    <div class="group-head">Startup</div>
    <label class="row">
      <div class="body">
        <div class="label">Launch on login</div>
        <div class="desc">Start FlowControl silently when you sign in.</div>
      </div>
      <input type="checkbox" class="switch" bind:checked={launchOnLogin}/>
    </label>
    <label class="row">
      <div class="body">
        <div class="label">Auto-reconnect to last peer</div>
        <div class="desc">Rejoin the most recent pair when both devices come back online.</div>
      </div>
      <input type="checkbox" class="switch" bind:checked={autoReconnect}/>
    </label>
  </div>

  <div class="group">
    <div class="group-head">Privacy</div>
    <label class="row">
      <div class="body">
        <div class="label">Anonymous telemetry</div>
        <div class="desc">Off by default. We don't collect input data, ever.</div>
      </div>
      <input type="checkbox" class="switch" bind:checked={telemetry}/>
    </label>
  </div>

  <div class="group">
    <div class="group-head">Appearance</div>
    <div class="row">
      <div class="body">
        <div class="label">Theme</div>
        <div class="desc">System follows your OS preference.</div>
      </div>
      <div class="seg">
        {#each ['system', 'light', 'dark'] as opt}
          <button class="seg-btn" class:on={darkOverride === opt} on:click={() => darkOverride = opt}>{opt}</button>
        {/each}
      </div>
    </div>
  </div>

  <div class="group">
    <div class="group-head">Network</div>
    <label class="row">
      <div class="body">
        <div class="label">Discovery port</div>
        <div class="desc">mDNS service port. Change if you have a conflict.</div>
      </div>
      <input type="number" class="num mono" bind:value={mdnsPort} min="1024" max="65535"/>
    </label>
  </div>
</section>

<style>
  .page { padding: 24px 30px; flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 16px; }
  .title { font-size: 22px; font-weight: 700; letter-spacing: -0.4px; color: var(--ink); }
  .sub { font-size: 13px; color: var(--ink-2); margin-top: 2px; }

  .group { display: flex; flex-direction: column; gap: 1px; background: var(--line); border-radius: var(--radius-md); overflow: hidden; border: 1px solid var(--line); }
  .group-head {
    padding: 10px 16px; background: var(--paper-2);
    font-size: 10.5px; letter-spacing: 0.8px; color: var(--ink-3);
    font-family: 'JetBrains Mono', monospace; text-transform: uppercase;
  }
  .row {
    padding: 12px 16px; background: var(--paper);
    display: flex; align-items: center; gap: 16px; cursor: pointer;
  }
  .body { flex: 1; }
  .label { font-size: 13px; font-weight: 500; color: var(--ink); }
  .desc  { font-size: 11.5px; color: var(--ink-3); margin-top: 2px; }

  .switch {
    appearance: none; width: 34px; height: 20px; border-radius: 999px;
    background: var(--line-2); position: relative; cursor: pointer;
    transition: background .15s;
  }
  .switch::after {
    content: ''; position: absolute; top: 2px; left: 2px;
    width: 16px; height: 16px; border-radius: 50%; background: #fff;
    box-shadow: var(--shadow-sm); transition: transform .15s;
  }
  .switch:checked { background: var(--coral); }
  .switch:checked::after { transform: translateX(14px); }

  .seg {
    display: inline-flex; background: var(--paper-2); border: 1px solid var(--line);
    border-radius: 7px; padding: 2px; gap: 1px;
  }
  .seg-btn {
    padding: 4px 10px; font-size: 11.5px; border: none; background: none; cursor: pointer;
    color: var(--ink-2); border-radius: 5px; font-family: inherit; text-transform: capitalize;
  }
  .seg-btn.on { background: var(--paper); color: var(--ink); box-shadow: var(--shadow-sm); }

  .num {
    width: 80px; padding: 5px 8px; font-size: 12.5px;
    border: 1px solid var(--line); border-radius: 6px; background: var(--paper);
    color: var(--ink); text-align: right;
  }
</style>
