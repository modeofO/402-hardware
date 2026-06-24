import { Router } from "express";
import type { MenuItem } from "../types.js";

export const menuRouter = Router();

const MENU: MenuItem[] = [
  { id: "1", name: "Soda", price_usdc: "1.50" },
  { id: "2", name: "Water", price_usdc: "1.00" },
  { id: "3", name: "Snack", price_usdc: "2.00" },
];

menuRouter.get("/menu", (_req, res) => {
  res.json(MENU);
});
