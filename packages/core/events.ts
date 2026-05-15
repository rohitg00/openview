import type { EventType, JsonObject, OpenViewEvent } from "./types.js";

export interface EventStore {
  append(type: EventType, payload: JsonObject): OpenViewEvent;
  all(): OpenViewEvent[];
  after(sequence: number): OpenViewEvent[];
}

export function createEventStore(seed: OpenViewEvent[] = []): EventStore {
  const events = [...seed];
  let sequence = events.at(-1)?.sequence ?? 0;

  return {
    append(type, payload) {
      sequence += 1;
      const event: OpenViewEvent = {
        id: `evt_${sequence.toString(36)}`,
        sequence,
        type,
        timestamp: new Date().toISOString(),
        payload,
      };
      events.push(event);
      return event;
    },
    all() {
      return [...events];
    },
    after(cursor) {
      return events.filter((event) => event.sequence > cursor);
    },
  };
}
