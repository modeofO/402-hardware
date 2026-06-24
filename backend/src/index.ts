import express from "express";
import dotenv from "dotenv";
import { menuRouter } from "./routes/menu.js";
import { sessionRouter } from "./routes/session.js";
import { payRouter } from "./routes/pay.js";
import { SessionStore } from "./session-store.js";

dotenv.config();

const app = express();
const port = parseInt(process.env.PORT || "3000");

app.use(express.json());

const store = new SessionStore();
app.locals.store = store;

app.get("/health", (_req, res) => {
  res.json({ status: "ok" });
});

app.use("/api", menuRouter);
app.use("/api", sessionRouter);
app.use("/", payRouter);

app.listen(port, () => {
  console.log(`x402 terminal backend listening on port ${port}`);
});

export { app };
