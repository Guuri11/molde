"""Servicio de facturas."""
from __future__ import annotations

import json
from dataclasses import dataclass
from decimal import Decimal
from pathlib import Path

from fastapi import APIRouter, HTTPException

from .db import session

router = APIRouter(prefix="/invoices")


@dataclass
class Invoice:
    id: str
    customer_id: str
    total: Decimal

    def to_json(self) -> str:
        return json.dumps({"id": self.id, "customer_id": self.customer_id, "total": str(self.total)})


class InvoiceRepository:
    def __init__(self, path: Path):
        self.path = path
        self.cache: dict[str, Invoice] = {}

    def find_invoice(self, invoice_id: str) -> Invoice | None:
        if invoice_id in self.cache:
            return self.cache[invoice_id]
        row = session.execute("SELECT * FROM invoices WHERE id = :id", {"id": invoice_id}).first()
        if row is None:
            return None
        invoice = Invoice(id=row.id, customer_id=row.customer_id, total=Decimal(row.total))
        self.cache[invoice_id] = invoice
        return invoice

    def list_invoices(self, customer_id: str, limit: int = 50) -> list[Invoice]:
        rows = session.execute(
            "SELECT * FROM invoices WHERE customer_id = :c LIMIT :l",
            {"c": customer_id, "l": limit},
        )
        return [Invoice(id=r.id, customer_id=r.customer_id, total=Decimal(r.total)) for r in rows]


@router.get("/{invoice_id}")
def get_invoice(invoice_id: str):
    invoice = InvoiceRepository(Path("/tmp/invoices")).find_invoice(invoice_id)
    if invoice is None:
        raise HTTPException(status_code=404, detail=f"invoice {invoice_id} not found")
    return {"invoice": invoice.to_json(), "currency": "EUR"}
