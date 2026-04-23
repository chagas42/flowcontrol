<!-- PeerCardSkeleton.svelte — shimmer placeholder while searching -->
<div class="card">
  <div class="avatar shimmer"></div>
  <div class="body">
    <div class="line shimmer" style="width: 110px;"></div>
    <div class="line shimmer" style="width: 170px; height: 9px; margin-top: 6px;"></div>
  </div>
  <div class="right">
    <div class="line shimmer" style="width: 44px;"></div>
  </div>
</div>

<style>
  .card {
    padding: 14px 16px; border-radius: var(--radius-md);
    background: var(--paper); border: 1px solid var(--line);
    display: flex; align-items: center; gap: 12px;
  }
  .avatar { width: 28px; height: 28px; border-radius: 6px; }
  .body { flex: 1; }
  .line { height: 11px; border-radius: 4px; }
  .right { margin-left: auto; }
  .shimmer {
    background: linear-gradient(90deg, var(--paper-2) 0%, var(--line) 40%, var(--paper-2) 80%);
    background-size: 200% 100%;
    animation: shim 1.4s ease-in-out infinite;
  }
  @keyframes shim { 0% { background-position: 200% 0 } 100% { background-position: -200% 0 } }
</style>
