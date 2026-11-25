const DAY = 86_400_000;

const midnight = (ms: number) => new Date(ms).setHours(0, 0, 0, 0);

export function messageTime(at: number, now: number): string {
  const time = new Date(at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const days = Math.round((midnight(now) - midnight(at)) / DAY);
  if (days === 0) return time;
  if (days === 1) return `yesterday at ${time}`;
  const sameYear = new Date(at).getFullYear() === new Date(now).getFullYear();
  const date = new Date(at).toLocaleDateString([], { month: "short", day: "numeric", ...(!sameYear && { year: "numeric" }) });
  return `${date} at ${time}`;
}
