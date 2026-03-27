<script lang="ts">
  import type { CalendarEvent } from '../types';

  interface Props {
    mode: 'add' | 'delete';
    date: Date;
    event?: CalendarEvent;
    onConfirm: (data?: { title: string; startTime: string; endTime: string }) => void;
    onCancel: () => void;
  }

  let { mode, date, event, onConfirm, onCancel }: Props = $props();

  let title = $state('');
  let startTime = $state('09:00');
  let endTime = $state('10:00');

  function formatDateDisplay(d: Date): string {
    const year = d.getFullYear();
    const month = d.getMonth() + 1;
    const day = d.getDate();
    const weekdays = ['일', '월', '화', '수', '목', '금', '토'];
    const wd = weekdays[d.getDay()];
    return `${year}년 ${month}월 ${day}일 (${wd})`;
  }

  function handleSubmit() {
    if (mode === 'add') {
      if (!title.trim()) return;
      const y = date.getFullYear();
      const m = date.getMonth();
      const d = date.getDate();
      const [sh, sm] = startTime.split(':').map(Number);
      const [eh, em] = endTime.split(':').map(Number);
      onConfirm({
        title: title.trim(),
        startTime: new Date(y, m, d, sh, sm).toISOString(),
        endTime: new Date(y, m, d, eh, em).toISOString(),
      });
    } else {
      onConfirm();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onCancel();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onCancel();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="modal-overlay" onclick={handleBackdropClick}>
  <div class="modal-card">
    {#if mode === 'add'}
      <h3 class="modal-title">새 일정 추가</h3>
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
        <div class="form-group">
          <!-- svelte-ignore a11y_label_has_associated_control -->
          <label class="form-label">날짜</label>
          <div class="form-date">{formatDateDisplay(date)}</div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label class="form-label" for="start-time">시작 시간</label>
            <input
              id="start-time"
              class="form-input"
              type="time"
              bind:value={startTime}
            />
          </div>
          <div class="form-group">
            <label class="form-label" for="end-time">종료 시간</label>
            <input
              id="end-time"
              class="form-input"
              type="time"
              bind:value={endTime}
            />
          </div>
        </div>
        <div class="modal-actions">
          <button type="button" class="btn btn-cancel" onclick={onCancel}>취소</button>
          <button type="submit" class="btn btn-confirm">추가</button>
        </div>
      </form>
    {:else}
      <h3 class="modal-title">일정 삭제</h3>
      <p class="modal-message">'{event?.title}' 일정을 삭제하시겠습니까?</p>
      <div class="modal-actions">
        <button type="button" class="btn btn-cancel" onclick={onCancel}>취소</button>
        <button type="button" class="btn btn-danger" onclick={handleSubmit}>삭제</button>
      </div>
    {/if}
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
    width: 360px;
    max-width: 90vw;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
  }

  .modal-title {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 16px;
  }

  .modal-message {
    font-size: 14px;
    color: var(--text-primary);
    margin-bottom: 20px;
    line-height: 1.5;
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

  .form-date {
    font-size: 13px;
    color: var(--text-primary);
    padding: 8px 10px;
    background: var(--bg-secondary);
    border-radius: 8px;
    border: 1px solid var(--border);
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
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
