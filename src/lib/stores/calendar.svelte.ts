import { invoke } from '@tauri-apps/api/core';
import type { CalendarEvent, CalendarDay, DayEvent, EventPosition } from '../types';

function isSameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

function toDateKey(d: Date): string {
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}

/** Parse event time string to local midnight date */
function toLocalDate(s: string): Date {
  if (s.length === 10) {
    // "YYYY-MM-DD" → parse as local date
    const [y, m, d] = s.split('-').map(Number);
    return new Date(y, m - 1, d);
  }
  const dt = new Date(s);
  return new Date(dt.getFullYear(), dt.getMonth(), dt.getDate());
}

/** Shift a date string by N days, preserving format */
function shiftDateStr(s: string, days: number): string {
  if (s.length === 10) {
    // "YYYY-MM-DD" format
    const [y, m, d] = s.split('-').map(Number);
    const date = new Date(y, m - 1, d + days);
    return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
  }
  // ISO datetime format
  const date = new Date(s);
  date.setDate(date.getDate() + days);
  return date.toISOString();
}

/** Convert Rust snake_case event to frontend camelCase */
function fromRustEvent(e: any): CalendarEvent {
  return {
    id: e.id,
    title: e.title,
    description: e.description,
    startTime: e.start_time,
    endTime: e.end_time,
    color: e.color,
    isAllDay: e.is_all_day,
    source: e.source,
  };
}

const today = new Date();
let nextId = 100;

// --- Reactive store using Svelte 5 runes ---
class CalendarStore {
  currentDate = $state(new Date(today.getFullYear(), today.getMonth(), 1));
  selectedDate = $state(new Date(today.getFullYear(), today.getMonth(), today.getDate()));
  events = $state<CalendarEvent[]>([]);
  loaded = $state(false);
  googleConnected = $state(false);

  selectedDateEvents = $derived(
    this.events
      .filter((e) => {
        const evStart = toLocalDate(e.startTime);
        let evEnd = toLocalDate(e.endTime);
        // Google all-day events use exclusive end date
        if (e.isAllDay && evEnd > evStart) {
          evEnd = new Date(evEnd.getFullYear(), evEnd.getMonth(), evEnd.getDate() - 1);
        }
        return this.selectedDate >= evStart && this.selectedDate <= evEnd;
      })
      .sort((a, b) => {
        if (a.isAllDay && !b.isAllDay) return -1;
        if (!a.isAllDay && b.isAllDay) return 1;
        return new Date(a.startTime).getTime() - new Date(b.startTime).getTime();
      })
  );

  calendarDays = $derived.by(() => {
    const year = this.currentDate.getFullYear();
    const month = this.currentDate.getMonth();

    const firstDay = new Date(year, month, 1);
    const startDayOfWeek = firstDay.getDay();
    const gridStart = new Date(year, month, 1 - startDayOfWeek);

    // Build 42 dates
    const dates: Date[] = [];
    for (let i = 0; i < 42; i++) {
      dates.push(new Date(gridStart.getFullYear(), gridStart.getMonth(), gridStart.getDate() + i));
    }

    // Distribute events across all days they cover
    const dayEventsMap = new Map<string, DayEvent[]>();

    for (const ev of this.events) {
      const evStart = toLocalDate(ev.startTime);
      let evEnd = toLocalDate(ev.endTime);

      // Google all-day events use exclusive end date — convert to inclusive
      if (ev.isAllDay && evEnd > evStart) {
        evEnd = new Date(evEnd.getFullYear(), evEnd.getMonth(), evEnd.getDate() - 1);
      }

      const isMultiDay = !isSameDay(evStart, evEnd);

      for (const date of dates) {
        if (date >= evStart && date <= evEnd) {
          const key = toDateKey(date);
          let position: EventPosition;
          if (!isMultiDay) {
            position = 'single';
          } else if (isSameDay(date, evStart)) {
            position = 'start';
          } else if (isSameDay(date, evEnd)) {
            position = 'end';
          } else {
            position = 'middle';
          }

          const arr = dayEventsMap.get(key) ?? [];
          arr.push({ event: ev, position });
          dayEventsMap.set(key, arr);
        }
      }
    }

    // Sort: multi-day first (by start time), then single-day
    for (const [, dayEvents] of dayEventsMap) {
      dayEvents.sort((a, b) => {
        const aMulti = a.position !== 'single';
        const bMulti = b.position !== 'single';
        if (aMulti && !bMulti) return -1;
        if (!aMulti && bMulti) return 1;
        if (a.event.isAllDay && !b.event.isAllDay) return -1;
        if (!a.event.isAllDay && b.event.isAllDay) return 1;
        return new Date(a.event.startTime).getTime() - new Date(b.event.startTime).getTime();
      });
    }

    const days: CalendarDay[] = [];
    const now = new Date();

    for (const date of dates) {
      const key = toDateKey(date);
      days.push({
        date,
        isCurrentMonth: date.getMonth() === month,
        isToday: isSameDay(date, now),
        isSelected: isSameDay(date, this.selectedDate),
        events: dayEventsMap.get(key) ?? [],
      });
    }

    return days;
  });

  nextMonth() {
    const cur = this.currentDate;
    this.currentDate = new Date(cur.getFullYear(), cur.getMonth() + 1, 1);
    if (this.googleConnected) this.fetchGoogleEvents();
  }

  prevMonth() {
    const cur = this.currentDate;
    this.currentDate = new Date(cur.getFullYear(), cur.getMonth() - 1, 1);
    if (this.googleConnected) this.fetchGoogleEvents();
  }

