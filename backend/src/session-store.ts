import { randomUUID } from "crypto";
import type { MenuItem, Session, PaymentStatus } from "./types.js";

export class SessionStore {
  private sessions = new Map<string, Session>();

  create(item: MenuItem, baseUrl: string): Session {
    const session_id = randomUUID();
    const session: Session = {
      session_id,
      item,
      payment_url: `${baseUrl}/pay/${session_id}`,
      status: "pending",
      created_at: Date.now(),
    };
    this.sessions.set(session_id, session);
    return session;
  }

  get(id: string): Session | undefined {
    return this.sessions.get(id);
  }

  updateStatus(id: string, status: PaymentStatus): void {
    const session = this.sessions.get(id);
    if (session) {
      session.status = status;
    }
  }
}
