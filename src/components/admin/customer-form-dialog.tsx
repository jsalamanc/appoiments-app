"use client";

import { standardSchemaResolver } from "@hookform/resolvers/standard-schema";
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
import { updateCustomer } from "@/lib/customers";
import { customerSchema, type CustomerValues } from "@/lib/validations";
import type { Customer } from "@/types/database";

type CustomerFormDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  customer: Customer;
  onSuccess: (customer: Customer) => void;
};

export function CustomerFormDialog({
  open,
  onOpenChange,
  customer,
  onSuccess,
}: CustomerFormDialogProps) {
  const [loading, setLoading] = useState(false);

  const {
    register,
    handleSubmit,
    reset,
    formState: { errors },
  } = useForm<CustomerValues>({
    resolver: standardSchemaResolver(customerSchema),
    defaultValues: {
      name: customer.name,
      email: customer.email,
      phone: customer.phone,
      document: customer.document,
    },
  });

  useEffect(() => {
    if (open) {
      reset({
        name: customer.name,
        email: customer.email,
        phone: customer.phone,
        document: customer.document,
      });
    }
  }, [open, customer, reset]);

  const save = async (values: CustomerValues) => {
    setLoading(true);
    try {
      const updated = await updateCustomer(customer.id, values);
      toast.success("Cliente actualizado");
      onOpenChange(false);
      onSuccess(updated);
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
          <DialogTitle>Editar cliente</DialogTitle>
          <DialogDescription>
            Actualiza los datos de contacto y documento del cliente.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit(save)} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="customer-name">Nombre</Label>
            <Input id="customer-name" {...register("name")} />
            {errors.name && (
              <p className="text-xs text-destructive">{errors.name.message}</p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="customer-email">Correo</Label>
            <Input
              id="customer-email"
              type="email"
              autoComplete="email"
              {...register("email")}
            />
            {errors.email && (
              <p className="text-xs text-destructive">{errors.email.message}</p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="customer-phone">Teléfono</Label>
            <Input
              id="customer-phone"
              type="tel"
              autoComplete="tel"
              {...register("phone")}
            />
            {errors.phone && (
              <p className="text-xs text-destructive">{errors.phone.message}</p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="customer-document">Documento</Label>
            <Input id="customer-document" {...register("document")} />
            {errors.document && (
              <p className="text-xs text-destructive">
                {errors.document.message}
              </p>
            )}
          </div>

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
