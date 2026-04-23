<!-- Kbd.svelte -->
<script lang="ts">
  // tiny keycap
</script>
<span class="k mono"><slot/></span>

<style>
  .k {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 20px; height: 20px; padding: 0 6px;
    border-radius: 5px; font-size: 11px; font-weight: 600;
    background: var(--paper-2); color: var(--ink);
    border: 1px solid var(--line);
    box-shadow: inset 0 -1px 0 rgba(0,0,0,0.06);
  }
</style>
