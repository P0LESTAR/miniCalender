import { invoke } from '@tauri-apps/api/core';
import type { CalendarEvent, CalendarDay } from '../types';

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
        const eventDate = new Date(e.startTime);
        return isSameDay(eventDate, this.selectedDate);
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

    const startDate = new Date(year, month, 1 - startDayOfWeek);

    const eventMap = new Map<string, CalendarEvent[]>();
    for (const ev of this.events) {
      const key = toDateKey(new Date(ev.startTime));
      const arr = eventMap.get(key);
      if (arr) {
        arr.push(ev);
      } else {
        eventMap.set(key, [ev]);
      }
    }

    const days: CalendarDay[] = [];
    const now = new Date();

    for (let i = 0; i < 42; i++) {
      const date = new Date(startDate.getFullYear(), startDate.getMonth(), startDate.getDate() + i);
      const key = toDateKey(date);
      days.push({
        date,
        isCurrentMonth: date.getMonth() === month,
        isToday: isSameDay(date, now),
        isSelected: isSameDay(date, this.selectedDate),
        events: eventMap.get(key) ?? [],
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
        // Add the Google-created event (with real Google ID)
        this.events.push(fromRustEvent(created));
      } catch (e) {
        console.error('[calendar] Failed to create Google event:', e);
        // Fallback to local
        event.source = 'local';
        this.events.push(event);
        this.persistLocal();
      }
    } else {
      console.log('[calendar] saving locally (no Google)');
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
    const endDate = new Date(year, month + 2, 0); // last day of month+1

    try {
      const googleEvents: any[] = await invoke('fetch_events', {
        startDate: startDate.toISOString(),
        endDate: endDate.toISOString(),
      });

      // Remove old google events, keep local events
      const localEvents = this.events.filter((e) => e.source === 'local');
      const mapped = googleEvents.map(fromRustEvent);
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
        this.events = parsed;
      }
    } catch (e) {
      console.warn('Failed to load local events:', e);
    }
    this.loaded = true;

    // Check Google auth and fetch if connected
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
  }
}

export const calendarStore = new CalendarStore();
