import type { Session } from "./types.js";

interface CheckoutRequirements {
  scheme: string;
  network: string;
  amount: string;
  asset: string;
  payTo: string;
  maxTimeoutSeconds: number;
  extra: Record<string, unknown>;
}

interface CheckoutParams {
  session: Session;
  paymentRequirements: CheckoutRequirements;
  x402Version: number;
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/**
 * Browser-facing checkout page for a payment session. x402-aware clients
 * never see this — they get the bare 402 + PAYMENT-REQUIRED protocol
 * response. This page drives an EIP-1193 wallet (in-app dapp browser)
 * through the EIP-3009 transferWithAuthorization signature and resubmits
 * it as a PAYMENT-SIGNATURE header against the same URL.
 */
export function renderCheckoutPage(params: CheckoutParams): string {
  const { session, paymentRequirements, x402Version } = params;
  const chainId = parseInt(paymentRequirements.network.split(":")[1] || "0", 10);
  const data = {
    x402Version,
    chainId,
    paymentUrl: session.payment_url,
    requirements: paymentRequirements,
    resource: {
      url: session.payment_url,
      description: session.item.name,
      mimeType: "application/json",
    },
  };

  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Pay ${escapeHtml(session.item.price_usdc)} USDC</title>
<style>
  body { margin: 0; font-family: system-ui, sans-serif; background: #0d1117; color: #e6edf3;
         display: flex; min-height: 100vh; align-items: center; justify-content: center; }
  main { text-align: center; padding: 24px; max-width: 420px; }
  .item { font-size: 1.4rem; margin-bottom: 4px; }
  .price { font-size: 2.4rem; font-weight: 700; color: #ffd700; margin-bottom: 24px; }
  button { font-size: 1.2rem; padding: 14px 36px; border-radius: 12px; border: 0;
           background: #2da44e; color: white; width: 100%; }
  button:disabled { background: #30363d; color: #8b949e; }
  #status { margin-top: 18px; min-height: 2em; color: #8b949e; }
  #status.error { color: #f85149; }
  #status.ok { color: #2da44e; font-weight: 600; }
  .hint { font-size: 0.85rem; color: #8b949e; margin-top: 24px; }
  code { word-break: break-all; }
</style>
</head>
<body>
<main>
  <div class="item">${escapeHtml(session.item.name)}</div>
  <div class="price">${escapeHtml(session.item.price_usdc)} USDC</div>
  <button id="pay">Pay with wallet</button>
  <div id="status"></div>
  <div class="hint" id="hint">Pays USDC on Base via x402 (gasless — the facilitator submits the transfer).</div>
</main>
<script id="x402-data" type="application/json">${JSON.stringify(data)}</script>
<script>
const DATA = JSON.parse(document.getElementById("x402-data").textContent);
const btn = document.getElementById("pay");
const statusEl = document.getElementById("status");

function setStatus(msg, cls) {
  statusEl.textContent = msg;
  statusEl.className = cls || "";
}

function b64EncodeJson(obj) {
  const json = JSON.stringify(obj);
  const bytes = new TextEncoder().encode(json);
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin);
}

async function ensureChain(eth) {
  const chainIdHex = "0x" + DATA.chainId.toString(16);
  try {
    await eth.request({ method: "wallet_switchEthereumChain", params: [{ chainId: chainIdHex }] });
  } catch (e) {
    if (e && e.code === 4902) {
      await eth.request({ method: "wallet_addEthereumChain", params: [{
        chainId: chainIdHex, chainName: "Base",
        nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
        rpcUrls: ["https://mainnet.base.org"], blockExplorerUrls: ["https://basescan.org"],
      }] });
    } else {
      throw e;
    }
  }
}

async function pay() {
  const eth = window.ethereum;
  if (!eth) {
    setStatus("No wallet found. Open this link inside your wallet app's browser.", "error");
    return;
  }
  btn.disabled = true;
  try {
    setStatus("Connecting wallet...");
    const accounts = await eth.request({ method: "eth_requestAccounts" });
    const from = accounts[0];
    await ensureChain(eth);

    const req = DATA.requirements;
    const nonce = "0x" + Array.from(crypto.getRandomValues(new Uint8Array(32)))
      .map((b) => b.toString(16).padStart(2, "0")).join("");
    const authorization = {
      from,
      to: req.payTo,
      value: req.amount,
      validAfter: "0",
      validBefore: String(Math.floor(Date.now() / 1000) + 600),
      nonce,
    };
    const typedData = {
      types: {
        EIP712Domain: [
          { name: "name", type: "string" },
          { name: "version", type: "string" },
          { name: "chainId", type: "uint256" },
          { name: "verifyingContract", type: "address" },
        ],
        TransferWithAuthorization: [
          { name: "from", type: "address" },
          { name: "to", type: "address" },
          { name: "value", type: "uint256" },
          { name: "validAfter", type: "uint256" },
          { name: "validBefore", type: "uint256" },
          { name: "nonce", type: "bytes32" },
        ],
      },
      domain: {
        name: req.extra.name,
        version: req.extra.version,
        chainId: DATA.chainId,
        verifyingContract: req.asset,
      },
      primaryType: "TransferWithAuthorization",
      message: authorization,
    };

    setStatus("Confirm the signature in your wallet...");
    const signature = await eth.request({
      method: "eth_signTypedData_v4",
      params: [from, JSON.stringify(typedData)],
    });

    const paymentPayload = {
      x402Version: DATA.x402Version,
      resource: DATA.resource,
      accepted: req,
      payload: { signature, authorization },
    };

    setStatus("Submitting payment...");
    const resp = await fetch(DATA.paymentUrl, {
      headers: {
        "PAYMENT-SIGNATURE": b64EncodeJson(paymentPayload),
        "Accept": "application/json",
      },
    });
    if (resp.ok) {
      const body = await resp.json().catch(() => ({}));
      setStatus("Paid! Grab your item.", "ok");
      if (body.transaction) {
        document.getElementById("hint").innerHTML =
          "tx: <code>" + body.transaction + "</code>";
      }
      btn.style.display = "none";
      return;
    }
    let msg = "Payment failed (HTTP " + resp.status + ")";
    const header = resp.headers.get("PAYMENT-REQUIRED");
    if (header) {
      try { msg = JSON.parse(atob(header)).error || msg; } catch {}
    } else {
      try { msg = (await resp.json()).error || msg; } catch {}
    }
    setStatus(msg, "error");
    btn.disabled = false;
  } catch (e) {
    setStatus(e && e.message ? e.message : "Payment failed", "error");
    btn.disabled = false;
  }
}

btn.addEventListener("click", pay);
</script>
</body>
</html>`;
}

/** Minimal page for a session that has already been paid. */
export function renderPaidPage(session: Session): string {
  return `<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Paid</title></head>
<body style="margin:0;font-family:system-ui,sans-serif;background:#0d1117;color:#2da44e;display:flex;min-height:100vh;align-items:center;justify-content:center;font-size:1.6rem;font-weight:600">
Payment complete — enjoy your ${escapeHtml(session.item.name)}!
</body></html>`;
}
