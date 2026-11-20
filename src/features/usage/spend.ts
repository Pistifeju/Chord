// Rules for the Usage panel: the ranges and the time each starts at, the ranked list of harnesses and their models, the picked row's detail, and the number texts.
import type { Spend, SpendRow } from "../../ipc/bindings";

const HOUR = 3600000;
const DAY = 86400000;

export type Range = "24h" | "7d" | "30d" | "90d";
export type Metric = "cost" | "tokens";

/** Past 24h charts hours; the other ranges chart local days. */
export const hourly = (range: Range) => range === "24h";

export const ranges: { value: Range; label: string; title: string; days: number }[] = [
  { value: "24h", label: "24h", title: "Past 24 hours", days: 1 },
  { value: "7d", label: "7d", title: "Past 7 days", days: 7 },
  { value: "30d", label: "30d", title: "Past 30 days", days: 30 },
  { value: "90d", label: "90d", title: "Past 90 days", days: 90 },
];

const startOfDay = (at: number) => new Date(at).setHours(0, 0, 0, 0);
const daysAfter = (at: number, days: number) => {
  const date = new Date(at);
  date.setDate(date.getDate() + days);
  return date.getTime();
};

/** The epoch ms a range starts at: 24 hours back, or local midnight of its first day. */
export function since(range: Range, now: number): number {
  if (hourly(range)) return now - DAY;
  const { days } = ranges.find((r) => r.value === range)!;
  return daysAfter(startOfDay(now), -(days - 1));
}

export type TokenTypes = { input: number; cacheRead: number; cacheWrite: number; output: number };

export const tokensOf = (types: { [K in keyof TokenTypes]: number | null }) => (types.input ?? 0) + (types.cacheRead ?? 0) + (types.cacheWrite ?? 0) + (types.output ?? 0);

/** The rows of a range; a read for a longer range holds the shorter ones. */
export const inRange = (spend: Spend, range: Range, now: number): Spend => {
  const first = Math.floor(since(range, now) / HOUR) * HOUR;
  return { ...spend, rows: spend.rows.filter((row) => (row.hour ?? 0) >= first) };
};

/** A sum that stays null until one of its parts is known. */
const plus = (sum: number | null, part: number | null | undefined) => (part == null ? sum : (sum ?? 0) + part);

const valueOf = (metric: Metric, tokens: number, cost: number | null) => (metric === "cost" ? cost : tokens);

/** The list's row and the inspector's pick: All agents, a harness, or one model of a harness. */
export type Pick = { harness?: string; model?: string };

export type ListRow = Pick & { key: string; value: number | null; share: number | null; rank?: number };

type Sum = { tokens: number; cost: number | null };

function sums(rows: SpendRow[], keyOf: (row: SpendRow) => string) {
  const out = new Map<string, Sum>();
  for (const row of rows) {
    const key = keyOf(row);
    const sum = out.get(key) ?? { tokens: 0, cost: null };
    sum.tokens += tokensOf(row);
    sum.cost = plus(sum.cost, row.cost);
    out.set(key, sum);
  }
  return out;
}

/** Keys by the shown value, highest first; ties, and rows with no value, by tokens. */
function ranked(map: Map<string, Sum>, metric: Metric): string[] {
  const value = (sum: Sum) => valueOf(metric, sum.tokens, sum.cost) ?? -1;
  return [...map.keys()].sort((a, b) => value(map.get(b)!) - value(map.get(a)!) || map.get(b)!.tokens - map.get(a)!.tokens);
}

/** Each harness with rows, ranked; its rank picks its color. */
export const harnessRanks = (spend: Spend, metric: Metric): string[] => ranked(sums(spend.rows, (row) => row.harness), metric);

/** All agents, then each harness ranked by the shown value, each followed by its models in the same order. A row's share is its value over the total; a row without a cost has neither in Cost mode. */
export function list(spend: Spend, metric: Metric): ListRow[] {
  const all = sums(spend.rows, () => "");
  const total = valueOf(metric, all.get("")?.tokens ?? 0, all.get("")?.cost ?? null);
  const row = (key: string, sum: Sum, pick: Pick, rank?: number): ListRow => {
    const value = valueOf(metric, sum.tokens, sum.cost);
    return { key, ...pick, value, share: value != null && total ? value / total : null, rank };
  };
  const out: ListRow[] = [row("all", all.get("") ?? { tokens: 0, cost: null }, {})];
  const harnesses = sums(spend.rows, (r) => r.harness);
  ranked(harnesses, metric).forEach((harness, rank) => {
    out.push(row(harness, harnesses.get(harness)!, { harness }, rank));
    const models = sums(
      spend.rows.filter((r) => r.harness === harness),
      (r) => r.model,
    );
    for (const model of ranked(models, metric)) out.push(row(`${harness}/${model}`, models.get(model)!, { harness, model }));
  });
  return out;
}

export type Detail = {
  /** The shown value; null in Cost mode for a pick with no cost. */
  total: number | null;
  /** Null for a model: the session counts are per harness. */
  sessions: number | null;
  /** The start of each chart point: hours for Past 24h, local days otherwise. */
  buckets: number[];
  series: { harness: string; rank: number; values: number[] }[];
  types: TokenTypes;
  figures: { processed: number; cached: number; uncached: number; output: number; saved: number | null };
  /** The three buckets with the most of the shown value, most first. */
  busiest: { at: number; value: number }[];
};

