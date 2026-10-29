"use client";

import { Lock, Unlock } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  closeCashSession,
  getCurrentCashSession,
  openCashSession,
  type CashSession,
} from "@/lib/cash-sessions";
import { formatCurrency, formatDateTime } from "@/lib/format";

export function CashSessionPanel() {
  const [session, setSession] = useState<CashSession | null>(null);
  const [loading, setLoading] = useState(true);
  const [openDialogOpen, setOpenDialogOpen] = useState(false);
  const [closeDialogOpen, setCloseDialogOpen] = useState(false);
  const [openingAmount, setOpeningAmount] = useState("0");
  const [countedAmount, setCountedAmount] = useState("0");
  const [notes, setNotes] = useState("");
  const [saving, setSaving] = useState(false);
  const [lastClosed, setLastClosed] = useState<CashSession | null>(null);

  useEffect(() => {
    let cancelled = false;
    getCurrentCashSession()
      .then((s) => {
        if (!cancelled) setSession(s);
      })
      .catch((err) => {
        if (!cancelled) {
          toast.error(err instanceof Error ? err.message : "Error al cargar la caja");
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const handleOpen = async () => {
    const amount = Number(openingAmount);
    if (!Number.isFinite(amount) || amount < 0) {
      toast.error("Ingresa un monto válido");
      return;
    }
    setSaving(true);
    try {
      const opened = await openCashSession(amount);
      setSession(opened);
      setOpenDialogOpen(false);
      setOpeningAmount("0");
      toast.success("Caja abierta");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo abrir la caja");
    } finally {
      setSaving(false);
    }
  };

  const handleClose = async () => {
    if (!session) return;
    const amount = Number(countedAmount);
    if (!Number.isFinite(amount) || amount < 0) {
      toast.error("Ingresa un monto válido");
      return;
    }
    setSaving(true);
    try {
      const closed = await closeCashSession(session.id, amount, notes);
      setSession(null);
      setLastClosed(closed);
      setCloseDialogOpen(false);
      setCountedAmount("0");
      setNotes("");
      toast.success("Caja cerrada");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo cerrar la caja");
    } finally {
      setSaving(false);
    }
  };

  if (loading) return null;

  return (
    <div className="print-hidden space-y-3">
      {session ? (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-3xl border border-border/70 bg-card px-5 py-4 shadow-sm">
          <div className="flex items-center gap-3">
            <Unlock className="size-4 text-muted-foreground" />
            <div>
              <p className="font-medium">
                Caja abierta — apertura {formatCurrency(session.openingAmount)}
              </p>
              <p className="text-sm text-muted-foreground">
                Desde {formatDateTime(session.openedAt)}
              </p>
            </div>
          </div>
          <Button variant="outline" size="sm" onClick={() => setCloseDialogOpen(true)}>
            <Lock data-icon="inline-start" />
            Cerrar caja
          </Button>
        </div>
      ) : (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-3xl border border-dashed border-border px-5 py-4">
          <div className="flex items-center gap-3">
            <Lock className="size-4 text-muted-foreground" />
            <p className="text-sm text-muted-foreground">
              No hay una caja abierta en este momento.
            </p>
          </div>
          <Button size="sm" onClick={() => setOpenDialogOpen(true)}>
            <Unlock data-icon="inline-start" />
            Abrir caja
          </Button>
        </div>
      )}

      {lastClosed && (
        <div className="rounded-2xl border border-border/60 px-4 py-3 text-sm">
          <p className="font-medium">Última caja cerrada</p>
          <p className="text-muted-foreground">
            Esperado {formatCurrency(lastClosed.expectedAmount ?? 0)} · Contado{" "}
            {formatCurrency(lastClosed.countedAmount ?? 0)} · Diferencia{" "}
            <span
              className={
                (lastClosed.difference ?? 0) === 0
                  ? "text-muted-foreground"
                  : (lastClosed.difference ?? 0) > 0
                    ? "text-emerald-600"
                    : "text-destructive"
              }
            >
              {formatCurrency(lastClosed.difference ?? 0)}
            </span>
          </p>
        </div>
      )}

      <Dialog open={openDialogOpen} onOpenChange={setOpenDialogOpen}>
        <DialogContent className="sm:max-w-sm">
          <DialogHeader>
            <DialogTitle>Abrir caja</DialogTitle>
            <DialogDescription>
              Registra el monto en efectivo con el que arranca el turno.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            <Label htmlFor="opening-amount">Monto de apertura</Label>
            <Input
              id="opening-amount"
              type="number"
              min={0}
              step="100"
              value={openingAmount}
              onChange={(e) => setOpeningAmount(e.target.value)}
            />
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => setOpenDialogOpen(false)}
            >
              Cancelar
            </Button>
            <Button type="button" disabled={saving} onClick={() => void handleOpen()}>
              {saving ? "Abriendo..." : "Abrir caja"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      <Dialog open={closeDialogOpen} onOpenChange={setCloseDialogOpen}>
        <DialogContent className="sm:max-w-sm">
          <DialogHeader>
            <DialogTitle>Cerrar caja</DialogTitle>
            <DialogDescription>
              Cuenta el efectivo físico y regístralo — se compara contra lo
              esperado (apertura + ventas en efectivo del turno).
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            <Label htmlFor="counted-amount">Monto contado</Label>
            <Input
              id="counted-amount"
              type="number"
              min={0}
              step="100"
              value={countedAmount}
              onChange={(e) => setCountedAmount(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="close-notes">Notas (opcional)</Label>
            <Input
              id="close-notes"
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder="Ej. faltante por vuelto mal dado"
            />
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => setCloseDialogOpen(false)}
            >
              Cancelar
            </Button>
            <Button type="button" disabled={saving} onClick={() => void handleClose()}>
              {saving ? "Cerrando..." : "Cerrar caja"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
