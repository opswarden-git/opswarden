export type NotificationSound = "message" | "release-completed";

const STORAGE_KEY = "opswarden.notification-sounds";
const CHANGE_EVENT = "opswarden:notification-sounds-changed";

let audioContext: AudioContext | null = null;

export function notificationSoundsEnabled(): boolean {
  return typeof window !== "undefined" && window.localStorage.getItem(STORAGE_KEY) === "true";
}

export function setNotificationSoundsEnabled(enabled: boolean): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(STORAGE_KEY, String(enabled));
  window.dispatchEvent(new Event(CHANGE_EVENT));
}

export function subscribeToNotificationSounds(listener: () => void): () => void {
  if (typeof window === "undefined") return () => undefined;
  const onStorage = (event: StorageEvent) => {
    if (event.key === STORAGE_KEY) listener();
  };
  window.addEventListener("storage", onStorage);
  window.addEventListener(CHANGE_EVENT, listener);
  return () => {
    window.removeEventListener("storage", onStorage);
    window.removeEventListener(CHANGE_EVENT, listener);
  };
}

function context(): AudioContext | null {
  if (typeof window === "undefined" || !("AudioContext" in window)) return null;
  audioContext ??= new window.AudioContext();
  return audioContext;
}

function tone(
  audio: AudioContext,
  destination: AudioNode,
  frequency: number,
  startsAt: number,
  duration: number,
): void {
  const oscillator = audio.createOscillator();
  const gain = audio.createGain();
  oscillator.type = "sine";
  oscillator.frequency.setValueAtTime(frequency, startsAt);
  gain.gain.setValueAtTime(0.0001, startsAt);
  gain.gain.exponentialRampToValueAtTime(0.035, startsAt + 0.015);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + duration);
  oscillator.connect(gain);
  gain.connect(destination);
  oscillator.start(startsAt);
  oscillator.stop(startsAt + duration + 0.01);
}

/** A restrained two-note message cue and a three-note release completion cue. */
export async function playNotificationSound(sound: NotificationSound): Promise<boolean> {
  if (!notificationSoundsEnabled()) return false;
  try {
    const audio = context();
    if (!audio) return false;

    if (audio.state === "suspended") await audio.resume();
    const startsAt = audio.currentTime + 0.01;

    if (sound === "message") {
      tone(audio, audio.destination, 659.25, startsAt, 0.14);
      tone(audio, audio.destination, 880, startsAt + 0.055, 0.16);
    } else {
      tone(audio, audio.destination, 523.25, startsAt, 0.18);
      tone(audio, audio.destination, 659.25, startsAt + 0.08, 0.2);
      tone(audio, audio.destination, 783.99, startsAt + 0.16, 0.24);
    }
    return true;
  } catch {
    // Audio is optional and must never interrupt realtime event handling.
    return false;
  }
}
