import { describe, expect, it } from "vitest";

const executeViews = ["today","planner","inbox","projects","habits","waiting","focus","review"] as const;
const canonical = (view:string|undefined) => executeViews.includes(view as typeof executeViews[number]) ? `/execute/${view}` : "/execute/today";

describe("navigation contract",()=>{
  it("keeps execute views under the execute workspace",()=>{
    expect(executeViews.map(canonical)).toEqual(executeViews.map(view=>`/execute/${view}`));
  });
  it("falls back unknown execute routes to today",()=>{
    expect(canonical(undefined)).toBe("/execute/today");
    expect(canonical("unknown")).toBe("/execute/today");
  });
  it("keeps core app destinations outside workspace namespace",()=>{
    const core=["/app/health","/app/fitness","/app/finance","/app/assistant","/app/settings"];
    expect(core.every(path=>path.startsWith("/app/"))).toBe(true);
    expect(executeViews).toContain("habits");
  });
});