  selectDate(date: Date) {
    this.selectedDate = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  }

  goToToday() {
    const now = new Date();
    this.currentDate = new Date(now.getFullYear(), now.getMonth(), 1);
    this.selectedDate = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    if (this.googleConnected) this.fetchGoogleEvents();
  }

  async addEvent(event: CalendarEvent) {
    console.log('[calendar] addEvent, googleConnected =', this.googleConnected, 'source =', event.source);
    if (this.googleConnected) {
      try {
        console.log('[calendar] calling create_event...');
        const created: any = await invoke('create_event', {
          event: {
            title: event.title,
            description: event.description ?? null,
            start_time: event.startTime,
            end_time: event.endTime,
            is_all_day: event.isAllDay,
          },
        });
        console.log('[calendar] create_event result:', created);
        const mapped = fromRustEvent(created);
        mapped.color = event.color;
        this.events.push(mapped);
        this.persistColorMap();
      } catch (e) {
        console.error('[calendar] Failed to create Google event:', e);
        event.source = 'local';
        this.events.push(event);
        this.persistLocal();
      }
    } else {
      this.events.push(event);
      this.persistLocal();
    }
  }

  async removeEvent(id: string) {
    const event = this.events.find((e) => e.id === id);
    if (event && event.source === 'google' && this.googleConnected) {
      try {
        await invoke('delete_event', { eventId: id });
      } catch (e) {
        console.error('Failed to delete Google event:', e);
      }
    }
    this.events = this.events.filter((e) => e.id !== id);
    this.persistLocal();
  }

  async moveEvent(id: string, dayOffset: number) {
    if (dayOffset === 0) return;
    const idx = this.events.findIndex((e) => e.id === id);
    if (idx === -1) return;

    const ev = this.events[idx];
    const newStartTime = shiftDateStr(ev.startTime, dayOffset);
    const newEndTime = shiftDateStr(ev.endTime, dayOffset);

    this.events[idx] = {
      ...ev,
      startTime: newStartTime,
      endTime: newEndTime,
    };

    if (ev.source === 'google' && this.googleConnected) {
      try {
        await invoke('update_event', {
          eventId: id,
          event: {
            title: ev.title,
            description: ev.description ?? null,
            start_time: newStartTime,
            end_time: newEndTime,
            is_all_day: ev.isAllDay,
          },
        });
      } catch (e) {
        console.error('Failed to update Google event:', e);
      }
    }

    this.persistLocal();
    this.persistColorMap();
  }

  generateEventId(): string {
    return `evt-${Date.now()}-${nextId++}`;
  }

  /** Check Google auth status */
  async checkGoogleAuth() {
    try {
      const status: boolean = await invoke('google_auth_status');
      console.log('[calendar] google_auth_status =', status);
      this.googleConnected = status;
    } catch (e) {
      console.error('[calendar] checkGoogleAuth failed:', e);
      this.googleConnected = false;
    }
  }

  /** Fetch Google Calendar events for current month ± 1 month */
  async fetchGoogleEvents() {
    if (!this.googleConnected) return;

    const year = this.currentDate.getFullYear();
    const month = this.currentDate.getMonth();

    const startDate = new Date(year, month - 1, 1);
    const endDate = new Date(year, month + 2, 0);

    try {
      const googleEvents: any[] = await invoke('fetch_events', {
        startDate: startDate.toISOString(),
        endDate: endDate.toISOString(),
      });

      const localEvents = this.events.filter((e) => e.source === 'local');
      // Restore user-selected colors from memory + localStorage
      const colorMap = this.loadColorMap();
      for (const e of this.events) {
        if (e.source === 'google' && e.color) colorMap[e.id] = e.color;
      }
      const mapped = googleEvents.map((e) => {
        const ev = fromRustEvent(e);
        if (colorMap[ev.id]) ev.color = colorMap[ev.id];
        return ev;
      });
      this.events = [...localEvents, ...mapped];
    } catch (e) {
      console.error('Failed to fetch Google events:', e);
    }
  }

  /** Load events from disk on startup */
  async load() {
    try {
      const json: string = await invoke('load_local_events');
      const parsed = JSON.parse(json) as CalendarEvent[];
      if (Array.isArray(parsed) && parsed.length > 0) {
        // Restore colors from color map
        const colorMap = this.loadColorMap();
        for (const ev of parsed) {
          if (!ev.color && colorMap[ev.id]) ev.color = colorMap[ev.id];
        }
        this.events = parsed;
      }
    } catch (e) {
      console.warn('Failed to load local events:', e);
    }
    this.loaded = true;

    await this.checkGoogleAuth();
    if (this.googleConnected) {
      await this.fetchGoogleEvents();
    }
  }

  /** Persist local events to disk */
  private async persistLocal() {
    try {
      const localEvents = this.events.filter((e) => e.source === 'local');
      await invoke('save_local_events', { eventsJson: JSON.stringify(localEvents) });
    } catch (e) {
      console.warn('Failed to save local events:', e);
    }
    this.persistColorMap();
  }

  /** Save user-selected colors for all events */
  private persistColorMap() {
    try {
      const colorMap: Record<string, string> = {};
      for (const e of this.events) {
        if (e.color) colorMap[e.id] = e.color;
      }
      localStorage.setItem('event-colors', JSON.stringify(colorMap));
    } catch {}
  }

  /** Load color map */
  private loadColorMap(): Record<string, string> {
    try {
      const data = localStorage.getItem('event-colors');
      if (data) return JSON.parse(data);
    } catch {}
    return {};
  }
}

export const calendarStore = new CalendarStore();
