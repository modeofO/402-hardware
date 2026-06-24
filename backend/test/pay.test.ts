import { describe, it, expect } from "vitest";
import request from "supertest";
import { app } from "../src/index.js";

describe("GET /pay/:sessionId", () => {
  it("returns 402 with payment requirements for valid session", async () => {
    const create = await request(app)
      .post("/api/session")
      .send({ item_id: "1" });
    const sessionId = create.body.session_id;

    const res = await request(app).get(`/pay/${sessionId}`);
    expect(res.status).toBe(402);

    const paymentRequired = JSON.parse(
      Buffer.from(res.headers["payment-required"], "base64").toString()
    );
    expect(paymentRequired.x402Version).toBe(2);
    expect(paymentRequired.accepts).toHaveLength(1);
    expect(paymentRequired.accepts[0].scheme).toBe("exact");
    expect(paymentRequired.accepts[0].network).toBe("eip155:8453");
    expect(paymentRequired.accepts[0].extra.name).toBe("USD Coin");
  });

  it("returns 404 for unknown session", async () => {
    const res = await request(app).get("/pay/nonexistent");
    expect(res.status).toBe(404);
  });
});
