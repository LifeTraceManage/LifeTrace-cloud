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
    const activity = habit({ scheduleType: "weekly", targetDays: [1, 3, 5] });
    expect(habitScheduledOnDate(activity, "2026-09-28")).toBe(true); // Monday
    expect(habitScheduledOnDate(activity, "2026-09-29")).toBe(false); // Tuesday
    expect(habitScheduledOnDate(activity, "2026-09-30")).toBe(true); // Wednesday
  });

  it("keeps legacy custom weekday schedules compatible", () => {
    const activity = habit({ scheduleType: "custom", targetDays: [2, 4] });
    expect(habitScheduledOnDate(activity, "2026-09-29")).toBe(true); // Tuesday
    expect(habitScheduledOnDate(activity, "2026-09-30")).toBe(false); // Wednesday
  });

  it("supports weekday and weekend presets through weekly target days", () => {
    const weekdays = habit({ scheduleType: "weekly", targetDays: [1, 2, 3, 4, 5] });
    const weekends = habit({ scheduleType: "weekly", targetDays: [6, 7] });
    expect(habitScheduledOnDate(weekdays, "2026-10-02")).toBe(true); // Friday
    expect(habitScheduledOnDate(weekdays, "2026-10-03")).toBe(false); // Saturday
    expect(habitScheduledOnDate(weekends, "2026-10-03")).toBe(true);
    expect(habitScheduledOnDate(weekends, "2026-10-05")).toBe(false); // Monday
  });

  it("supports fixed day intervals anchored to the start date", () => {
    const activity = habit({ scheduleType: "interval", targetDays: [3], startDate: "2026-09-28" });
    expect(habitScheduledOnDate(activity, "2026-09-28")).toBe(true);
    expect(habitScheduledOnDate(activity, "2026-09-29")).toBe(false);
    expect(habitScheduledOnDate(activity, "2026-10-01")).toBe(true);
    expect(habitScheduledOnDate(activity, "2026-10-04")).toBe(true);
  });

  it("supports monthly schedules on a selected day of month", () => {
    const activity = habit({ scheduleType: "monthly", targetPeriod: "monthly", targetDays: [15], startDate: "2026-09-01" });
    expect(habitScheduledOnDate(activity, "2026-09-15")).toBe(true);
    expect(habitScheduledOnDate(activity, "2026-10-15")).toBe(true);
    expect(habitScheduledOnDate(activity, "2026-10-14")).toBe(false);
  });

  it("skips months that do not contain the selected day", () => {
    const activity = habit({ scheduleType: "monthly", targetPeriod: "monthly", targetDays: [31], startDate: "2026-01-01" });
    expect(habitScheduledOnDate(activity, "2026-01-31")).toBe(true);
    expect(habitScheduledOnDate(activity, "2026-02-28")).toBe(false);
    expect(habitScheduledOnDate(activity, "2026-03-31")).toBe(true);
  });

  it("does not run before its start date", () => {
    const activity = habit({ startDate: "2026-10-01" });
    expect(habitScheduledOnDate(activity, "2026-09-30")).toBe(false);
    expect(habitScheduledOnDate(activity, "2026-10-01")).toBe(true);
  });
});
