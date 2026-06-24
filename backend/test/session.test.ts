import { describe, it, expect } from "vitest";
import request from "supertest";
import { app } from "../src/index.js";

describe("POST /api/session", () => {
  it("creates a session for a valid item", async () => {
    const res = await request(app)
      .post("/api/session")
      .send({ item_id: "1" });
    expect(res.status).toBe(201);
    expect(res.body).toHaveProperty("session_id");
    expect(res.body).toHaveProperty("payment_url");
  });

  it("returns 404 for invalid item", async () => {
    const res = await request(app)
      .post("/api/session")
      .send({ item_id: "999" });
    expect(res.status).toBe(404);
  });
});

describe("GET /api/session/:id/status", () => {
  it("returns pending status for new session", async () => {
    const create = await request(app)
      .post("/api/session")
      .send({ item_id: "1" });
    const sessionId = create.body.session_id;

    const res = await request(app).get(`/api/session/${sessionId}/status`);
    expect(res.status).toBe(200);
    expect(res.body.status).toBe("pending");
  });

  it("returns 404 for unknown session", async () => {
    const res = await request(app).get("/api/session/nonexistent/status");
    expect(res.status).toBe(404);
  });
});
