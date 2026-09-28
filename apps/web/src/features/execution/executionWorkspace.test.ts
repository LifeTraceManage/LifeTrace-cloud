import { describe, expect, it } from "vitest";
import { baseMeta } from "../../services/cloud/types";
import { createExecutionTaskDependency, dependencyCreatesCycle, taskBlockers } from "../../services/cloud/execution-advanced";
import type { JsonEntity } from "../../services/core";

const task=(id:string,status="todo"):JsonEntity=>({meta:baseMeta("u","d",id),title:id,status});

describe("execution workspace domain rules",()=>{
  it("detects dependency cycles",()=>{
    const deps=[createExecutionTaskDependency("u","d","b","a")];
    expect(dependencyCreatesCycle("a","b",deps)).toBe(true);
    expect(dependencyCreatesCycle("c","b",deps)).toBe(false);
  });
  it("only returns unfinished blockers",()=>{
    const tasks=[task("a","done"),task("b"),task("c")];
    const deps=[createExecutionTaskDependency("u","d","c","a"),createExecutionTaskDependency("u","d","c","b")];
    expect(taskBlockers("c",tasks,deps).map(x=>x.meta.id)).toEqual(["b"]);
  });
});