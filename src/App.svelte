<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import TitleBar from './lib/components/TitleBar.svelte';
  import Calendar from './lib/components/Calendar.svelte';
  import Settings from './lib/components/Settings.svelte';
  import { calendarStore } from './lib/stores/calendar.svelte';

  let settingsOpen = $state(false);
  let resizeMode = $state(false);

  // Resize state
  let resizing = $state(false);
  let resizeHandle = $state('');
  let startMouseX = 0;
  let startMouseY = 0;
  let startRect = { x: 0, y: 0, w: 0, h: 0 };
  const MIN_W = 600;
  const MIN_H = 400;

  onMount(() => {
    calendarStore.load();
  });

  function toggleSettings() {
    settingsOpen = !settingsOpen;
  }

  function closeSettings() {
    settingsOpen = false;
  }

  function enterResizeMode() {
    settingsOpen = false;
    resizeMode = true;
  }

  function exitResizeMode() {
    resizeMode = false;
  }

  async function handleResizeStart(e: MouseEvent, handle: string) {
    e.preventDefault();
    e.stopPropagation();
    resizing = true;
    resizeHandle = handle;
    startMouseX = e.screenX;
    startMouseY = e.screenY;
    try {
      const [x, y, w, h] = await invoke<[number, number, number, number]>('get_window_rect');
      startRect = { x, y, w, h };
    } catch {
      resizing = false;
    }
  }

  function handleResizeMove(e: MouseEvent) {
    if (!resizing) return;
    const dx = e.screenX - startMouseX;
    const dy = e.screenY - startMouseY;
    let { x, y, w, h } = startRect;

    if (resizeHandle.includes('r')) w = Math.max(MIN_W, w + dx);
    if (resizeHandle.includes('b')) h = Math.max(MIN_H, h + dy);
    if (resizeHandle.includes('l')) {
      const newW = Math.max(MIN_W, w - dx);
      x = x + (w - newW);
      w = newW;
    }
    if (resizeHandle.includes('t')) {
      const newH = Math.max(MIN_H, h - dy);
      y = y + (h - newH);
      h = newH;
    }

    invoke('set_window_rect', { x, y, w, h });
  }

  function handleResizeEnd() {
    resizing = false;
    resizeHandle = '';
  }
</script>

<svelte:window
  onmousemove={resizeMode ? handleResizeMove : undefined}
  onmouseup={resizeMode ? handleResizeEnd : undefined}
/>

<main class="app-shell" class:resize-active={resizeMode}>
  <TitleBar onToggleSettings={toggleSettings} />
  <Calendar />
</main>

{#if settingsOpen}
  <Settings onClose={closeSettings} onResizeMode={enterResizeMode} />
{/if}

{#if resizeMode}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="resize-overlay">
    <div class="resize-handle handle-t" onmousedown={(e) => handleResizeStart(e, 't')}></div>
    <div class="resize-handle handle-b" onmousedown={(e) => handleResizeStart(e, 'b')}></div>
    <div class="resize-handle handle-l" onmousedown={(e) => handleResizeStart(e, 'l')}></div>
    <div class="resize-handle handle-r" onmousedown={(e) => handleResizeStart(e, 'r')}></div>
    <div class="resize-handle handle-tl" onmousedown={(e) => handleResizeStart(e, 'tl')}></div>
    <div class="resize-handle handle-tr" onmousedown={(e) => handleResizeStart(e, 'tr')}></div>
    <div class="resize-handle handle-bl" onmousedown={(e) => handleResizeStart(e, 'bl')}></div>
    <div class="resize-handle handle-br" onmousedown={(e) => handleResizeStart(e, 'br')}></div>
    <button class="resize-done-btn" onclick={exitResizeMode}>완료</button>
  </div>
{/if}

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100vh;
    background: var(--bg-primary);
    border-radius: var(--radius);
    border: 1px solid var(--border);
    overflow: hidden;
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    box-shadow:
      0 8px 32px rgba(0, 0, 0, 0.4),
      inset 0 1px 0 rgba(255, 255, 255, 0.05);
  }

  .app-shell.resize-active {
    border: 2px dashed var(--accent);
  }

  .resize-overlay {
    position: fixed;
    inset: 0;
    z-index: 9999;
  }

  .resize-handle {
    position: absolute;
    background: transparent;
  }

  /* Edge handles */
  .handle-t { top: 0; left: 12px; right: 12px; height: 6px; cursor: n-resize; }
  .handle-b { bottom: 0; left: 12px; right: 12px; height: 6px; cursor: s-resize; }
  .handle-l { left: 0; top: 12px; bottom: 12px; width: 6px; cursor: w-resize; }
  .handle-r { right: 0; top: 12px; bottom: 12px; width: 6px; cursor: e-resize; }

  /* Corner handles */
  .handle-tl { top: 0; left: 0; width: 12px; height: 12px; cursor: nw-resize; }
  .handle-tr { top: 0; right: 0; width: 12px; height: 12px; cursor: ne-resize; }
  .handle-bl { bottom: 0; left: 0; width: 12px; height: 12px; cursor: sw-resize; }
  .handle-br { bottom: 0; right: 0; width: 12px; height: 12px; cursor: se-resize; }

  .resize-handle:hover {
    background: rgba(79, 195, 247, 0.3);
  }

  .resize-done-btn {
    position: absolute;
    bottom: 16px;
    left: 50%;
    transform: translateX(-50%);
    padding: 8px 24px;
    border-radius: 8px;
    background: var(--accent);
    color: #0d1117;
    font-size: 13px;
    font-weight: 600;
    font-family: inherit;
    border: none;
    cursor: pointer;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    transition: filter 0.15s;
  }

  .resize-done-btn:hover {
    filter: brightness(1.1);
  }
</style>
