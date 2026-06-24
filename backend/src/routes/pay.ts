import { Router } from "express";
import {
  decodePaymentSignatureHeader,
  encodePaymentResponseHeader,
  HTTPFacilitatorClient,
} from "@x402/core/http";
import { SettleError } from "@x402/core/types";
import type { SessionStore } from "../session-store.js";
import {
  USDC_BASE,
  BASE_NETWORK,
  X402_VERSION,
  FACILITATOR_URL,
} from "../x402.js";

export const payRouter = Router();

const facilitator = new HTTPFacilitatorClient({ url: FACILITATOR_URL });

payRouter.get("/pay/:sessionId", async (req, res) => {
  const store = req.app.locals.store as SessionStore;
  const session = store.get(req.params.sessionId);
  if (!session) {
    res.status(404).json({ error: "session not found" });
    return;
  }

  if (session.status === "confirmed") {
    res.status(200).json({ status: "already paid" });
    return;
  }

  const payTo =
    process.env.PAYMENT_RECIPIENT ||
    "0x0000000000000000000000000000000000000000";

  const amountAtomic = Math.round(
    parseFloat(session.item.price_usdc) * 1_000_000
  ).toString();

  const paymentRequirements = {
    scheme: "exact" as const,
    network: BASE_NETWORK,
    amount: amountAtomic,
    asset: USDC_BASE,
    payTo,
    maxTimeoutSeconds: 120,
    extra: {
      name: "USD Coin",
      version: "2",
    },
  };

  const signatureHeader =
    req.headers["payment-signature"] || req.headers["PAYMENT-SIGNATURE"];

  if (!signatureHeader) {
    const paymentRequired = {
      x402Version: X402_VERSION,
      error: "Payment required",
      resource: {
        url: session.payment_url,
        description: session.item.name,
        mimeType: "application/json",
      },
      accepts: [paymentRequirements],
      extensions: {},
    };

    const encoded = Buffer.from(JSON.stringify(paymentRequired)).toString(
      "base64"
    );

    res.status(402).set("PAYMENT-REQUIRED", encoded).json({});
    return;
  }

  try {
    const paymentPayload = decodePaymentSignatureHeader(
      signatureHeader as string
    );

    const verifyResult = await facilitator.verify(
      paymentPayload,
      paymentRequirements
    );

    if (!verifyResult.isValid) {
      const paymentRequired = {
        x402Version: X402_VERSION,
        error: verifyResult.invalidReason || "Invalid payment",
        resource: {
          url: session.payment_url,
          description: session.item.name,
          mimeType: "application/json",
        },
        accepts: [paymentRequirements],
        extensions: {},
      };

      const encoded = Buffer.from(JSON.stringify(paymentRequired)).toString(
        "base64"
      );

      res.status(402).set("PAYMENT-REQUIRED", encoded).json({});
      return;
    }

    const settleResult = await facilitator.settle(
      paymentPayload,
      paymentRequirements
    );

    if (!settleResult.success) {
      res.status(502).json({
        error: "Settlement failed",
        reason: settleResult.errorReason,
      });
      return;
    }

    store.confirm(
      req.params.sessionId,
      settleResult.transaction || "",
      settleResult.payer || verifyResult.payer || ""
    );

    const paymentResponse = encodePaymentResponseHeader(settleResult);

    res
      .status(200)
      .set("PAYMENT-RESPONSE", paymentResponse)
      .json({ status: "paid", transaction: settleResult.transaction });
  } catch (err) {
    if (err instanceof SettleError) {
      res.status(502).json({
        error: "Settlement failed",
        reason: err.errorReason,
      });
      return;
    }
    res.status(500).json({
      error: "Payment processing failed",
      message: err instanceof Error ? err.message : "Unknown error",
    });
  }
});
