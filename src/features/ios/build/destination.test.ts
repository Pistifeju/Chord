import assert from "node:assert/strict";
import { test } from "node:test";
import type { Destinations, RunDestination } from "../../../ipc/bindings";
import { myMac, pickAfterListing } from "./destination.ts";

const phone: RunDestination = { kind: "simulator", model: "p18", runtime: "ios-27", name: "iPhone 18", os: "iOS 27.0" };
const old: RunDestination = { kind: "simulator", model: "p17", runtime: "ios-26", name: "iPhone 17", os: "iOS 26.4" };
const device: RunDestination = { kind: "device", id: "U1", name: "Ada's iPhone" };

test("a pick that is still listed stays", () => {
  const all: Destinations = { devices: [device], simulators: [{ os: "iOS 27.0", models: [phone] }], fallback: null };
  assert.deepEqual(pickAfterListing(device, all), { pick: device });
});

test("a pick that is gone falls back to the first simulator and the notice names both", () => {
  const all: Destinations = { devices: [], simulators: [{ os: "iOS 27.0", models: [phone] }], fallback: null };
  assert.deepEqual(pickAfterListing(old, all), { pick: phone, notice: "iPhone 17, iOS 26.4 is not available. Using iPhone 18, iOS 27.0." });
  assert.deepEqual(pickAfterListing(device, { devices: [], simulators: [] }), { pick: null, notice: "Ada's iPhone is not available." });
});

test("My Mac stays while the project builds for macOS and falls back once it does not", () => {
  const all: Destinations = { devices: [], simulators: [{ os: "iOS 27.0", models: [phone] }], fallback: null };
  assert.deepEqual(pickAfterListing(myMac, all, "only"), { pick: myMac });
  assert.deepEqual(pickAfterListing(myMac, null, undefined), { pick: myMac });
  assert.deepEqual(pickAfterListing(myMac, all, "none"), { pick: phone, notice: "My Mac is not available. Using iPhone 18, iOS 27.0." });
});

test("a simulator or device pick falls back to My Mac in a macOS-only project", () => {
  const all: Destinations = { devices: [device], simulators: [{ os: "iOS 27.0", models: [phone] }], fallback: null };
  assert.deepEqual(pickAfterListing(phone, all, "only"), { pick: myMac, notice: "iPhone 18, iOS 27.0 is not available. Using My Mac." });
  assert.deepEqual(pickAfterListing(device, null, "only"), { pick: myMac, notice: "Ada's iPhone is not available. Using My Mac." });
});
