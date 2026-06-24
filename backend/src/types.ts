export interface MenuItem {
  id: string;
  name: string;
  price_usdc: string;
}

export interface Session {
  session_id: string;
  item: MenuItem;
  payment_url: string;
  status: PaymentStatus;
  created_at: number;
  tx_hash?: string;
  payer?: string;
}

export type PaymentStatus = "pending" | "confirmed" | "failed";

export interface SessionStatus {
  status: PaymentStatus;
}
