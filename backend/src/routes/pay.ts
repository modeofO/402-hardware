import { Router } from "express";
import type { SessionStore } from "../session-store.js";
import { USDC_BASE, BASE_NETWORK, X402_VERSION } from "../x402.js";

export const payRouter = Router();

payRouter.get("/pay/:sessionId", (req, res) => {
  const store = req.app.locals.store as SessionStore;
  const session = store.get(req.params.sessionId);
  if (!session) {
    res.status(404).json({ error: "session not found" });
    return;
  }

  const payTo = process.env.PAYMENT_RECIPIENT || "0x0000000000000000000000000000000000000000";

  const amountAtomic = Math.round(
    parseFloat(session.item.price_usdc) * 1_000_000
  ).toString();

  const paymentRequired = {
    x402Version: X402_VERSION,
    error: "Payment required",
    resource: {
      url: session.payment_url,
      description: session.item.name,
      mimeType: "application/json",
    },
    accepts: [
      {
        scheme: "exact",
        network: BASE_NETWORK,
        amount: amountAtomic,
        asset: USDC_BASE,
        payTo,
        maxTimeoutSeconds: 120,
        extra: {
          name: "USD Coin",
          version: "2",
        },
      },
    ],
    extensions: {},
  };

  const encoded = Buffer.from(JSON.stringify(paymentRequired)).toString(
    "base64"
  );

  res.status(402).set("PAYMENT-REQUIRED", encoded).json({});
});
