import { useState } from "react";
import type { Invoice } from "./servicio";

type InvoiceRowProps = { invoice: Invoice; onSelect: (id: string) => void };

export function InvoiceRow({ invoice, onSelect }: InvoiceRowProps) {
  return (
    <tr onClick={() => onSelect(invoice.id)} className="invoice-row">
      <td>{invoice.customerId}</td>
      <td>{invoice.total.toFixed(2)} €</td>
    </tr>
  );
}

export function InvoiceTable({ invoices }: { invoices: Invoice[] }) {
  const [selected, setSelected] = useState<string | null>(null);
  return (
    <table>
      <tbody>
        {invoices.map((invoice) => (
          <InvoiceRow key={invoice.id} invoice={invoice} onSelect={setSelected} />
        ))}
      </tbody>
      {selected && <caption>Factura {selected}</caption>}
    </table>
  );
}
