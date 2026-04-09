<script lang="ts">
  import type { CalendarEvent } from '../types';

  interface Props {
    mode: 'add' | 'edit';
    date: Date;
    event?: CalendarEvent;
    onConfirm: (data?: { title: string; startTime: string; endTime: string; isAllDay: boolean; color: string }) => void;
    onDelete?: () => void;
    onCancel: () => void;
  }

  let { mode, date, event, onConfirm, onDelete, onCancel }: Props = $props();

  const WEEKDAYS = ['일', '월', '화', '수', '목', '금', '토'];

  const COLOR_PALETTE = [
    '#039BE5', // Peacock (blue, default)
    '#33B679', // Sage (green)
    '#F4511E', // Tangerine (orange)
    '#D50000', // Tomato (red)
    '#8E24AA', // Grape (purple)
    '#F6BF26', // Banana (yellow)
    '#0B8043', // Basil (teal)
    '#E67C73', // Flamingo (pink)
    '#616161', // Graphite (gray)
    '#7986CB', // Lavender
    '#3F51B5', // Blueberry
  ];

  // Compute initial values from event (edit mode) or defaults (add mode)
  function initFromEvent() {
    let _title = '';
    let _color = COLOR_PALETTE[0];
    let _isAllDay = false;
    let _startTime = '09:00';
    let _endTime = '10:00';
    let _startDate = new Date(date.getFullYear(), date.getMonth(), date.getDate());
    let _endDate = new Date(date.getFullYear(), date.getMonth(), date.getDate());

    if (event && mode === 'edit') {
      _title = event.title;
      _color = event.color ?? COLOR_PALETTE[0];
      _isAllDay = event.isAllDay;

      if (event.startTime.length === 10) {
        const [sy, sm, sd] = event.startTime.split('-').map(Number);
        _startDate = new Date(sy, sm - 1, sd);
      } else {
        const s = new Date(event.startTime);
        _startDate = new Date(s.getFullYear(), s.getMonth(), s.getDate());
        _startTime = `${String(s.getHours()).padStart(2, '0')}:${String(s.getMinutes()).padStart(2, '0')}`;
      }

      if (event.endTime.length === 10) {
        const [ey, em, ed] = event.endTime.split('-').map(Number);
        let end = new Date(ey, em - 1, ed);
        if (event.isAllDay && end > _startDate) {
          end = new Date(end.getFullYear(), end.getMonth(), end.getDate() - 1);
        }
        _endDate = end;
      } else {
        const e = new Date(event.endTime);
        _endDate = new Date(e.getFullYear(), e.getMonth(), e.getDate());
        _endTime = `${String(e.getHours()).padStart(2, '0')}:${String(e.getMinutes()).padStart(2, '0')}`;
      }
    }

    return { _title, _color, _isAllDay, _startTime, _endTime, _startDate, _endDate };
  }

  const init = initFromEvent();

  let title = $state(init._title);
  let selectedColor = $state(init._color);
  let isAllDay = $state(init._isAllDay);
  let startTime = $state(init._startTime);
  let endTime = $state(init._endTime);

  let startDate = $state(init._startDate);
  let endDate = $state(init._endDate);
  let pickerOpen = $state(false);
  let pickerMonth = $state(new Date(init._startDate.getFullYear(), init._startDate.getMonth(), 1));
  let selectingEnd = $state(false);
  let hoverDate = $state<Date | null>(null);

  function isSameDay(a: Date, b: Date): boolean {
    return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
  }

  let isMultiDay = $derived(!isSameDay(startDate, endDate));

  // Auto-enable all-day when multi-day is selected
  $effect(() => {
    if (isMultiDay) {
      isAllDay = true;
    }
  });

  // Preview range while hovering during end-date selection
  let displayStart = $derived.by(() => {
    if (selectingEnd && hoverDate && hoverDate < startDate) return hoverDate;
    return startDate;
  });

  let displayEnd = $derived.by(() => {
    if (selectingEnd && hoverDate) {
      return hoverDate >= startDate ? hoverDate : startDate;
    }
    return endDate;
  });

  let pickerDays = $derived.by(() => {
    const year = pickerMonth.getFullYear();
    const month = pickerMonth.getMonth();
    const first = new Date(year, month, 1);
    const dow = first.getDay();
    const gridStart = new Date(year, month, 1 - dow);

    const days: Array<{
      date: Date;
      currentMonth: boolean;
      inRange: boolean;
      isStart: boolean;
      isEnd: boolean;
    }> = [];

    const s = displayStart;
    const e = displayEnd;

    for (let i = 0; i < 42; i++) {
      const d = new Date(gridStart.getFullYear(), gridStart.getMonth(), gridStart.getDate() + i);
      days.push({
        date: d,
        currentMonth: d.getMonth() === month,
        inRange: d >= s && d <= e,
        isStart: isSameDay(d, s),
        isEnd: isSameDay(d, e),
      });
    }
    return days;
  });

  function handlePickerClick(d: Date) {
    if (!selectingEnd) {
      startDate = new Date(d);
      endDate = new Date(d);
      selectingEnd = true;
    } else {
      if (d < startDate) {
        endDate = new Date(startDate);
        startDate = new Date(d);
      } else {
        endDate = new Date(d);
      }
      selectingEnd = false;
      hoverDate = null;
    }
  }

  function formatDateShort(d: Date): string {
    const weekdays = ['일', '월', '화', '수', '목', '금', '토'];
    return `${d.getFullYear()}년 ${d.getMonth() + 1}월 ${d.getDate()}일 (${weekdays[d.getDay()]})`;
  }

  function fmtDate(d: Date): string {
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  }

  function handleSubmit() {
    if (!title.trim()) return;

    if (isMultiDay || isAllDay) {
      const endNext = new Date(endDate.getFullYear(), endDate.getMonth(), endDate.getDate() + 1);
      onConfirm({
        title: title.trim(),
        startTime: fmtDate(startDate),
        endTime: fmtDate(endNext),
        isAllDay: true,
        color: selectedColor,
      });
    } else {
      const y = startDate.getFullYear();
      const m = startDate.getMonth();
      const d = startDate.getDate();
      const [sh, sm] = startTime.split(':').map(Number);
      const [eh, em] = endTime.split(':').map(Number);
      onConfirm({
        title: title.trim(),
        startTime: new Date(y, m, d, sh, sm).toISOString(),
        endTime: new Date(y, m, d, eh, em).toISOString(),
        isAllDay: false,
        color: selectedColor,
      });
    }
  }

  function handleDelete() {
    onDelete?.();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onCancel();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onCancel();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="modal-overlay" onclick={handleBackdropClick}>
  <div class="modal-card">
    <h3 class="modal-title">{mode === 'add' ? '새 일정 추가' : '일정 편집'}</h3>
    <form class="modal-form" onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
      <div class="form-group">
        <label class="form-label" for="event-title">제목</label>
        <input
          id="event-title"
          class="form-input"
          type="text"
          bind:value={title}
          placeholder="일정 제목을 입력하세요"
          required
        />
      </div>

      <!-- Date Range -->
      <div class="form-group">
        <label class="form-label">날짜</label>
        <!-- svelte-ignore a11y_consider_explicit_label -->
        <button type="button" class="date-btn" onclick={() => pickerOpen = !pickerOpen}>
          <span>
            {#if isMultiDay}
              {formatDateShort(startDate)} ~ {formatDateShort(endDate)}
            {:else}
              {formatDateShort(startDate)}
            {/if}
          </span>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points={pickerOpen ? "18 15 12 9 6 15" : "6 9 12 15 18 9"} />
          </svg>
        </button>
      </div>

      {#if pickerOpen}
        <div class="picker">
          <div class="picker-header">
            <button type="button" class="picker-nav" onclick={() => pickerMonth = new Date(pickerMonth.getFullYear(), pickerMonth.getMonth() - 1, 1)}>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="15 18 9 12 15 6" /></svg>
            </button>
            <span class="picker-label">{pickerMonth.getFullYear()}년 {pickerMonth.getMonth() + 1}월</span>
            <button type="button" class="picker-nav" onclick={() => pickerMonth = new Date(pickerMonth.getFullYear(), pickerMonth.getMonth() + 1, 1)}>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 6 15 12 9 18" /></svg>
            </button>
          </div>
          <div class="picker-weekdays">
            {#each WEEKDAYS as wd, i}
              <span class="picker-wd" class:sunday={i === 0} class:saturday={i === 6}>{wd}</span>
            {/each}
          </div>
          <div class="picker-grid">
            {#each pickerDays as pd}
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <button type="button"
                class="picker-day"
                class:other={!pd.currentMonth}
                class:in-range={pd.inRange && !pd.isStart && !pd.isEnd}
                class:range-start={pd.isStart}
                class:range-end={pd.isEnd}
                class:range-single={pd.isStart && pd.isEnd}
                onclick={() => handlePickerClick(pd.date)}
                onmouseenter={() => { if (selectingEnd) hoverDate = pd.date; }}
                onmouseleave={() => { if (selectingEnd) hoverDate = null; }}
              >
                {pd.date.getDate()}
              </button>
            {/each}
          </div>
          {#if selectingEnd}
            <p class="picker-hint">종료일을 선택하세요</p>
          {/if}
        </div>
      {/if}

      <!-- Color picker -->
      <div class="form-group">
        <label class="form-label">색상</label>
        <div class="color-palette">
          {#each COLOR_PALETTE as color}
            <button
              type="button"
              class="color-swatch"
              class:selected={selectedColor === color}
              style:background={color}
              onclick={() => selectedColor = color}
              aria-label="색상 선택"
            ></button>
          {/each}
        </div>
      </div>

      <!-- All-day toggle + Time inputs -->
      {#if isMultiDay}
        <p class="allday-note">여러 날 일정은 종일 일정으로 추가됩니다</p>
      {:else}
        <div class="allday-row">
          <span class="allday-label">종일</span>
          <button type="button" class="toggle-switch" class:active={isAllDay} onclick={() => { isAllDay = !isAllDay; }}>
            <div class="toggle-knob"></div>
          </button>
        </div>
        {#if !isAllDay}
          <div class="form-row">
            <div class="form-group">
              <label class="form-label" for="start-time">시작 시간</label>
              <input id="start-time" class="form-input" type="time" bind:value={startTime} />
            </div>
            <div class="form-group">
              <label class="form-label" for="end-time">종료 시간</label>
              <input id="end-time" class="form-input" type="time" bind:value={endTime} />
            </div>
          </div>
        {/if}
      {/if}

      <div class="modal-actions">
        {#if mode === 'edit'}
          <button
            type="button"
            class="btn btn-danger"
            onclick={handleDelete}
          >
            삭제
          </button>
          <div class="actions-spacer"></div>
        {/if}
        <button type="button" class="btn btn-cancel" onclick={onCancel}>취소</button>
        <button type="submit" class="btn btn-confirm">{mode === 'add' ? '추가' : '수정'}</button>
      </div>
    </form>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    z-index: 1000;
  }

  .modal-card {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 24px;
    width: 380px;
    max-width: 90vw;
    max-height: 85vh;
    overflow-y: auto;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
  }

  .modal-title {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 16px;
  }

  .modal-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .form-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .form-input {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    font-size: 13px;
    color: var(--text-primary);
    font-family: inherit;
    outline: none;
    transition: border-color 0.15s;
  }

  .form-input:focus {
    border-color: var(--accent);
  }

  .form-input::placeholder {
    color: var(--text-secondary);
    opacity: 0.6;
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  /* Date button */
  .date-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 8px 10px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 13px;
    color: var(--text-primary);
    font-family: inherit;
    cursor: pointer;
    transition: border-color 0.15s;
    text-align: left;
  }

  .date-btn:hover {
    border-color: var(--accent);
  }

  /* Mini Calendar Picker */
  .picker {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px;
  }

  .picker-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .picker-nav {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    background: none;
    border: none;
    transition: background 0.15s, color 0.15s;
  }

  .picker-nav:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .picker-label {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .picker-weekdays {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    margin-bottom: 2px;
  }

  .picker-wd {
    text-align: center;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
    padding: 2px 0;
  }

  .picker-wd.sunday { color: #ef9a9a; }
  .picker-wd.saturday { color: #90caf9; }

  .picker-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 0;
  }

  .picker-day {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 32px;
    font-size: 12px;
    color: var(--text-primary);
    background: none;
    border: none;
    cursor: pointer;
    font-family: inherit;
    transition: background 0.1s;
    border-radius: 0;
  }

  .picker-day:hover {
    background: var(--bg-hover);
  }

  .picker-day.other {
    color: var(--text-secondary);
    opacity: 0.35;
  }

  .picker-day.in-range {
    background: rgba(79, 195, 247, 0.12);
  }

  .picker-day.range-start {
    background: var(--accent);
    color: #0d1117;
    font-weight: 600;
    border-radius: 6px 0 0 6px;
  }

  .picker-day.range-end {
    background: var(--accent);
    color: #0d1117;
    font-weight: 600;
    border-radius: 0 6px 6px 0;
  }

  .picker-day.range-single {
    border-radius: 6px;
  }

  .picker-hint {
    text-align: center;
    font-size: 11px;
    color: var(--accent);
    margin: 6px 0 0;
  }

  .color-palette {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .color-swatch {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    transition: border-color 0.15s, transform 0.15s;
    padding: 0;
  }

  .color-swatch:hover {
    transform: scale(1.15);
  }

  .color-swatch.selected {
    border-color: var(--text-primary);
    transform: scale(1.15);
  }

  /* All-day toggle */
  .allday-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 0;
  }

  .allday-label {
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
    cursor: pointer;
    padding: 0;
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

  .allday-note {
    font-size: 12px;
    color: var(--text-secondary);
    text-align: center;
    padding: 4px 0;
    margin: 0;
  }

  .modal-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }

  .actions-spacer {
    flex: 1;
  }

  .btn {
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    border: none;
    transition: background 0.15s, color 0.15s;
  }

  .btn-cancel {
    background: var(--bg-secondary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .btn-cancel:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .btn-confirm {
    background: var(--accent);
    color: #0d1117;
  }

  .btn-confirm:hover {
    background: #5dcfff;
  }

  .btn-danger {
    background: rgba(220, 60, 60, 0.8);
    color: #fff;
  }

  .btn-danger:hover {
    background: rgba(220, 60, 60, 1);
  }
</style>
