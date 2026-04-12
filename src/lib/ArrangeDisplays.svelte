<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  const dispatch = createEventDispatcher();

  // Displays state (canvas center is ~300x160)
  let primaryRect = { x: 220, y: 110, w: 160, h: 100, active: true };
  let secRect = { x: 380, y: 110, w: 160, h: 100, active: false };
  
  let isDragging = false;
  let dragOffsetX = 0;
  let dragOffsetY = 0;

  function handlePointerDown(e: PointerEvent) {
    const target = e.target as HTMLElement;
    if (target.closest('.secondary-display')) {
      isDragging = true;
      dragOffsetX = e.clientX - secRect.x;
      dragOffsetY = e.clientY - secRect.y;
      primaryRect.active = false;
      secRect.active = true;
      target.setPointerCapture(e.pointerId);
    } else if (target.closest('.primary-display')) {
      primaryRect.active = true;
      secRect.active = false;
    }
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging) return;
    secRect.x = e.clientX - dragOffsetX;
    secRect.y = e.clientY - dragOffsetY;
  }

  function handlePointerUp(e: PointerEvent) {
    if (!isDragging) return;
    isDragging = false;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);

    // Snap to left or right
    let centerX_sec = secRect.x + secRect.w / 2;
    let centerX_prim = primaryRect.x + primaryRect.w / 2;
    
    let side = 'Right';
    if (centerX_sec < centerX_prim) {
      secRect.x = primaryRect.x - secRect.w;
      side = 'Left';
    } else {
      secRect.x = primaryRect.x + primaryRect.w;
      side = 'Right';
    }
    secRect.y = primaryRect.y; // perfect alignment 

    dispatch('layoutChanged', { side });
  }
</script>

<div class="arrangement-view">
  <div class="header-text">
    <p>To rearrange the displays, drag them to the desired position.</p>
    <p>To relocate the menu bar, drag it to a different display.</p>
  </div>

  <div class="canvas-container" on:pointerdown={handlePointerDown} on:pointermove={handlePointerMove} on:pointerup={handlePointerUp}>
    <!-- Primary Display (Mac) -->
    <div 
      class="display-rect primary-display {primaryRect.active ? 'active' : ''}" 
      style="left: {primaryRect.x}px; top: {primaryRect.y}px; width: {primaryRect.w}px; height: {primaryRect.h}px;"
    >
      <div class="menu-bar"></div>
    </div>

    <!-- Secondary Display (Windows) -->
    <div 
      class="display-rect secondary-display {secRect.active ? 'active' : ''}" 
      style="left: {secRect.x}px; top: {secRect.y}px; width: {secRect.w}px; height: {secRect.h}px; cursor: {isDragging ? 'grabbing' : 'grab'};"
    >
    </div>
  </div>
</div>

<style>
  .arrangement-view {
    padding: 20px 40px;
    display: flex;
    flex-direction: column;
    flex: 1;
  }

  .header-text {
    font-size: 13px;
    color: #4a4a4a;
    margin-bottom: 16px;
    line-height: 1.4;
  }

  .canvas-container {
    position: relative;
    width: 100%;
    height: 320px;
    background: #fdfdfd;
    border: 1px solid #d4d4d4;
    box-shadow: inset 0 2px 4px rgba(0,0,0,0.04);
    overflow: hidden;
  }

  .display-rect {
    position: absolute;
    background: var(--mac-blue);
    border: 1px solid var(--mac-blue-border);
    box-shadow: 0 2px 6px rgba(0,0,0,0.2);
    box-sizing: border-box;
    transition: box-shadow 0.2s;
  }

  .display-rect.active {
    border: 2px solid white;
    box-shadow: 0 0 0 1px var(--mac-blue-border), 0 4px 12px rgba(0,0,0,0.3);
    z-index: 10;
  }

  .menu-bar {
    width: 100%;
    height: 8px;
    background: rgba(255,255,255,0.85);
    border-bottom: 1px solid rgba(0,0,0,0.1);
  }
</style>
