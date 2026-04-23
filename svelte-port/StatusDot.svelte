<!-- StatusDot.svelte -->
<script lang="ts">
  export let color: string = '#22c55e';
  export let pulse = false;
  export let size = 8;
</script>

<span class="sd" style="width:{size}px; height:{size}px;">
  {#if pulse}<span class="pulse" style="background:{color}"></span>{/if}
  <span class="dot" style="background:{color}; box-shadow:0 0 6px {color}88;"></span>
</span>

<style>
  .sd { position: relative; display: inline-flex; }
  .dot { width: 100%; height: 100%; border-radius: 50%; }
  .pulse {
    position: absolute; inset: 0; border-radius: 50%; opacity: 0.35;
    animation: fcPulse 1.6s ease-out infinite;
  }
  @keyframes fcPulse { 0% { transform: scale(1); opacity: .5 } 100% { transform: scale(2.6); opacity: 0 } }
</style>
