import { describe, it, expect, afterEach } from "vitest";
import request from "supertest";
import { app } from "../src/index.js";

describe("GET /pay/:sessionId", () => {
  it("returns 402 with payment requirements when no signature", async () => {
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

  it("returns 200 for already-confirmed session", async () => {
    const create = await request(app)
      .post("/api/session")
      .send({ item_id: "1" });
    const sessionId = create.body.session_id;

    const store = app.locals.store;
    store.confirm(sessionId, "0xabc123", "0xpayer");

    const res = await request(app).get(`/pay/${sessionId}`);
    expect(res.status).toBe(200);
    expect(res.body.status).toBe("already paid");
  });

  it("returns 500 when payment signature is malformed", async () => {
    const create = await request(app)
      .post("/api/session")
      .send({ item_id: "1" });
    const sessionId = create.body.session_id;

    const res = await request(app)
      .get(`/pay/${sessionId}`)
      .set("PAYMENT-SIGNATURE", "not-valid-base64!!!");
    expect(res.status).toBe(500);
    expect(res.body.error).toBe("Payment processing failed");
  });

  describe("with mocked facilitator", () => {
    const originalFetch = globalThis.fetch;

    afterEach(() => {
      globalThis.fetch = originalFetch;
    });

    const makeSignatureHeader = () => {
      const payload = {
        x402Version: 2,
        accepted: {
          scheme: "exact",
          network: "eip155:8453",
          amount: "1500000",
          asset: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
          payTo: "0x0000000000000000000000000000000000000000",
          maxTimeoutSeconds: 120,
          extra: { name: "USD Coin", version: "2" },
        },
        payload: {
          signature: "0xdeadbeef",
          authorization: {
            from: "0xpayer",
            to: "0x0000000000000000000000000000000000000000",
            value: "1500000",
            validAfter: "0",
            validBefore: "9999999999",
            nonce: "0x1234",
          },
        },
      };
      return Buffer.from(JSON.stringify(payload)).toString("base64");
    };

    it("settles payment when facilitator verifies and settles successfully", async () => {
      const create = await request(app)
        .post("/api/session")
        .send({ item_id: "1" });
      const sessionId = create.body.session_id;

      globalThis.fetch = async (url: RequestInfo | URL) => {
        const urlStr = url.toString();
        if (urlStr.includes("/verify")) {
          return new Response(
            JSON.stringify({
              isValid: true,
              payer: "0xpayer123",
            }),
            { status: 200, headers: { "content-type": "application/json" } }
          );
        }
        if (urlStr.includes("/settle")) {
          return new Response(
            JSON.stringify({
              success: true,
              transaction: "0xtxhash456",
              network: "eip155:8453",
              payer: "0xpayer123",
            }),
            { status: 200, headers: { "content-type": "application/json" } }
          );
        }
        return new Response("not found", { status: 404 });
      };

      const res = await request(app)
        .get(`/pay/${sessionId}`)
        .set("PAYMENT-SIGNATURE", makeSignatureHeader());

      expect(res.status).toBe(200);
      expect(res.body.status).toBe("paid");
      expect(res.body.transaction).toBe("0xtxhash456");
      expect(res.headers["payment-response"]).toBeDefined();

      const session = app.locals.store.get(sessionId);
      expect(session.status).toBe("confirmed");
      expect(session.tx_hash).toBe("0xtxhash456");
      expect(session.payer).toBe("0xpayer123");
    });

    it("returns 402 when facilitator rejects the payment", async () => {
      const create = await request(app)
        .post("/api/session")
        .send({ item_id: "1" });
      const sessionId = create.body.session_id;

      globalThis.fetch = async (url: RequestInfo | URL) => {
        const urlStr = url.toString();
        if (urlStr.includes("/verify")) {
          return new Response(
            JSON.stringify({
              isValid: false,
              invalidReason: "insufficient_balance",
              invalidMessage: "Payer has insufficient USDC balance",
            }),
            { status: 200, headers: { "content-type": "application/json" } }
          );
        }
        return new Response("not found", { status: 404 });
      };

      const res = await request(app)
        .get(`/pay/${sessionId}`)
        .set("PAYMENT-SIGNATURE", makeSignatureHeader());

      expect(res.status).toBe(402);
      const paymentRequired = JSON.parse(
        Buffer.from(res.headers["payment-required"], "base64").toString()
      );
      expect(paymentRequired.error).toBe("insufficient_balance");
    });

    it("returns 502 when settlement fails", async () => {
      const create = await request(app)
        .post("/api/session")
        .send({ item_id: "1" });
      const sessionId = create.body.session_id;

      globalThis.fetch = async (url: RequestInfo | URL) => {
        const urlStr = url.toString();
        if (urlStr.includes("/verify")) {
          return new Response(
            JSON.stringify({ isValid: true, payer: "0xpayer" }),
            { status: 200, headers: { "content-type": "application/json" } }
          );
        }
        if (urlStr.includes("/settle")) {
          return new Response(
            JSON.stringify({
              success: false,
              errorReason: "tx_reverted",
              errorMessage: "Transaction reverted",
            }),
            { status: 500, headers: { "content-type": "application/json" } }
          );
        }
        return new Response("not found", { status: 404 });
      };

      const res = await request(app)
        .get(`/pay/${sessionId}`)
        .set("PAYMENT-SIGNATURE", makeSignatureHeader());

      expect(res.status).toBe(502);
      expect(res.body.error).toBe("Settlement failed");

      const session = app.locals.store.get(sessionId);
      expect(session.status).toBe("pending");
    });
  });
});
