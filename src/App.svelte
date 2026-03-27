<script lang="ts">
  import { onMount } from 'svelte';
  import TitleBar from './lib/components/TitleBar.svelte';
  import Calendar from './lib/components/Calendar.svelte';
  import Settings from './lib/components/Settings.svelte';
  import { calendarStore } from './lib/stores/calendar.svelte';

  let settingsOpen = $state(false);

  onMount(() => {
    calendarStore.load();
  });

  function toggleSettings() {
    settingsOpen = !settingsOpen;
  }

  function closeSettings() {
    settingsOpen = false;
  }
</script>

<main class="app-shell">
  <TitleBar onToggleSettings={toggleSettings} />
  <Calendar />
</main>

{#if settingsOpen}
  <Settings onClose={closeSettings} />
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
</style>
