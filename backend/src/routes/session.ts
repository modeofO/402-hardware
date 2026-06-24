import { Router } from "express";
import type { SessionStore } from "../session-store.js";

const MENU_IDS = new Set(["1", "2", "3"]);

const MENU = [
  { id: "1", name: "Soda", price_usdc: "1.50" },
  { id: "2", name: "Water", price_usdc: "1.00" },
  { id: "3", name: "Snack", price_usdc: "2.00" },
];

export const sessionRouter = Router();

sessionRouter.post("/session", (req, res) => {
  const { item_id } = req.body;
  if (!item_id || !MENU_IDS.has(item_id)) {
    res.status(404).json({ error: "item not found" });
    return;
  }

  const item = MENU.find((m) => m.id === item_id)!;
  const store = req.app.locals.store as SessionStore;
  const baseUrl = `${req.protocol}://${req.get("host")}`;
  const session = store.create(item, baseUrl);

  res.status(201).json({
    session_id: session.session_id,
    payment_url: session.payment_url,
  });
});

sessionRouter.get("/session/:id/status", (req, res) => {
  const store = req.app.locals.store as SessionStore;
  const session = store.get(req.params.id);
  if (!session) {
    res.status(404).json({ error: "session not found" });
    return;
  }
  res.json({ status: session.status });
});
