<!-- screens/About.svelte -->
<script lang="ts">
  const version = '0.1.0';
  const repo = 'https://github.com/chagas42/flowcontrol';
</script>

<section class="page">
  <div class="hero">
    <div class="logo">
      <svg width="28" height="28" viewBox="0 0 22 22" fill="none">
        <path d="M2 11 H20 M13 4 L20 11 L13 18" stroke="#fff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </div>
    <div>
      <div class="name">FlowControl</div>
      <div class="ver mono">v{version} · early preview</div>
    </div>
  </div>

  <p class="blurb">
    One mouse. Every screen on your desk. FlowControl bridges your Mac and PC over the local network so your cursor moves continuously between them — no KVM, no HDMI dance.
  </p>

  <div class="grid">
    <div class="card">
      <div class="k mono">LICENSE</div>
      <div class="v">MIT</div>
    </div>
    <div class="card">
      <div class="k mono">TRANSPORT</div>
      <div class="v">mDNS + TCP · LAN only</div>
    </div>
    <div class="card">
      <div class="k mono">TELEMETRY</div>
      <div class="v">None</div>
    </div>
    <div class="card">
      <div class="k mono">ENGINE</div>
      <div class="v">Rust · Tauri v2</div>
    </div>
  </div>

  <div class="links">
    <a href={repo} target="_blank" rel="noreferrer">GitHub repository</a>
    <a href="{repo}/issues" target="_blank" rel="noreferrer">Report an issue</a>
    <a href="{repo}/blob/main/LICENSE" target="_blank" rel="noreferrer">License</a>
  </div>

  <footer class="foot">Made with care by the FlowControl contributors.</footer>
</section>

<style>
  .page { padding: 30px 36px; flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 20px; }
  .hero { display: flex; align-items: center; gap: 14px; }
  .logo {
    width: 52px; height: 52px; border-radius: 13px;
    background: linear-gradient(135deg, var(--coral), var(--coral-deep));
    display: grid; place-items: center;
    box-shadow: 0 6px 18px rgba(201,100,66,0.35), inset 0 1px 0 rgba(255,255,255,0.25);
  }
  .name { font-size: 24px; font-weight: 700; letter-spacing: -0.5px; color: var(--ink); }
  .ver  { font-size: 11px; color: var(--ink-3); letter-spacing: 0.4px; margin-top: 2px; }
  .blurb { font-size: 13.5px; line-height: 1.55; color: var(--ink-2); max-width: 520px; }

  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .card {
    padding: 12px 14px; border-radius: var(--radius-md);
    background: var(--paper-2); border: 1px solid var(--line);
  }
  .k { font-size: 9.5px; color: var(--ink-3); letter-spacing: 0.8px; margin-bottom: 4px; }
  .v { font-size: 12.5px; color: var(--ink); font-weight: 500; }

  .links { display: flex; gap: 14px; flex-wrap: wrap; }
  .links a {
    font-size: 12.5px; color: var(--coral-deep); text-decoration: none;
    padding: 4px 8px; border-radius: 6px; transition: background .12s;
  }
  .links a:hover { background: var(--coral-soft); }

  .foot { font-size: 11px; color: var(--ink-3); margin-top: auto; text-align: center; }
</style>
