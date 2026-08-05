"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
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
import { createEmployee, updateEmployee } from "@/lib/employees";
import { employeeSchema, type EmployeeValues } from "@/lib/validations";
import type { Employee } from "@/types/database";

const DEFAULT_COLOR = "#b89bb8";

type EmployeeFormDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  employee?: Employee | null;
  onSuccess: () => void;
};

export function EmployeeFormDialog({
  open,
  onOpenChange,
  employee,
  onSuccess,
}: EmployeeFormDialogProps) {
  const [loading, setLoading] = useState(false);

  const {
    register,
    handleSubmit,
    reset,
    watch,
    setValue,
    formState: { errors },
  } = useForm<EmployeeValues>({
    resolver: zodResolver(employeeSchema),
    defaultValues: { name: "", color: DEFAULT_COLOR, active: true },
  });

  const active = watch("active");
  const color = watch("color");

  useEffect(() => {
    if (open) {
      reset({
        name: employee?.name ?? "",
        color: employee?.color ?? DEFAULT_COLOR,
        active: employee?.active ?? true,
      });
    }
  }, [open, employee, reset]);

  const save = async (values: EmployeeValues) => {
    setLoading(true);
    try {
      if (employee) {
        await updateEmployee(employee.id, values);
        toast.success("Empleado actualizado");
      } else {
        await createEmployee(values);
        toast.success("Empleado creado");
      }
      onOpenChange(false);
      onSuccess();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Error al guardar");
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            {employee ? "Editar empleado" : "Nuevo empleado"}
          </DialogTitle>
          <DialogDescription>
            Cada empleado tiene su propio calendario en la agenda.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit(save)} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="employee-name">Nombre</Label>
            <Input id="employee-name" {...register("name")} />
            {errors.name && (
              <p className="text-xs text-destructive">{errors.name.message}</p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="employee-color">Color en agenda</Label>
            <div className="flex items-center gap-3">
              <Input
                id="employee-color"
                type="color"
                className="h-10 w-14 cursor-pointer p-1"
                value={color}
                onChange={(e) =>
                  setValue("color", e.target.value, { shouldValidate: true })
                }
              />
              <Input {...register("color")} placeholder="#b89bb8" />
            </div>
            {errors.color && (
              <p className="text-xs text-destructive">{errors.color.message}</p>
            )}
          </div>

          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={active}
              onChange={(e) =>
                setValue("active", e.target.checked, { shouldValidate: true })
              }
              className="size-4 rounded border"
            />
            Empleado activo
          </label>

          <div className="flex justify-end gap-2 pt-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancelar
            </Button>
            <Button type="submit" disabled={loading}>
              {loading ? "Guardando..." : "Guardar"}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