/** The picked row's numbers over a range. */
export function detail(spend: Spend, range: Range, now: number, metric: Metric, pick: Pick): Detail {
  const start = since(range, now);
  const rows = spend.rows.filter((row) => (row.hour ?? 0) >= Math.floor(start / HOUR) * HOUR && (pick.harness == null || row.harness === pick.harness) && (pick.model == null || row.model === pick.model));
  const hours = hourly(range);
  const first = hours ? Math.floor(start / HOUR) * HOUR : start;
  const count = hours ? Math.floor(now / HOUR) - first / HOUR + 1 : ranges.find((r) => r.value === range)!.days;
  const buckets = Array.from({ length: count }, (_, i) => (hours ? first + i * HOUR : daysAfter(first, i)));
  const bucketOf = (hour: number) => Math.min(count - 1, Math.max(0, hours ? Math.round((hour - first) / HOUR) : Math.round((startOfDay(hour) - first) / DAY)));

  const ranks = harnessRanks(spend, metric);
  const series = new Map<string, number[]>();
  const perBucket = buckets.map(() => ({ tokens: 0, cost: null as number | null }));
  const types: TokenTypes = { input: 0, cacheRead: 0, cacheWrite: 0, output: 0 };
  let cost: number | null = null;
  let saved: number | null = null;
  for (const row of rows) {
    const tokens = tokensOf(row);
    const i = bucketOf(row.hour ?? 0);
    const values = series.get(row.harness) ?? buckets.map(() => 0);
    values[i] += valueOf(metric, tokens, row.cost ?? null) ?? 0;
    series.set(row.harness, values);
    perBucket[i].tokens += tokens;
    perBucket[i].cost = plus(perBucket[i].cost, row.cost);
    types.input += row.input ?? 0;
    types.cacheRead += row.cacheRead ?? 0;
    types.cacheWrite += row.cacheWrite ?? 0;
    types.output += row.output ?? 0;
    cost = plus(cost, row.cost);
    saved = plus(saved, row.saved);
  }
  const priced = new Set(rows.filter((row) => row.cost != null).map((row) => row.harness));
  const processed = tokensOf(types);
  return {
    total: valueOf(metric, processed, cost),
    sessions: pick.model != null ? null : pick.harness != null ? (spend.sessions[pick.harness] ?? 0) : Object.values(spend.sessions).reduce((n, s) => n + s, 0),
    buckets,
    series: [...series]
      .filter(([harness]) => metric === "tokens" || priced.has(harness))
      .map(([harness, values]) => ({ harness, rank: ranks.indexOf(harness), values }))
      .sort((a, b) => a.rank - b.rank),
    types,
    figures: { processed, cached: types.cacheRead, uncached: types.input + types.cacheWrite, output: types.output, saved },
    busiest: perBucket
      .flatMap((b, i) => {
        const value = valueOf(metric, b.tokens, b.cost);
        return b.tokens > 0 && value != null ? [{ at: buckets[i], tokens: b.tokens, value }] : [];
      })
      .sort((a, b) => b.value - a.value || b.tokens - a.tokens)
      .slice(0, 3),
  };
}

const compact = new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 2 });
const currency = new Intl.NumberFormat(undefined, { style: "currency", currency: "USD" });
const percent = new Intl.NumberFormat(undefined, { style: "percent", minimumFractionDigits: 1, maximumFractionDigits: 1 });

/** "6.92B", "123M". */
export const tokensText = (tokens: number) => compact.format(tokens);
/** pi records its costs in US dollars. */
export const costText = (cost: number) => currency.format(cost);
/** "98.2%"; a share above 0 and under 0.1% reads "<0.1%". */
export const shareText = (share: number) => (share > 0 && share < 0.001 ? `<${percent.format(0.001)}` : percent.format(share));
export const valueText = (value: number, metric: Metric) => (metric === "cost" ? costText(value) : tokensText(value));

const compactCost = new Intl.NumberFormat(undefined, { style: "currency", currency: "USD", notation: "compact", maximumFractionDigits: 1 });
const dayFormat = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" });
const hourFormat = new Intl.DateTimeFormat(undefined, { hour: "numeric" });
const weekdayFormat = new Intl.DateTimeFormat(undefined, { weekday: "short", month: "short", day: "numeric" });
const hourOfDayFormat = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", hour: "numeric" });

/** A grid line's value: "1.5B", "$40". */
export const axisText = (value: number, metric: Metric) => (metric === "cost" ? compactCost.format(value) : tokensText(value));
/** A chart point under the axis: "3 PM" for Past 24h, "Jun 12" otherwise. */
export const bucketText = (at: number, range: Range) => (hourly(range) ? hourFormat : dayFormat).format(at);
/** A busiest row: "Jun 12, 3 PM" for Past 24h, "Tue, Jun 12" otherwise. */
export const bucketTitle = (at: number, range: Range) => (hourly(range) ? hourOfDayFormat : weekdayFormat).format(at);
export const sessionsText = (count: number) => (count === 1 ? "1 session" : `${count.toLocaleString()} sessions`);

/** The chart's top: the first of 1, 1.5, 2, 3, 4, 5, 6, 8, 10 times a power of ten at or above `peak`. */
export function chartMax(peak: number): number {
  if (peak <= 0) return 1;
  const power = 10 ** Math.floor(Math.log10(peak));
  return [1, 1.5, 2, 3, 4, 5, 6, 8, 10].map((step) => step * power).find((top) => top >= peak) ?? 10 * power;
}
