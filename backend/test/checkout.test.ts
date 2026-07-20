import { describe, it, expect, afterEach } from "vitest";
import request from "supertest";
import { app } from "../src/index.js";

async function createSession(): Promise<string> {
  const create = await request(app).post("/api/session").send({ item_id: "1" });
  return create.body.session_id;
}

describe("GET /pay/:sessionId (browser)", () => {
  it("serves the checkout page when the client accepts html", async () => {
    const sessionId = await createSession();
    const res = await request(app)
      .get(`/pay/${sessionId}`)
      .set("Accept", "text/html,application/xhtml+xml");
    expect(res.status).toBe(200);
    expect(res.headers["content-type"]).toContain("text/html");
    expect(res.text).toContain("Soda");
    expect(res.text).toContain("Pay with wallet");
    expect(res.text).toContain("TransferWithAuthorization");
  });

  it("still returns the bare 402 protocol response for non-browser clients", async () => {
    const sessionId = await createSession();
    const res = await request(app).get(`/pay/${sessionId}`);
    expect(res.status).toBe(402);
    expect(res.headers["payment-required"]).toBeDefined();
  });

  it("serves a paid page for confirmed sessions when the client accepts html", async () => {
    const sessionId = await createSession();
    app.locals.store.confirm(sessionId, "0xtest", "0xtest");
    const res = await request(app)
      .get(`/pay/${sessionId}`)
      .set("Accept", "text/html");
    expect(res.status).toBe(200);
    expect(res.text).toContain("Payment complete");
  });
});

describe("POST /pay/:sessionId/dev-confirm", () => {
  afterEach(() => {
    delete process.env.ENABLE_DEV_CONFIRM;
  });

  it("is hidden when ENABLE_DEV_CONFIRM is not set", async () => {
    const sessionId = await createSession();
    const res = await request(app).post(`/pay/${sessionId}/dev-confirm`);
    expect(res.status).toBe(404);
  });

  it("confirms a session when enabled", async () => {
    process.env.ENABLE_DEV_CONFIRM = "1";
    const sessionId = await createSession();
    const res = await request(app).post(`/pay/${sessionId}/dev-confirm`);
    expect(res.status).toBe(200);

    const status = await request(app).get(`/api/session/${sessionId}/status`);
    expect(status.body.status).toBe("confirmed");
  });

  it("returns 404 for unknown sessions when enabled", async () => {
    process.env.ENABLE_DEV_CONFIRM = "1";
    const res = await request(app).post("/pay/nonexistent/dev-confirm");
    expect(res.status).toBe(404);
  });
});
