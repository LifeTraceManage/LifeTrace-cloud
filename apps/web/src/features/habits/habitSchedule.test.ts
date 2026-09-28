import { describe, expect, it } from "vitest";
import { baseMeta } from "../../services/cloud/types";
import { habitScheduledOnDate } from "./HabitsPage";
import type { JsonEntity } from "../../services/core";

function habit(overrides: Partial<JsonEntity> = {}): JsonEntity {
  return {
    meta: baseMeta("user", "device", "habit-1"),
    name: "阅读",
    activityType: "duration",
    unit: "分钟",
    minimumTarget: 10,
    normalTarget: 30,
    targetPeriod: "daily",
    targetDays: [],
    scheduleType: "daily",
    startDate: "2026-09-01",
    checkinMethod: "manual",
    syncSource: "web",
    description: null,
    isArchived: false,
    ...overrides,
  };
}

describe("habit scheduling", () => {
  it("runs every day when no target days are configured", () => {
    const activity = habit();
    expect(habitScheduledOnDate(activity, "2026-09-28")).toBe(true);
    expect(habitScheduledOnDate(activity, "2026-09-29")).toBe(true);
  });

  it("runs only on configured weekdays", () => {
    const activity = habit({ scheduleType: "custom", targetDays: [1, 3, 5] });
    expect(habitScheduledOnDate(activity, "2026-09-28")).toBe(true); // Monday
    expect(habitScheduledOnDate(activity, "2026-09-29")).toBe(false); // Tuesday
    expect(habitScheduledOnDate(activity, "2026-09-30")).toBe(true); // Wednesday
  });

  it("does not run before its start date", () => {
    const activity = habit({ startDate: "2026-10-01" });
    expect(habitScheduledOnDate(activity, "2026-09-30")).toBe(false);
    expect(habitScheduledOnDate(activity, "2026-10-01")).toBe(true);
  });
});
