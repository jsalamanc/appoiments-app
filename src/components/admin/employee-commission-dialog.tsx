"use client";

import { Trash2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { AppointmentServiceOption } from "@/lib/appointments";
import {
  updateEmployeeCommissionRates,
  type CommissionRateInput,
  type CommissionRateType,
  type EmployeeCommissionRate,
} from "@/lib/commissions";
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
import type { Employee } from "@/types/database";

type OverrideRow = {
  serviceId: string;
  rateType: CommissionRateType;
  rateValue: number;
};

function ratesToOverrides(
  rates: EmployeeCommissionRate[]
): { defaultRate: CommissionRateInput | null; overrides: OverrideRow[] } {
  const defaultRate = rates.find((r) => r.serviceId === null);
  const overrides = rates
    .filter((r): r is EmployeeCommissionRate & { serviceId: string } => r.serviceId !== null)
    .map((r) => ({ serviceId: r.serviceId, rateType: r.rateType, rateValue: r.rateValue }));
  return {
    defaultRate: defaultRate
      ? { serviceId: null, rateType: defaultRate.rateType, rateValue: defaultRate.rateValue }
      : null,
    overrides,
  };
}

type EmployeeCommissionDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  employee: Employee | null;
  /** `null` mientras se está cargando desde el llamador (ver employees-manager.tsx). */
  rates: EmployeeCommissionRate[] | null;
  services: AppointmentServiceOption[];
};

export function EmployeeCommissionDialog({
  open,
  onOpenChange,
  employee,
  rates,
  services,
}: EmployeeCommissionDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>
            Comisión {employee ? `de ${employee.name}` : ""}
          </DialogTitle>
          <DialogDescription>
            Se calcula sobre el valor cobrado en las citas completadas que
            atendió. Puedes dar una tasa general y, opcionalmente, excepciones
            por servicio.
          </DialogDescription>
        </DialogHeader>

        {!employee || rates === null ? (
          <p className="text-sm text-muted-foreground">Cargando...</p>
        ) : (
          <CommissionForm
            employeeId={employee.id}
            initialRates={rates}
            services={services}
            onClose={() => onOpenChange(false)}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

/**
 * Se monta solo cuando `rates` ya llegó (igual convención que
 * `AvailabilityForm` en employee-availability-dialog.tsx) — evita el patrón
 * de sincronizar estado derivado de props dentro de un efecto.
 */
function CommissionForm({
  employeeId,
  initialRates,
  services,
  onClose,
}: {
  employeeId: string;
  initialRates: EmployeeCommissionRate[];
  services: AppointmentServiceOption[];
  onClose: () => void;
}) {
  const initial = ratesToOverrides(initialRates);
  const [hasDefault, setHasDefault] = useState(initial.defaultRate !== null);
  const [defaultRate, setDefaultRate] = useState<CommissionRateInput>(
    initial.defaultRate ?? { serviceId: null, rateType: "percentage", rateValue: 0 }
  );
  const [overrides, setOverrides] = useState<OverrideRow[]>(initial.overrides);
  const [saving, setSaving] = useState(false);

  const availableServices = services.filter(
    (s) => !overrides.some((o) => o.serviceId === s.id)
  );

  const addOverride = () => {
    const next = availableServices[0];
    if (!next) return;
    setOverrides((prev) => [
      ...prev,
      { serviceId: next.id, rateType: "percentage", rateValue: 0 },
    ]);
  };

  const updateOverride = (index: number, patch: Partial<OverrideRow>) => {
    setOverrides((prev) =>
      prev.map((row, i) => (i === index ? { ...row, ...patch } : row))
    );
  };

  const removeOverride = (index: number) => {
    setOverrides((prev) => prev.filter((_, i) => i !== index));
  };

  const save = async () => {
    setSaving(true);
    try {
      const input: CommissionRateInput[] = [
        ...(hasDefault ? [defaultRate] : []),
        ...overrides.map((o) => ({
          serviceId: o.serviceId,
          rateType: o.rateType,
          rateValue: o.rateValue,
        })),
      ];
      await updateEmployeeCommissionRates(employeeId, input);
      toast.success("Comisión actualizada");
      onClose();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo guardar la comisión");
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <div className="space-y-4">
        <div className="space-y-2 rounded-2xl border border-border/60 px-4 py-3 text-sm">
          <label className="flex items-center gap-2 font-medium">
            <input
              type="checkbox"
              checked={hasDefault}
              onChange={(e) => setHasDefault(e.target.checked)}
              className="size-4 rounded border"
            />
            Tasa general
          </label>
          {hasDefault && (
            <RateFields
              rateType={defaultRate.rateType}
              rateValue={defaultRate.rateValue}
              onChange={(patch) => setDefaultRate((prev) => ({ ...prev, ...patch }))}
            />
          )}
        </div>

        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <Label className="text-sm font-medium">Excepciones por servicio</Label>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={availableServices.length === 0}
              onClick={addOverride}
            >
              Agregar
            </Button>
          </div>
          {overrides.length === 0 ? (
            <p className="text-sm text-muted-foreground">Sin excepciones.</p>
          ) : (
            overrides.map((row, index) => (
              <div
                key={row.serviceId}
                className="space-y-2 rounded-2xl border border-border/60 px-4 py-3 text-sm"
              >
                <div className="flex items-center justify-between gap-2">
                  <select
                    className="h-9 flex-1 rounded-md border border-input bg-transparent px-3 text-sm"
                    value={row.serviceId}
                    onChange={(e) => updateOverride(index, { serviceId: e.target.value })}
                  >
                    <option value={row.serviceId}>
                      {services.find((s) => s.id === row.serviceId)?.name ?? row.serviceId}
                    </option>
                    {availableServices.map((s) => (
                      <option key={s.id} value={s.id}>
                        {s.name}
                      </option>
                    ))}
                  </select>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    onClick={() => removeOverride(index)}
                  >
                    <Trash2 className="size-4" />
                  </Button>
                </div>
                <RateFields
                  rateType={row.rateType}
                  rateValue={row.rateValue}
                  onChange={(patch) => updateOverride(index, patch)}
                />
              </div>
            ))
          )}
        </div>
      </div>

      <div className="flex justify-end gap-2 pt-2">
        <Button type="button" variant="outline" onClick={onClose}>
          Cancelar
        </Button>
        <Button type="button" disabled={saving} onClick={() => void save()}>
          {saving ? "Guardando..." : "Guardar"}
        </Button>
      </div>
    </>
  );
}

function RateFields({
  rateType,
  rateValue,
  onChange,
}: {
  rateType: CommissionRateType;
  rateValue: number;
  onChange: (patch: Partial<Pick<OverrideRow, "rateType" | "rateValue">>) => void;
}) {
  return (
    <div className="flex items-center gap-2">
      <select
        className="h-9 rounded-md border border-input bg-transparent px-3 text-sm"
        value={rateType}
        onChange={(e) => onChange({ rateType: e.target.value as CommissionRateType })}
      >
        <option value="percentage">Porcentaje</option>
        <option value="fixed">Monto fijo</option>
      </select>
      <Input
        type="number"
        min={0}
        max={rateType === "percentage" ? 100 : undefined}
        step="0.01"
        className="w-28"
        value={rateValue}
        onChange={(e) => onChange({ rateValue: Number(e.target.value) })}
      />
      <span className="text-muted-foreground">{rateType === "percentage" ? "%" : "$"}</span>
    </div>
  );
}
