<script lang="ts">
  import type { CalendarEvent, CalendarDay, DayEvent } from '../types';
  import { calendarStore } from '../stores/calendar.svelte';
  import EventModal from './EventModal.svelte';

  const WEEKDAYS = ['일', '월', '화', '수', '목', '금', '토'];

  const MONTH_NAMES = [
    'January', 'February', 'March', 'April', 'May', 'June',
    'July', 'August', 'September', 'October', 'November', 'December',
  ];

  const MAX_LANES = 3;
  const DRAG_THRESHOLD = 5;

  let headerLabel = $derived(
    `${MONTH_NAMES[calendarStore.currentDate.getMonth()]} ${calendarStore.currentDate.getFullYear()}`
  );

  // Modal state
  let modalOpen = $state(false);
  let modalMode = $state<'add' | 'edit'>('add');
  let modalDate = $state(new Date());
  let modalEvent = $state<CalendarEvent | undefined>(undefined);

  // Drag state
  let dragActive = $state(false);
  let dragEventId = $state<string | null>(null);
  let dragOriginDate = $state<Date | null>(null);
  let dragTargetDay = $state<CalendarDay | null>(null);
  let pendingDragStartX = 0;
  let pendingDragStartY = 0;
  let pendingDragEventId: string | null = null;
  let pendingDragOriginDate: Date | null = null;

  function isSameDay(a: Date, b: Date): boolean {
    return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
  }

  // --- Unified week-based layout ---
  interface SpanEvent {
    event: CalendarEvent;
    startCol: number;
    endCol: number;
    lane: number;
    roundLeft: boolean;
    roundRight: boolean;
    isSingle: boolean;
  }

  interface WeekRow {
    days: CalendarDay[];
    spans: SpanEvent[];
    laneCount: number;
    overflow: number[];
  }

  let weeks = $derived.by(() => {
    const allDays = calendarStore.calendarDays;
    const result: WeekRow[] = [];

    for (let w = 0; w < 6; w++) {
      const weekDays = allDays.slice(w * 7, w * 7 + 7);
      const spans: SpanEvent[] = [];
      const seen = new Set<string>();

      // 1) Multi-day spanning events
      for (let col = 0; col < 7; col++) {
        for (const de of weekDays[col].events) {
          if (de.position === 'single') continue;
          if (seen.has(de.event.id)) continue;
          seen.add(de.event.id);

          let endCol = col;
          for (let c = col + 1; c < 7; c++) {
            if (weekDays[c].events.some(d => d.event.id === de.event.id)) {
              endCol = c;
            } else {
              break;
            }
          }

          const startDE = weekDays[col].events.find(d => d.event.id === de.event.id);
          const endDE = weekDays[endCol].events.find(d => d.event.id === de.event.id);

          spans.push({
            event: de.event,
            startCol: col,
            endCol,
            lane: -1,
            roundLeft: startDE?.position === 'start',
            roundRight: endDE?.position === 'end',
            isSingle: false,
          });
        }
      }

      // 2) Single-day events
      for (let col = 0; col < 7; col++) {
        for (const de of weekDays[col].events) {
          if (de.position !== 'single') continue;
          spans.push({
            event: de.event,
            startCol: col,
            endCol: col,
            lane: -1,
            roundLeft: true,
            roundRight: true,
            isSingle: true,
          });
        }
      }

      // Sort: multi-day first, then by startCol, then longer first
      spans.sort((a, b) => {
        if (a.isSingle !== b.isSingle) return a.isSingle ? 1 : -1;
        if (a.startCol !== b.startCol) return a.startCol - b.startCol;
        return (b.endCol - b.startCol) - (a.endCol - a.startCol);
      });

      // Greedy lane assignment
      const laneEnds: number[] = [];
      for (const span of spans) {
        let assigned = -1;
        for (let l = 0; l < laneEnds.length; l++) {
          if (span.startCol > laneEnds[l]) {
            assigned = l;
            laneEnds[l] = span.endCol;
            break;
          }
        }
        if (assigned === -1) {
          assigned = laneEnds.length;
          laneEnds.push(span.endCol);
        }
        span.lane = assigned;
      }

      // Count overflow per column
      const overflow: number[] = Array(7).fill(0);
      for (const span of spans) {
        if (span.lane >= MAX_LANES) {
          for (let c = span.startCol; c <= span.endCol; c++) {
            overflow[c]++;
          }
        }
      }

      const visibleSpans = spans.filter(s => s.lane < MAX_LANES);
      const laneCount = Math.min(laneEnds.length, MAX_LANES);
      result.push({ days: weekDays, spans: visibleSpans, laneCount, overflow });
    }

    return result;
  });

  // --- Handlers ---
  function handleDayClick(day: CalendarDay) {
    if (dragActive) return;
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
    if (dragActive) return;
    const target = e.target as HTMLElement;
    if (target.closest('.span-bar')) return;
    calendarStore.selectDate(day.date);
    modalMode = 'add';
    modalDate = day.date;
    modalEvent = undefined;
    modalOpen = true;
  }

  function handleSpanDblClick(event: CalendarEvent, weekDays: CalendarDay[], startCol: number, e: MouseEvent) {
    if (dragActive) return;
    e.stopPropagation();
    const day = weekDays[startCol];
    calendarStore.selectDate(day.date);
    modalMode = 'edit';
    modalDate = day.date;
    modalEvent = event;
    modalOpen = true;
  }

  // --- Drag & Drop ---
  function handleBarMouseDown(eventId: string, weekDays: CalendarDay[], e: MouseEvent) {
    if (e.button !== 0) return;
    e.preventDefault();

    // Calculate which day column the mouse is on
    const grid = (e.currentTarget as HTMLElement).parentElement!;
    const rect = grid.getBoundingClientRect();
    const colWidth = rect.width / 7;
    const col = Math.min(6, Math.max(0, Math.floor((e.clientX - rect.left) / colWidth)));

    pendingDragStartX = e.clientX;
    pendingDragStartY = e.clientY;
    pendingDragEventId = eventId;
    pendingDragOriginDate = weekDays[col].date;
  }

  function handleWindowMouseMove(e: MouseEvent) {
    if (!pendingDragEventId) return;

    if (!dragActive) {
      const dx = Math.abs(e.clientX - pendingDragStartX);
      const dy = Math.abs(e.clientY - pendingDragStartY);
      if (dx + dy > DRAG_THRESHOLD) {
        dragActive = true;
        dragEventId = pendingDragEventId;
        dragOriginDate = pendingDragOriginDate;
      }
    }
  }

  function handleDayEnter(day: CalendarDay) {
    if (dragActive) {
      dragTargetDay = day;
    }
  }

  function handleWindowMouseUp() {
    if (dragActive && dragTargetDay && dragEventId && dragOriginDate) {
      const originTime = new Date(dragOriginDate.getFullYear(), dragOriginDate.getMonth(), dragOriginDate.getDate()).getTime();
      const targetTime = new Date(dragTargetDay.date.getFullYear(), dragTargetDay.date.getMonth(), dragTargetDay.date.getDate()).getTime();
      const dayOffset = Math.round((targetTime - originTime) / (1000 * 60 * 60 * 24));
      if (dayOffset !== 0) {
        calendarStore.moveEvent(dragEventId, dayOffset);
      }
    }
    resetDrag();
  }

  function resetDrag() {
    dragActive = false;
    dragEventId = null;
    dragOriginDate = null;
    dragTargetDay = null;
    pendingDragEventId = null;
    pendingDragOriginDate = null;
  }

  function handleModalConfirm(data?: { title: string; startTime: string; endTime: string; isAllDay: boolean; color: string }) {
    if (modalMode === 'add' && data) {
      calendarStore.addEvent({
        id: calendarStore.generateEventId(),
        title: data.title,
        startTime: data.startTime,
        endTime: data.endTime,
        isAllDay: data.isAllDay,
        color: data.color,
        source: calendarStore.googleConnected ? 'google' : 'local',
      });
    } else if (modalMode === 'edit' && modalEvent && data) {
      calendarStore.updateEvent(modalEvent.id, data);
    }
    modalOpen = false;
  }

  function handleModalDelete() {
    if (modalEvent) {
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

<svelte:window onmousemove={handleWindowMouseMove} onmouseup={handleWindowMouseUp} />

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

  <div class="calendar-body" class:dragging={dragActive}>
    {#each weeks as week}
      <div class="week-row">
        <!-- Day number row -->
        <div class="week-date-row">
          {#each week.days as day}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div
              class="date-cell"
              class:other-month={!day.isCurrentMonth}
              class:today={day.isToday}
              class:selected={day.isSelected}
              class:drag-target={dragActive && dragTargetDay && isSameDay(dragTargetDay.date, day.date)}
              onclick={() => handleDayClick(day)}
              ondblclick={(e) => handleCellDblClick(day, e)}
              onmouseenter={() => handleDayEnter(day)}
            >
              <span
                class="day-number"
                class:sunday={getDayOfWeekIndex(day) === 0}
                class:saturday={getDayOfWeekIndex(day) === 6}
                class:today-number={day.isToday}
              >{day.date.getDate()}</span>
            </div>
          {/each}
        </div>

        <!-- Unified event grid (multi-day + single-day) -->
        <div class="week-events" style="grid-template-rows: {week.laneCount > 0 ? `repeat(${week.laneCount}, 18px) 1fr` : '1fr'};">
          <!-- Clickable background cells -->
          {#each week.days as day, col}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div
              class="event-bg-cell"
              style="grid-column: {col + 1}; grid-row: 1 / -1;"
              onclick={() => handleDayClick(day)}
              ondblclick={(e) => handleCellDblClick(day, e)}
              onmouseenter={() => handleDayEnter(day)}
            ></div>
          {/each}
          <!-- All event bars -->
          {#each week.spans as span}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div
              class="span-bar"
              class:round-left={span.roundLeft}
              class:round-right={span.roundRight}
              class:single={span.isSingle}
              class:drag-source={dragActive && dragEventId === span.event.id}
              style="grid-column: {span.startCol + 1} / {span.endCol + 2}; grid-row: {span.lane + 1};"
              style:background={span.event.color ?? 'var(--accent)'}
              onmousedown={(e) => handleBarMouseDown(span.event.id, week.days, e)}
              ondblclick={(e) => handleSpanDblClick(span.event, week.days, span.startCol, e)}
              title={span.event.title}
            >
              <span class="span-bar-title">{span.event.title}</span>
            </div>
          {/each}
        </div>

        <!-- Overflow indicators -->
        {#if week.overflow.some(n => n > 0)}
          <div class="week-overflow">
            {#each week.overflow as count, col}
              <div class="overflow-cell">
                {#if count > 0}
                  <span class="more-events">+{count} more</span>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
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
    onDelete={handleModalDelete}
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

  .weekday.sunday { color: #ef9a9a; }
  .weekday.saturday { color: #90caf9; }

  /* Week-based layout */
  .calendar-body {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .calendar-body.dragging {
    cursor: grabbing;
    user-select: none;
  }

  .week-row {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  /* Date number row */
  .week-date-row {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    flex-shrink: 0;
  }

  .date-cell {
    padding: 3px 4px 1px;
    cursor: pointer;
    transition: background 0.12s;
    border-radius: 6px;
    border: 1px solid transparent;
  }

  .date-cell:hover {
    background: var(--bg-hover);
  }

  .date-cell.selected {
    border-color: rgba(79, 195, 247, 0.4);
    background: var(--accent-dim);
  }

  .date-cell.other-month {
    opacity: 0.35;
  }

  .date-cell.drag-target {
    background: rgba(79, 195, 247, 0.2);
    border-color: rgba(79, 195, 247, 0.5);
  }

  .day-number {
    font-size: 12px;
    font-weight: 400;
    line-height: 1;
    padding: 2px 4px;
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

  .day-number.sunday { color: #ef9a9a; }
  .day-number.saturday { color: #90caf9; }

  .day-number.today-number.sunday,
  .day-number.today-number.saturday {
    color: #0d1117;
  }

  /* Unified event grid */
  .week-events {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 1px 0;
    flex: 1;
    min-height: 0;
    padding: 1px 0;
  }

  .event-bg-cell {
    cursor: pointer;
  }

  .span-bar {
    display: flex;
    align-items: center;
    padding: 1px 6px;
    cursor: grab;
    border-radius: 0;
    transition: filter 0.12s, opacity 0.12s;
    min-width: 0;
    overflow: hidden;
    z-index: 1;
  }

  /* During drag, make bars pass-through so mouseenter fires on bg-cells */
  .calendar-body.dragging .span-bar {
    pointer-events: none;
  }

  .span-bar.drag-source {
    opacity: 0.4;
  }

  .span-bar.round-left {
    border-top-left-radius: 4px;
    border-bottom-left-radius: 4px;
    margin-left: 2px;
  }

  .span-bar.round-right {
    border-top-right-radius: 4px;
    border-bottom-right-radius: 4px;
    margin-right: 2px;
  }

  .span-bar.single {
    border-radius: 4px;
    margin: 0 2px;
  }

  .span-bar:hover {
    filter: brightness(1.2);
  }

  .span-bar-title {
    font-size: 10px;
    font-weight: 500;
    color: #0d1117;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.3;
  }

  /* Overflow row */
  .week-overflow {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    flex-shrink: 0;
  }

  .overflow-cell {
    padding: 0 4px;
  }

  .more-events {
    font-size: 10px;
    color: var(--text-secondary);
  }
</style>
