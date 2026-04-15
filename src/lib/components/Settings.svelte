<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
  import { calendarStore } from '../stores/calendar.svelte';

  interface Props {
    onClose: () => void;
    onResizeMode: () => void;
  }

  let { onClose, onResizeMode }: Props = $props();

  // Google Calendar connection status
  let googleConnected = $state(false);
  let googleLoading = $state(false);

  // Opacity
  let opacity = $state(100);

  // Widget size (free resize mode)

  // Autostart toggle
  let autostart = $state(false);
  let autostartLoading = $state(false);

  // Theme colors
  const THEME_COLORS = [
    { value: '#4fc3f7', label: 'Blue' },
    { value: '#81c784', label: 'Green' },
    { value: '#f06292', label: 'Pink' },
    { value: '#ffb74d', label: 'Orange' },
    { value: '#ba68c8', label: 'Purple' },
  ];
  let selectedColor = $state('#4fc3f7');

  // Slide-in animation state
  let visible = $state(false);

  $effect(() => {
    // Trigger slide-in on mount
    requestAnimationFrame(() => {
      visible = true;
    });
  });

  // Initialize on mount (plain call, not $effect — avoids reactive re-runs)
  onMount(() => {
    checkGoogleAuth();
    loadAutostartStatus();
  });

  async function checkGoogleAuth() {
    try {
      const status = await invoke<boolean>('google_auth_status');
      googleConnected = status;
    } catch {
      // Command not available yet, leave as disconnected
      googleConnected = false;
    }
  }

  async function loadAutostartStatus() {
    try {
      autostart = await isEnabled();
    } catch {
      autostart = false;
    }
  }

  async function handleAutostartToggle() {
    autostartLoading = true;
    try {
      if (autostart) {
        await disable();
        autostart = false;
      } else {
        await enable();
        autostart = true;
      }
    } catch {
      // ignore — state stays unchanged
    } finally {
      autostartLoading = false;
    }
  }

  async function handleGoogleConnect() {
    googleLoading = true;
    try {
      await invoke('google_auth_start');
      googleConnected = true;
      calendarStore.googleConnected = true;
      await calendarStore.fetchGoogleEvents();
    } catch {
      // Auth failed or not implemented yet
    } finally {
      googleLoading = false;
    }
  }

  async function handleGoogleDisconnect() {
    googleLoading = true;
    try {
      await invoke('google_auth_disconnect');
    } catch {
      // Not implemented yet
    }
    googleConnected = false;
    calendarStore.googleConnected = false;
    // Remove google events from store, keep local only
    calendarStore.events = calendarStore.events.filter((e) => e.source === 'local');
    googleLoading = false;
  }

  function handleOpacityChange(e: Event) {
    const target = e.target as HTMLInputElement;
    opacity = parseInt(target.value);
    // Apply opacity to the window in real-time
    // Note: setOpacity is not available on all platforms; fallback to CSS
    try {
      getCurrentWindow().setOpacity(opacity / 100);
    } catch {
      // Fallback: apply via CSS on the app shell
      document.documentElement.style.opacity = String(opacity / 100);
    }
  }

  function handleResizeMode() {
    onResizeMode();
  }

  function handleColorChange(color: string) {
    selectedColor = color;
    document.documentElement.style.setProperty('--accent', color);
    // Also update accent-dim for selected day cells
    const r = parseInt(color.slice(1, 3), 16);
    const g = parseInt(color.slice(3, 5), 16);
    const b = parseInt(color.slice(5, 7), 16);
    document.documentElement.style.setProperty('--accent-dim', `rgba(${r}, ${g}, ${b}, 0.15)`);
  }

  function handleClose() {
    visible = false;
    setTimeout(() => {
      onClose();
    }, 200);
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="settings-overlay" class:visible onclick={handleBackdropClick}>
  <div class="settings-panel" class:visible>
    <div class="settings-header">
      <h2 class="settings-title">설정</h2>
      <button class="close-btn" onclick={handleClose} aria-label="Close settings">
        <svg width="14" height="14" viewBox="0 0 12 12" fill="currentColor">
          <path d="M2.22 2.22a.75.75 0 0 1 1.06 0L6 4.94l2.72-2.72a.75.75 0 0 1 1.06 1.06L7.06 6l2.72 2.72a.75.75 0 0 1-1.06 1.06L6 7.06 3.28 9.78a.75.75 0 0 1-1.06-1.06L4.94 6 2.22 3.28a.75.75 0 0 1 0-1.06z" />
        </svg>
      </button>
    </div>

    <div class="settings-body">
      <!-- Google Calendar -->
      <section class="settings-section">
        <h3 class="section-title">Google Calendar 연동</h3>
        <div class="section-content">
          <div class="status-row">
            <span class="status-dot" class:connected={googleConnected}></span>
            <span class="status-text">{googleConnected ? '연동됨' : '연동 안됨'}</span>
          </div>
          {#if googleConnected}
            <button
              class="settings-btn settings-btn--danger"
              onclick={handleGoogleDisconnect}
              disabled={googleLoading}
            >
              {googleLoading ? '처리 중...' : '연동 해제'}
            </button>
          {:else}
            <button
              class="settings-btn settings-btn--primary"
              onclick={handleGoogleConnect}
              disabled={googleLoading}
            >
              {googleLoading ? '처리 중...' : '연동하기'}
            </button>
          {/if}
        </div>
      </section>

      <div class="divider"></div>

      <!-- Opacity -->
      <section class="settings-section">
        <h3 class="section-title">배경 투명도</h3>
        <div class="section-content">
          <div class="slider-row">
            <input
              type="range"
              class="range-slider"
              min="50"
              max="100"
              value={opacity}
              oninput={handleOpacityChange}
            />
            <span class="slider-value">{opacity}%</span>
          </div>
        </div>
      </section>

      <div class="divider"></div>

      <!-- Widget Size -->
      <section class="settings-section">
        <h3 class="section-title">위젯 크기</h3>
        <div class="section-content">
          <button class="settings-btn settings-btn--primary" onclick={handleResizeMode}>
            크기 조절
          </button>
        </div>
      </section>

      <div class="divider"></div>

      <!-- Autostart -->
      <section class="settings-section">
        <h3 class="section-title">시작 설정</h3>
        <div class="section-content">
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="toggle-row" onclick={handleAutostartToggle} class:loading={autostartLoading}>
            <span class="toggle-label">Windows 시작 시 자동 실행</span>
            <div class="toggle-switch" class:active={autostart}>
              <div class="toggle-knob"></div>
            </div>
          </div>
        </div>
      </section>

      <div class="divider"></div>

      <!-- Theme Color -->
      <section class="settings-section">
        <h3 class="section-title">테마 색상</h3>
        <div class="section-content">
          <div class="color-swatches">
            {#each THEME_COLORS as color}
              <button
                class="color-swatch"
                class:active={selectedColor === color.value}
                style:background={color.value}
                onclick={() => handleColorChange(color.value)}
                aria-label={color.label}
                title={color.label}
              >
                {#if selectedColor === color.value}
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#0d1117" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                {/if}
              </button>
            {/each}
          </div>
        </div>
      </section>
    </div>
  </div>
</div>

<style>
  .settings-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0);
    z-index: 900;
    transition: background 0.2s ease;
  }

  .settings-overlay.visible {
    background: rgba(0, 0, 0, 0.3);
  }

  .settings-panel {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 320px;
    max-width: 85vw;
    background: rgba(25, 25, 35, 0.97);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    transform: translateX(100%);
    transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
  }

  .settings-panel.visible {
    transform: translateX(0);
  }

  .settings-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    height: 44px;
  }

  .settings-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }

  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .settings-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  .settings-section {
    margin-bottom: 4px;
  }

  .section-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 10px;
  }

  .section-content {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .divider {
    height: 1px;
    background: var(--border);
    margin: 14px 0;
  }

  /* Google Calendar */
  .status-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #666;
    transition: background 0.2s;
  }

  .status-dot.connected {
    background: #81c784;
  }

  .status-text {
    font-size: 13px;
    color: var(--text-primary);
  }

  .settings-btn {
    padding: 8px 14px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    border: none;
    transition: background 0.15s, opacity 0.15s;
    width: fit-content;
  }

  .settings-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .settings-btn--primary {
    background: var(--accent);
    color: #0d1117;
  }

  .settings-btn--primary:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .settings-btn--danger {
    background: rgba(220, 60, 60, 0.7);
    color: #fff;
  }

  .settings-btn--danger:hover:not(:disabled) {
    background: rgba(220, 60, 60, 0.9);
  }

  /* Opacity Slider */
  .slider-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .range-slider {
    flex: 1;
    -webkit-appearance: none;
    appearance: none;
    height: 4px;
    background: var(--bg-hover);
    border-radius: 2px;
    outline: none;
    cursor: pointer;
  }

  .range-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--accent);
    cursor: pointer;
    border: 2px solid rgba(0, 0, 0, 0.3);
    transition: transform 0.1s;
  }

  .range-slider::-webkit-slider-thumb:hover {
    transform: scale(1.15);
  }

  .slider-value {
    font-size: 13px;
    color: var(--text-secondary);
    min-width: 40px;
    text-align: right;
  }

  /* Toggle Switch */
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
    padding: 4px 0;
  }

  .toggle-row.loading {
    pointer-events: none;
    opacity: 0.6;
  }

  .toggle-label {
    font-size: 13px;
    color: var(--text-primary);
  }

  .toggle-switch {
    width: 40px;
    height: 22px;
    border-radius: 11px;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    position: relative;
    transition: background 0.2s, border-color 0.2s;
    flex-shrink: 0;
  }

  .toggle-switch.active {
    background: var(--accent);
    border-color: var(--accent);
  }

  .toggle-knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    transition: transform 0.2s;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  }

  .toggle-switch.active .toggle-knob {
    transform: translateX(18px);
  }

  /* Color Swatches */
  .color-swatches {
    display: flex;
    gap: 10px;
  }

  .color-swatch {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.15s, border-color 0.15s, box-shadow 0.15s;
  }

  .color-swatch:hover {
    transform: scale(1.1);
  }

  .color-swatch.active {
    border-color: #fff;
    box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.2);
  }
</style>
