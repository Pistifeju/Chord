import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import type { Spend } from "../../ipc/bindings";
import { tooltip } from "../../app/commands";
import { sheet } from "../../app/sheet";
import { dragWindowOrClose } from "../../ui/backdrop";
import { CloseIcon, ReloadIcon } from "../../ui/icons";
import { Segmented } from "../../ui/Segmented";
import { harnessMarks } from "../harnesses/logos";
import { closeUsagePanel } from "./commands";
import { axisText, bucketText, bucketTitle, chartMax, costText, detail, hourly, inRange, list, ranges, sessionsText, shareText, tokensOf, tokensText, valueText, type ListRow, type Metric, type Range, type TokenTypes as Types } from "./spend";
import { readSpend, spend as lastSpend, spendReading } from "./store";
import "./usage.css";

const metrics: { value: Metric; label: string }[] = [
  { value: "cost", label: "Cost" },
  { value: "tokens", label: "Tokens" },
];

const titleOf = (harness: string) => harnessMarks[harness]?.title ?? harness;
const shown = (value: number | null, metric: Metric) => (value == null ? "" : valueText(value, metric));

export function UsagePanel() {
  const sheetOpen = sheet.use() !== null;
  const last = lastSpend.use();
  const reading = spendReading.use();
  const [range, setRange] = useState<Range>("30d");
  const [metric, setMetric] = useState<Metric>("cost");
  const [pick, setPick] = useState("all");
  useEffect(() => readSpend(range), [range]);
  const spend = useMemo(() => last && inRange(last.read, range, last.at), [last, range]);

  return (
    <div className="sheet-backdrop" inert={sheetOpen} onMouseDown={(e) => dragWindowOrClose(e, closeUsagePanel)}>
      <div className="usage-panel" role="dialog" aria-modal="true" aria-label="Usage" onMouseDown={(e) => e.stopPropagation()}>
        <header className="usage-header">
          <h2 className="usage-title">Usage</h2>
          <Segmented label="Range" options={ranges.map((r) => ({ value: r.value, label: r.label, title: r.title }))} value={range} onChange={setRange} />
          <Segmented label="Show" options={metrics} value={metric} onChange={setMetric} />
          <button type="button" className="icon-button usage-refresh" aria-label="Refresh" title="Refresh" aria-busy={reading} onClick={() => readSpend(range)}>
            <ReloadIcon />
          </button>
          <button type="button" className="icon-button" aria-label="Close Usage" title={tooltip("usage.close")} onClick={closeUsagePanel}>
            <CloseIcon />
          </button>
        </header>
        {spend == null ? (
          <p className="usage-state">Loading</p>
        ) : spend.rows.length === 0 ? (
          <p className="usage-state usage-empty">No usage in this range</p>
        ) : (
          <Body spend={spend} range={range} metric={metric} now={last!.at} pick={pick} onPick={setPick} />
        )}
      </div>
    </div>
  );
}

function Body({ spend, range, metric, now, pick, onPick }: { spend: Spend; range: Range; metric: Metric; now: number; pick: string; onPick: (key: string) => void }) {
  const rows = useMemo(() => list(spend, metric), [spend, metric]);
  const listbox = useRef<HTMLDivElement>(null);
  const index = Math.max(0, rows.findIndex((row) => row.key === pick));
  useEffect(() => listbox.current?.focus(), []);
  useEffect(() => {
    listbox.current?.querySelector(`[data-index="${index}"]`)?.scrollIntoView({ block: "nearest" });
  }, [index]);
  const step = (e: KeyboardEvent) => {
    const next = { ArrowDown: index + 1, ArrowUp: index - 1, Home: 0, End: rows.length - 1 }[e.key];
    if (next == null || next < 0 || next >= rows.length) return;
    e.preventDefault();
    onPick(rows[next].key);
  };
  return (
    <div className="usage-split">
      <div ref={listbox} className="usage-list" role="listbox" aria-label="Agents and models" tabIndex={0} aria-activedescendant={`usage-row-${index}`} onKeyDown={step}>
        {rows.map((row, i) => (
          <ListItem key={row.key} row={row} index={i} picked={i === index} metric={metric} onPick={() => onPick(row.key)} />
        ))}
      </div>
      <Inspector spend={spend} range={range} metric={metric} now={now} row={rows[index]} />
    </div>
  );
}

function ListItem({ row, index, picked, metric, onPick }: { row: ListRow; index: number; picked: boolean; metric: Metric; onPick: () => void }) {
  const kind = row.model != null ? "model" : row.harness != null ? "harness" : "all";
  const Logo = kind === "harness" && row.harness ? harnessMarks[row.harness]?.Logo : undefined;
  return (
    <div id={`usage-row-${index}`} data-index={index} role="option" aria-selected={picked} className="usage-row" data-kind={kind} onClick={onPick}>
      {row.rank != null && <span className="usage-dot" data-rank={Math.min(row.rank, 2)} />}
      {Logo && (
        <span className="usage-logo">
          <Logo />
        </span>
      )}
      <span className="usage-row-name">
        <span className="usage-row-title">{kind === "all" ? "All agents" : kind === "harness" ? titleOf(row.harness!) : row.model}</span>
        {kind === "model" && <span className="usage-row-bar" style={{ width: `${(row.share ?? 0) * 100}%` }} />}
      </span>
      <span className="usage-row-share">{row.share == null || kind === "all" ? "" : shareText(row.share)}</span>
      <span className="usage-row-value">{shown(row.value, metric)}</span>
    </div>
  );
}

