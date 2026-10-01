import { describe, expect, it, vi } from "vitest";
import { AuthApi, CloudDataStore } from "./api";

describe("CloudDataStore sync client identity", () => {
  it("uses the authenticated desktop application identity when supplied", async () => {
    let requestBody: Record<string, unknown> | undefined;
    const fetcher = async (_input: RequestInfo | URL, init?: RequestInit) => {
      requestBody = JSON.parse(String(init?.body)) as Record<string, unknown>;
      return new Response(JSON.stringify({
        snapshotId: "snapshot-1",
        snapshotCursor: "cursor-1",
        items: [],
        nextPageToken: null,
        completed: true,
      }), { status: 200, headers: { "content-type": "application/json" } });
    };
    const store = new CloudDataStore("user-1", "device-1", "", fetcher, {
      appId: "lifetrace-desktop",
      clientVersion: "0.3.3",
      platform: "windows",
    });

    await store.load();

    expect(requestBody?.client).toMatchObject({
      appId: "lifetrace-desktop",
      clientVersion: "0.3.3",
      platform: "windows",
      deviceId: "device-1",
    });
  });
});


describe("AuthApi session probe", () => {
  it("accepts HTTP 204 as the normal no-cookie signed-out state", async () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const api = new AuthApi(async () => new Response(null, { status: 204 }));

    await expect(api.session()).resolves.toBeNull();
    expect(errorSpy).not.toHaveBeenCalled();
    errorSpy.mockRestore();
  });

  it("treats HTTP 401 as a normal signed-out state without error logging", async () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const api = new AuthApi(async () => new Response(
      JSON.stringify({ message: "authentication required" }),
      { status: 401, headers: { "content-type": "application/json" } },
    ));

    await expect(api.session()).resolves.toBeNull();
    expect(errorSpy).not.toHaveBeenCalled();
    errorSpy.mockRestore();
  });
});
