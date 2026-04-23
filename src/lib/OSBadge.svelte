<!-- OSBadge.svelte — with proper Windows 11 quadrants logo -->
<script lang="ts">
  export let os: 'mac' | 'win' = 'mac';
  export let size = 20;
</script>

<div class="b" class:mac={os === 'mac'} class:win={os === 'win'}
     style="width:{size}px; height:{size}px; border-radius:{size * 0.22}px;">
  {#if os === 'mac'}
    <svg width={size * 0.62} height={size * 0.62} viewBox="0 0 16 16" fill="#fff">
      <path d="M10.8 2.3c-.6.7-1.5 1.2-2.4 1.1-.1-.9.3-1.8.8-2.4.6-.7 1.6-1.2 2.3-1.2.1.9-.2 1.8-.7 2.5zM11.5 4c-1.3-.1-2.4.7-3 .7-.7 0-1.6-.7-2.7-.7-1.4 0-2.7.8-3.4 2.1-1.5 2.5-.4 6.3 1 8.3.7 1 1.6 2.1 2.7 2.1 1.1 0 1.5-.7 2.8-.7s1.6.7 2.8.7c1.1 0 1.9-1 2.6-2 .8-1.1 1.2-2.3 1.2-2.3s-2.3-.9-2.3-3.5c0-2.2 1.8-3.2 1.9-3.3-1-1.5-2.6-1.7-3.2-1.8z"/>
    </svg>
  {:else}
    <svg width={size * 0.6} height={size * 0.6} viewBox="0 0 24 24" fill="#fff">
      <path d="M0 3.449L9.75 2.1v9.451H0m10.949-9.602L24 0v11.4H10.949M0 12.6h9.75v9.451L0 20.699M10.949 12.6H24V24l-12.9-1.801"/>
    </svg>
  {/if}
</div>

<style>
  .b { display: grid; place-items: center; flex-shrink: 0; }
  .b.mac { background: #1e1e22; }
  .b.win { background: #0078d4; }
</style>