function Inspector({ spend, range, metric, now, row }: { spend: Spend; range: Range; metric: Metric; now: number; row: ListRow }) {
  const data = useMemo(() => detail(spend, range, now, metric, row), [spend, range, now, metric, row]);
  const rangeTitle = ranges.find((r) => r.value === range)!.title;
  const Logo = row.harness ? harnessMarks[row.harness]?.Logo : undefined;
  const name = row.model ?? (row.harness ? titleOf(row.harness) : "All agents");
  const figures: [string, string][] = [
    ["Processed tokens", tokensText(data.figures.processed)],
    ["Cached input", tokensText(data.figures.cached)],
    ["Uncached input", tokensText(data.figures.uncached)],
    ["Output", tokensText(data.figures.output)],
    ["Cache savings", data.figures.saved == null ? "No cost recorded" : costText(data.figures.saved)],
  ];
  return (
    <aside className="usage-inspector" aria-label={name}>
      <h3 className="usage-inspector-title" key={row.key}>
        {Logo && (
          <span className="usage-logo">
            <Logo />
          </span>
        )}
        <span data-selectable>{name}</span>
      </h3>
      <div className="usage-total-block">
        {data.total == null ? (
          <p className="usage-total" data-none>
            No cost recorded
          </p>
        ) : (
          <p className="usage-total">{valueText(data.total, metric)}</p>
        )}
        <p className="usage-meta">{data.sessions == null ? rangeTitle : `${sessionsText(data.sessions)} · ${rangeTitle}`}</p>
      </div>
      <Chart buckets={data.buckets} series={data.series} range={range} metric={metric} label={`${rangeTitle}: ${data.total == null ? "no cost recorded" : valueText(data.total, metric)}`} />
      <TokenTypes types={data.types} />
      <dl className="usage-facts">
        {figures.map(([label, value]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd>{value}</dd>
          </div>
        ))}
      </dl>
      {data.busiest.length > 0 && (
        <div>
          <h4 className="field-label usage-facts-label">{hourly(range) ? "Busiest hours" : "Busiest days"}</h4>
          <dl className="usage-facts">
            {data.busiest.map((bucket) => (
              <div key={bucket.at}>
                <dt>{bucketTitle(bucket.at, range)}</dt>
                <dd>{valueText(bucket.value, metric)}</dd>
              </div>
            ))}
          </dl>
        </div>
      )}
    </aside>
  );
}

const CHART = { width: 280, height: 140, left: 52, right: 8, top: 10, bottom: 24 };

function Chart({ buckets, series, range, metric, label }: { buckets: number[]; series: { harness: string; rank: number; values: number[] }[]; range: Range; metric: Metric; label: string }) {
  const { width, height, left, right, top, bottom } = CHART;
  const plot = height - top - bottom;
  const n = buckets.length;
  const max = chartMax(Math.max(0, ...series.flatMap((s) => s.values)));
  const x = (i: number) => left + (n > 1 ? (i * (width - left - right)) / (n - 1) : (width - left - right) / 2);
  const y = (v: number) => top + plot - (v / max) * plot;
  const path = (values: number[]) => values.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)} ${y(v).toFixed(1)}`).join("");
  const labels = n > 2 ? [0, Math.floor((n - 1) / 2), n - 1] : [0, n - 1];
  const lead = series[0];
  return (
    <svg className="usage-chart" width={width} height={height} role="img" aria-label={label}>
      {[0, 1, 2, 3].map((step) => {
        const value = (max * step) / 3;
        return (
          <g key={step}>
            <line className="usage-grid" x1={left} x2={width - right} y1={y(value)} y2={y(value)} />
            <text className="usage-axis" x={left - 8} y={y(value)} textAnchor="end" dominantBaseline="middle">
              {axisText(value, metric)}
            </text>
          </g>
        );
      })}
      {[...new Set(labels)].map((i) => (
        <text key={i} className="usage-axis" x={x(i)} y={height - 4} textAnchor={i === 0 ? "start" : i === n - 1 ? "end" : "middle"}>
          {bucketText(buckets[i], range)}
        </text>
      ))}
      {lead?.rank === 0 && <path className="usage-area" d={`${path(lead.values)}L${x(n - 1)} ${y(0)}L${x(0)} ${y(0)}Z`} />}
      {[...series].reverse().map((s) => (
        <path key={s.harness} className="usage-line" data-rank={Math.min(s.rank, 2)} d={path(s.values)} />
      ))}
    </svg>
  );
}

const typeParts = [
  ["input", "Input"],
  ["cacheRead", "Cache read"],
  ["cacheWrite", "Cache write"],
  ["output", "Output"],
] as const;

function TokenTypes({ types }: { types: Types }) {
  const sum = tokensOf(types);
  const label = typeParts.map(([key, name]) => `${name} ${shareText(sum ? types[key] / sum : 0)}`).join(", ");
  return (
    <div className="usage-types">
      <div className="usage-typebar" role="img" aria-label={label}>
        {typeParts.map(([key], i) => (
          <span key={key} data-part={i} style={{ flexGrow: sum ? types[key] / sum : 0 }} />
        ))}
      </div>
      <ul className="usage-legend">
        {typeParts.map(([key, name], i) => (
          <li key={key}>
            <span className="usage-swatch" data-part={i} />
            {name}
            <span className="usage-legend-value">{tokensText(types[key])}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
