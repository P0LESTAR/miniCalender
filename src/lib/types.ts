export interface CalendarEvent {
  id: string;
  title: string;
  description?: string;
  startTime: string; // ISO 8601 or YYYY-MM-DD for all-day
  endTime: string; // ISO 8601 or YYYY-MM-DD for all-day
  color?: string;
  isAllDay: boolean;
  source: 'google' | 'local';
}

export type EventPosition = 'single' | 'start' | 'middle' | 'end';

export interface DayEvent {
  event: CalendarEvent;
  position: EventPosition;
}

export interface CalendarDay {
  date: Date;
  isCurrentMonth: boolean;
  isToday: boolean;
  isSelected: boolean;
  events: DayEvent[];
}
