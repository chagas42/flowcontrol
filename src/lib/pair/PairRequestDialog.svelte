<!-- pair/PairRequestDialog.svelte
     Renders inside StatusStrip's `pair-actions` slot AND as a standalone modal.
     Consumes:   listen('pair-incoming', { peer_name, fingerprint })
     Emits:      invoke('pair_accept') / invoke('pair_decline')
-->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { createEventDispatcher } from 'svelte';

  export let peerName = 'Studio PC';
  export let fingerprint: string = '84-ZX-19-PF';
  export let os: 'mac' | 'win' = 'win';
  export let variant: 'inline' | 'modal' = 'modal';

  const dispatch = createEventDispatcher();
  let busy = false;

  async function accept() {
    busy = true;
    try { await invoke('pair_accept'); dispatch('accepted'); }
    finally { busy = false; }
  }
  async function decline() {
    busy = true;
    try { await invoke('pair_decline'); dispatch('declined'); }
    finally { busy = false; }
  }

  $: fpParts = fingerprint.split('-');
</script>

{#if variant === 'modal'}
  <div class="backdrop" on:click={decline} on:keydown role="presentation">
    <div class="modal" on:click|stopPropagation role="dialog" aria-modal="true">
      <div class="head">
        <div class="avatar" class:win={os === 'win'} class:mac={os === 'mac'}>{os === 'mac' ? '' : '⊞'}</div>
        <div>
          <div class="title">{peerName} wants to pair</div>
          <div class="sub">A new device on your network is asking to share a mouse and keyboard.</div>
        </div>
      </div>

      <div class="fp-block">
        <div class="fp-label mono">VERIFY THIS CODE MATCHES ON BOTH DEVICES</div>
        <div class="fp mono" aria-label={fingerprint}>
          {#each fpParts as p, i}
            <span class="chunk">{p}</span>
            {#if i < fpParts.length - 1}<span class="sep">·</span>{/if}
          {/each}
        </div>
        <div class="fp-hint">If the code doesn't match exactly, decline — someone else may be trying to connect.</div>
      </div>

      <div class="actions">
        <button class="btn ghost" on:click={decline} disabled={busy}>Decline</button>
        <button class="btn solid" on:click={accept} disabled={busy}>Accept & pair</button>
      </div>
    </div>
  </div>
{:else}
  <div class="inline">
    <div class="fp-inline mono">{fingerprint}</div>
    <div class="btns">
      <button class="btn ghost sm" on:click={decline} disabled={busy}>Decline</button>
      <button class="btn solid sm" on:click={accept} disabled={busy}>Accept</button>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.35);
    display: grid; place-items: center; z-index: 1000;
    animation: fadeIn .12s ease-out;
  }
  .modal {
    width: 420px; background: var(--paper); border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg); padding: 24px;
    animation: pop .18s cubic-bezier(.2,.8,.3,1.2);
  }
  @keyframes fadeIn { from { opacity: 0 } to { opacity: 1 } }
  @keyframes pop { from { opacity: 0; transform: scale(.96) translateY(4px) } to { opacity: 1; transform: scale(1) } }

  .head { display: flex; gap: 14px; align-items: flex-start; margin-bottom: 20px; }
  .avatar {
    width: 44px; height: 44px; border-radius: 11px; flex-shrink: 0;
    display: grid; place-items: center; color: #fff; font-size: 20px; font-weight: 700;
  }
  .avatar.win { background: #0078d4; }
  .avatar.mac { background: #1e1e22; }

  .title { font-size: 16px; font-weight: 700; letter-spacing: -0.2px; color: var(--ink); }
  .sub   { font-size: 12.5px; color: var(--ink-2); margin-top: 4px; line-height: 1.4; }

  .fp-block {
    padding: 18px; border-radius: var(--radius-md);
    background: var(--coral-soft); border: 1px solid var(--coral);
    text-align: center; margin-bottom: 18px;
  }
  .fp-label { font-size: 9.5px; letter-spacing: 1px; color: var(--coral-deep); margin-bottom: 10px; }
  .fp {
    font-size: 28px; font-weight: 700; color: var(--coral-deep);
    letter-spacing: 2px; display: flex; justify-content: center; gap: 6px; margin-bottom: 10px;
  }
  .fp .chunk { display: inline-block; }
  .fp .sep { opacity: 0.4; }
  .fp-hint { font-size: 11px; color: var(--ink-2); line-height: 1.45; max-width: 320px; margin: 0 auto; }

  .actions { display: flex; justify-content: flex-end; gap: 8px; }

  /* inline variant (slot inside StatusStrip) */
  .inline { display: flex; align-items: center; gap: 10px; }
  .fp-inline {
    font-size: 13px; font-weight: 700; color: var(--coral-deep);
    letter-spacing: 1.5px; padding: 4px 10px; border-radius: 6px;
    background: var(--coral-soft); border: 1px solid var(--coral);
  }
  .btns { display: flex; gap: 6px; }

  .btn {
    padding: 8px 16px; border-radius: 999px; border: none; cursor: pointer;
    font-family: inherit; font-size: 13px; font-weight: 600;
    transition: opacity .12s, transform .12s;
  }
  .btn.sm { padding: 6px 12px; font-size: 12px; }
  .btn.solid {
    background: linear-gradient(180deg, var(--coral), var(--coral-deep)); color: #fff;
    box-shadow: 0 1px 2px rgba(0,0,0,0.15), inset 0 0.5px 0 rgba(255,255,255,0.3);
  }
  .btn.solid:active { transform: translateY(1px); }
  .btn.ghost { background: rgba(0,0,0,0.05); color: var(--ink); }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
