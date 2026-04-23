<!-- Welcome.svelte -->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher();
</script>

<div class="onboard">
  <div class="brand">
    <div class="logo">
      <svg width="22" height="22" viewBox="0 0 22 22" fill="none">
        <path d="M2 11 H20 M13 4 L20 11 L13 18" stroke="#fff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </div>
    <div>
      <div class="name">FlowControl</div>
      <div class="ver mono">v0.1.0 · early preview</div>
    </div>
  </div>

  <h1>One mouse.<br/>Every screen on your desk.</h1>
  <p>Park your Mac and PC side by side. When your cursor runs off one screen, it continues on the other — no switch, no lag, no HDMI dance.</p>

  <div class="cta">
    <button class="btn solid" on:click={() => dispatch('continue')}>Get started</button>
    <button class="btn ghost">Already on another device</button>
  </div>

  <footer class="reqs">
    <span>Same Wi-Fi required</span>
    <span>macOS 13+</span>
    <span>Windows 10+</span>
    <span class="mono">MIT · open source</span>
  </footer>
</div>

<style>
  .onboard {
    padding: 60px 56px; height: 100vh; box-sizing: border-box;
    display: flex; flex-direction: column; background: var(--paper);
  }
  .brand { display: flex; align-items: center; gap: 14px; margin-bottom: 36px; }
  .logo {
    width: 44px; height: 44px; border-radius: 11px;
    background: linear-gradient(135deg, var(--coral), var(--coral-deep));
    display: grid; place-items: center;
    box-shadow: 0 6px 18px rgba(201,100,66,0.35), inset 0 1px 0 rgba(255,255,255,0.25);
  }
  .name { font-size: 22px; font-weight: 700; letter-spacing: -0.4px; color: var(--ink); }
  .ver  { font-size: 11px; color: var(--ink-3); letter-spacing: 0.4px; }
  h1 {
    font-size: 40px; font-weight: 700; letter-spacing: -1.2px; line-height: 1.05;
    color: var(--ink); margin-bottom: 18px; text-wrap: pretty; max-width: 560px;
  }
  p { font-size: 15px; line-height: 1.5; color: var(--ink-2); max-width: 520px; margin-bottom: 32px; }
  .cta { display: flex; gap: 10px; margin-bottom: auto; }
  .btn {
    padding: 12px 22px; border-radius: 999px; font-size: 14px; font-weight: 600;
    cursor: pointer; border: none; font-family: inherit;
  }
  .btn.solid { background: linear-gradient(180deg, var(--coral), var(--coral-deep)); color: #fff; }
  .btn.ghost { background: rgba(0,0,0,0.05); color: var(--ink); }
  .reqs {
    display: flex; gap: 24px; padding-top: 22px;
    border-top: 1px solid var(--line);
    font-size: 12px; color: var(--ink-2);
  }
  .reqs .mono { margin-left: auto; }
</style>
