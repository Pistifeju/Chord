// Tests for the status rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { ageLabel, stampedReport, urgentReport, type StatusReport } from "./status.ts";

const report = (status: StatusReport["status"], since = 0): StatusReport => ({ status, lastMessage: "", since });

test("the most urgent report is the one that waits, then an error, an exit, a run, then done", () => {
  const kinds = ["done", "running", "exited", "error", "waiting"] as const;
  const reports = kinds.map((kind) => report(kind === "exited" ? { kind, code: 1 } : { kind }));
  assert.equal(urgentReport(reports)?.status.kind, "waiting");
  assert.equal(urgentReport(reports.slice(0, 4))?.status.kind, "error");
  assert.equal(urgentReport(reports.slice(0, 3))?.status.kind, "exited");
  assert.equal(urgentReport([null, undefined]), null);
});

test("a report keeps its start time while its status stays the same", () => {
  const before = report({ kind: "done" }, 100);
  assert.equal(stampedReport(before, report({ kind: "done" }), 500).since, 100);
  assert.equal(stampedReport(before, report({ kind: "running" }), 500).since, 500);
  assert.equal(stampedReport(report({ kind: "exited", code: 1 }, 100), report({ kind: "exited", code: 2 }), 500).since, 500);
});

test("an age reads in minutes, hours and days", () => {
  assert.deepEqual([0, 59_000, 5 * 60_000, 3 * 3_600_000, 2 * 86_400_000].map((ms) => ageLabel(0, ms)), ["now", "now", "5m", "3h", "2d"]);
});
