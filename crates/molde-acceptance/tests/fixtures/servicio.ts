import { z } from "zod";
import type { Request, Response } from "express";
import { db } from "./db";

export const InvoiceSchema = z.object({
  id: z.string().uuid(),
  customerId: z.string(),
  total: z.number().nonnegative(),
  issuedAt: z.coerce.date(),
});

export type Invoice = z.infer<typeof InvoiceSchema>;

export class InvoiceService {
  private readonly cache = new Map<string, Invoice>();

  constructor(private readonly table = "invoices") {}

  async findInvoice(id: string): Promise<Invoice | undefined> {
    const cached = this.cache.get(id);
    if (cached) {
      return cached;
    }
    const row = await db.query(`SELECT * FROM ${this.table} WHERE id = $1`, [id]);
    const invoice = InvoiceSchema.parse(row);
    this.cache.set(id, invoice);
    return invoice;
  }

  async listInvoices(customerId: string, limit = 50): Promise<Invoice[]> {
    const rows = await db.query(`SELECT * FROM ${this.table} WHERE customer_id = $1 LIMIT $2`, [customerId, limit]);
    return rows.map((r: unknown) => InvoiceSchema.parse(r));
  }
}

export async function getInvoice(req: Request, res: Response) {
  const service = new InvoiceService();
  const invoice = await service.findInvoice(req.params.id);
  if (!invoice) {
    res.status(404).json({ error: "invoice not found" });
    return;
  }
  res.json({ ...invoice, total: invoice.total.toFixed(2) });
}

export const routes = {
  "GET /invoices/:id": getInvoice,
  "GET /customers/:customerId/invoices": async (req: Request, res: Response) => {
    const invoices = await new InvoiceService().listInvoices(req.params.customerId, 20);
    res.json(invoices);
  },
};
