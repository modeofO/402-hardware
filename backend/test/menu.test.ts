import { describe, it, expect } from "vitest";
import request from "supertest";
import { app } from "../src/index.js";

describe("GET /api/menu", () => {
  it("returns a list of menu items", async () => {
    const res = await request(app).get("/api/menu");
    expect(res.status).toBe(200);
    expect(Array.isArray(res.body)).toBe(true);
    expect(res.body.length).toBeGreaterThan(0);
    expect(res.body[0]).toHaveProperty("id");
    expect(res.body[0]).toHaveProperty("name");
    expect(res.body[0]).toHaveProperty("price_usdc");
  });
});
