<script lang="ts">
  import type { CalendarEvent, CalendarDay } from '../types';
  import { calendarStore } from '../stores/calendar.svelte';
  import EventModal from './EventModal.svelte';

  const WEEKDAYS = ['일', '월', '화', '수', '목', '금', '토'];

  const MONTH_NAMES = [
    'January', 'February', 'March', 'April', 'May', 'June',
    'July', 'August', 'September', 'October', 'November', 'December',
  ];

  const MAX_VISIBLE_EVENTS = 3;

  let headerLabel = $derived(
    `${MONTH_NAMES[calendarStore.currentDate.getMonth()]} ${calendarStore.currentDate.getFullYear()}`
  );

  // Modal state
  let modalOpen = $state(false);
  let modalMode = $state<'add' | 'delete'>('add');
  let modalDate = $state(new Date());
  let modalEvent = $state<CalendarEvent | undefined>(undefined);

  function handleDayClick(day: CalendarDay) {
    calendarStore.selectDate(day.date);
  }

  function handlePrev() {
    calendarStore.prevMonth();
  }

  function handleNext() {
    calendarStore.nextMonth();
  }

  function handleToday() {
    calendarStore.goToToday();
  }

  function handleCellDblClick(day: CalendarDay, e: MouseEvent) {
    // Only open add modal if we didn't click on an event bar
    const target = e.target as HTMLElement;
    if (target.closest('.event-bar')) return;
    calendarStore.selectDate(day.date);
    modalMode = 'add';
    modalDate = day.date;
    modalEvent = undefined;
    modalOpen = true;
  }

  function handleEventDblClick(event: CalendarEvent, day: CalendarDay, e: MouseEvent) {
    e.stopPropagation();
    calendarStore.selectDate(day.date);
    modalMode = 'delete';
    modalDate = day.date;
    modalEvent = event;
    modalOpen = true;
  }

  function handleModalConfirm(data?: { title: string; startTime: string; endTime: string }) {
    if (modalMode === 'add' && data) {
      calendarStore.addEvent({
        id: calendarStore.generateEventId(),
        title: data.title,
        startTime: data.startTime,
        endTime: data.endTime,
        isAllDay: false,
        source: calendarStore.googleConnected ? 'google' : 'local',
      });
    } else if (modalMode === 'delete' && modalEvent) {
      calendarStore.removeEvent(modalEvent.id);
    }
    modalOpen = false;
  }

  function handleModalCancel() {
    modalOpen = false;
  }

  function getDayOfWeekIndex(day: CalendarDay): number {
    return day.date.getDay();
  }
</script>

<section class="calendar">
  <div class="calendar-header">
    <button class="nav-btn" onclick={handlePrev} aria-label="Previous month">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="15 18 9 12 15 6" />
      </svg>
    </button>
    <button class="header-label" onclick={handleToday} title="Go to today">
      {headerLabel}
    </button>
    <button class="nav-btn" onclick={handleNext} aria-label="Next month">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="9 6 15 12 9 18" />
      </svg>
    </button>
  </div>

  <div class="weekdays">
    {#each WEEKDAYS as day, i}
      <span
        class="weekday"
        class:sunday={i === 0}
        class:saturday={i === 6}
      >{day}</span>
    {/each}
  </div>

  <div class="days-grid">
    {#each calendarStore.calendarDays as day}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        class="day-cell"
        class:other-month={!day.isCurrentMonth}
        class:today={day.isToday}
        class:selected={day.isSelected}
        onclick={() => handleDayClick(day)}
        ondblclick={(e) => handleCellDblClick(day, e)}
      >
        <span
          class="day-number"
          class:sunday={getDayOfWeekIndex(day) === 0}
          class:saturday={getDayOfWeekIndex(day) === 6}
          class:today-number={day.isToday}
        >{day.date.getDate()}</span>
        <div class="day-events">
          {#each day.events.slice(0, MAX_VISIBLE_EVENTS) as event (event.id)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="event-bar"
              style:background={event.color ?? 'var(--accent)'}
              ondblclick={(e) => handleEventDblClick(event, day, e)}
              title={event.title}
            >
              <span class="event-bar-title">{event.title}</span>
            </div>
          {/each}
          {#if day.events.length > MAX_VISIBLE_EVENTS}
            <span class="more-events">+{day.events.length - MAX_VISIBLE_EVENTS} more</span>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</section>

{#if modalOpen}
  <EventModal
    mode={modalMode}
    date={modalDate}
    event={modalEvent}
    onConfirm={handleModalConfirm}
    onCancel={handleModalCancel}
  />
{/if}

<style>
  .calendar {
    display: flex;
    flex-direction: column;
    flex: 1;
    padding: 8px 12px 8px;
    min-height: 0;
  }

  .calendar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 2px 8px;
    flex-shrink: 0;
  }

  .nav-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }

  .nav-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .header-label {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.2px;
    padding: 4px 16px;
    border-radius: 8px;
    transition: background 0.15s;
  }

  .header-label:hover {
    background: var(--bg-hover);
  }

  .weekdays {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 0;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 2px;
    flex-shrink: 0;
  }

  .weekday {
    text-align: center;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    padding: 2px 0;
  }

  .weekday.sunday {
    color: #ef9a9a;
  }

  .weekday.saturday {
    color: #90caf9;
  }

  .days-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    grid-template-rows: repeat(6, 1fr);
    gap: 1px;
    flex: 1;
    min-height: 0;
  }

  .day-cell {
    display: flex;
    flex-direction: column;
    padding: 3px 4px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.12s;
    overflow: hidden;
    min-height: 0;
    border: 1px solid transparent;
  }

  .day-cell:hover {
    background: var(--bg-hover);
  }

  .day-cell.selected {
    border-color: rgba(79, 195, 247, 0.4);
    background: var(--accent-dim);
  }

  .day-cell.other-month {
    opacity: 0.35;
  }

  .day-number {
    font-size: 12px;
    font-weight: 400;
    line-height: 1;
    padding: 2px 4px;
    margin-bottom: 2px;
    flex-shrink: 0;
    width: fit-content;
  }

  .day-number.today-number {
    background: var(--accent);
    color: #0d1117;
    font-weight: 700;
    border-radius: 50%;
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .day-number.sunday {
    color: #ef9a9a;
  }

  .day-number.saturday {
    color: #90caf9;
  }

  .day-number.today-number.sunday,
  .day-number.today-number.saturday {
    color: #0d1117;
  }

  .day-events {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-height: 0;
    overflow: hidden;
    flex: 1;
  }

  .event-bar {
    display: flex;
    align-items: center;
    padding: 1px 4px;
    border-radius: 3px;
    cursor: pointer;
    flex-shrink: 0;
    min-height: 16px;
    max-height: 18px;
    transition: filter 0.12s;
  }

  .event-bar:hover {
    filter: brightness(1.2);
  }

  .event-bar-title {
    font-size: 10px;
    font-weight: 500;
    color: #0d1117;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.3;
  }

  .more-events {
    font-size: 10px;
    color: var(--text-secondary);
    padding: 0 4px;
    flex-shrink: 0;
  }
</style>
