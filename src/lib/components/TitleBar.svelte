<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { calendarStore } from '../stores/calendar.svelte';

  interface Props {
    onToggleSettings: () => void;
  }

  let { onToggleSettings }: Props = $props();
  let syncing = $state(false);

  async function handleSync(e: MouseEvent) {
    e.stopPropagation();
    if (syncing || !calendarStore.googleConnected) return;
    syncing = true;
    try {
      await calendarStore.fetchGoogleEvents();
    } finally {
      syncing = false;
    }
  }

  let isDragging = $state(false);
  let dragStartX = 0;
  let dragStartY = 0;
  let windowStartX = 0;
  let windowStartY = 0;

  async function handleDragStart(e: MouseEvent) {
    isDragging = true;
    dragStartX = e.screenX;
    dragStartY = e.screenY;
    try {
      const pos: [number, number] = await invoke('get_window_position');
      windowStartX = pos[0];
      windowStartY = pos[1];
    } catch {
      windowStartX = 0;
      windowStartY = 0;
    }

    window.addEventListener('mousemove', handleDragMove);
    window.addEventListener('mouseup', handleDragEnd);
  }

  function handleDragMove(e: MouseEvent) {
    if (!isDragging) return;
    const dx = e.screenX - dragStartX;
    const dy = e.screenY - dragStartY;
    invoke('move_window', { x: windowStartX + dx, y: windowStartY + dy });
  }

  function handleDragEnd() {
    isDragging = false;
    window.removeEventListener('mousemove', handleDragMove);
    window.removeEventListener('mouseup', handleDragEnd);
  }

  function handleMinimize(e: MouseEvent) {
    e.stopPropagation();
    getCurrentWindow().hide();
  }

  function handleClose(e: MouseEvent) {
    e.stopPropagation();
    getCurrentWindow().close();
  }

  function handleSettings(e: MouseEvent) {
    e.stopPropagation();
    onToggleSettings();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header class="titlebar" onmousedown={handleDragStart}>
  <span class="titlebar-title">miniCalender</span>
  <div class="titlebar-controls">
    {#if calendarStore.googleConnected}
      <button class="titlebar-btn" class:syncing onclick={handleSync} onmousedown={(e) => e.stopPropagation()} aria-label="Sync" title="동기화">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21.5 2v6h-6" />
          <path d="M2.5 22v-6h6" />
          <path d="M2.5 11.5a10 10 0 0 1 18.8-4.3L21.5 8" />
          <path d="M21.5 12.5a10 10 0 0 1-18.8 4.2L2.5 16" />
        </svg>
      </button>
    {/if}
    <button class="titlebar-btn" onclick={handleSettings} onmousedown={(e) => e.stopPropagation()} aria-label="Settings" title="Settings">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
    </button>
    <button class="titlebar-btn" onclick={handleMinimize} onmousedown={(e) => e.stopPropagation()} aria-label="Minimize" title="Minimize">
      <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
        <rect x="2" y="5.5" width="8" height="1" rx="0.5" />
      </svg>
    </button>
    <button class="titlebar-btn titlebar-btn--close" onclick={handleClose} onmousedown={(e) => e.stopPropagation()} aria-label="Close" title="Close">
      <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
        <path d="M2.22 2.22a.75.75 0 0 1 1.06 0L6 4.94l2.72-2.72a.75.75 0 0 1 1.06 1.06L7.06 6l2.72 2.72a.75.75 0 0 1-1.06 1.06L6 7.06 3.28 9.78a.75.75 0 0 1-1.06-1.06L4.94 6 2.22 3.28a.75.75 0 0 1 0-1.06z" />
      </svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 32px;
    padding: 0 8px 0 14px;
    background: rgba(20, 20, 30, 0.5);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    cursor: grab;
  }

  .titlebar:active {
    cursor: grabbing;
  }

  .titlebar-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: 0.3px;
    pointer-events: none;
  }

  .titlebar-controls {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .titlebar-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 24px;
    border-radius: 6px;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
    cursor: default;
  }

  .titlebar-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .titlebar-btn--close:hover {
    background: rgba(220, 60, 60, 0.6);
    color: #fff;
  }

  .syncing svg {
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
