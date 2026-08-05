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
import { Textarea } from "@/components/ui/textarea";
import {
  createAppointment,
  toLocalInputValue,
  updateAppointment,
} from "@/lib/appointments";
import {
  createCustomer,
  findCustomerByEmail,
  normalizeEmail,
} from "@/lib/customers";
import { formatCurrency, formatDuration } from "@/lib/format";
import {
  appointmentSchema,
  type AppointmentValues,
} from "@/lib/validations";
import type {
  AppointmentWithDetails,
  Customer,
  Employee,
  Service,
} from "@/types/database";

type AppointmentFormDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  appointment?: AppointmentWithDetails | null;
  defaultStartsAt?: Date | null;
  defaultEmployeeId?: string | null;
  employees: Employee[];
  services: Service[];
  onSuccess: () => void;
};

export function AppointmentFormDialog({
  open,
  onOpenChange,
  ...rest
}: AppointmentFormDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      {open ? (
        <AppointmentFormFields open={open} onOpenChange={onOpenChange} {...rest} />
      ) : null}
    </Dialog>
  );
}

function AppointmentFormFields({
  open,
  onOpenChange,
  appointment,
  defaultStartsAt,
  defaultEmployeeId,
  employees,
  services,
  onSuccess,
}: AppointmentFormDialogProps) {
  const [loading, setLoading] = useState(false);
  const [email, setEmail] = useState("");
  const [name, setName] = useState("");
  const [phone, setPhone] = useState("");
  const [customerDocument, setCustomerDocument] = useState("");
  const [customer, setCustomer] = useState<Customer | null>(null);
  const [lookingUp, setLookingUp] = useState(false);

  const {
    register,
    handleSubmit,
    reset,
    setValue,
    watch,
    formState: { errors },
  } = useForm<AppointmentValues>({
    resolver: standardSchemaResolver(appointmentSchema),
    defaultValues: {
      employee_id: "",
      customer_id: "",
      service_id: "",
      starts_at: "",
      notes: "",
    },
  });

  const employeeId = watch("employee_id");
  const serviceId = watch("service_id");

  useEffect(() => {
    if (!open) return;

    if (appointment) {
      reset({
        employee_id: appointment.employee_id,
        customer_id: appointment.customer_id,
        service_id: appointment.service_id,
        starts_at: toLocalInputValue(new Date(appointment.starts_at)),
        notes: appointment.notes ?? "",
      });
      setCustomer({
        id: appointment.customers.id,
        center_id: appointment.center_id,
        name: appointment.customers.name,
        email: appointment.customers.email,
        phone: appointment.customers.phone,
        document: appointment.customers.document,
        created_at: "",
        updated_at: "",
      });
      setEmail(appointment.customers.email);
      setName(appointment.customers.name);
      setPhone(appointment.customers.phone);
      setCustomerDocument(appointment.customers.document);
      return;
    }

    const start = defaultStartsAt ? new Date(defaultStartsAt) : new Date();
    start.setMinutes(0, 0, 0);
    reset({
      employee_id: defaultEmployeeId ?? employees[0]?.id ?? "",
      customer_id: "",
      service_id: services[0]?.id ?? "",
      starts_at: toLocalInputValue(start),
      notes: "",
    });
    setCustomer(null);
    setEmail("");
    setName("");
    setPhone("");
    setCustomerDocument("");
  }, [
    open,
    appointment,
    defaultStartsAt,
    defaultEmployeeId,
    employees,
    services,
    reset,
  ]);

  useEffect(() => {
    if (customer) {
      setValue("customer_id", customer.id, { shouldValidate: true });
    }
  }, [customer, setValue]);

  const lookupCustomer = async () => {
    const normalized = normalizeEmail(email);
    if (!normalized.includes("@")) {
      toast.error("Ingresa un correo válido");
      return;
    }

    setLookingUp(true);
    try {
      const found = await findCustomerByEmail(normalized);
      setEmail(normalized);
      if (found) {
        setCustomer(found);
        setName(found.name);
        setPhone(found.phone);
        setCustomerDocument(found.document);
        toast.success("Cliente encontrado");
      } else {
        setCustomer(null);
        setValue("customer_id", "", { shouldValidate: true });
        toast.message("Cliente nuevo", {
          description: "Completa los datos para crearlo al guardar la cita.",
        });
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Error al buscar");
    } finally {
      setLookingUp(false);
    }
  };

  const ensureCustomer = async () => {
    if (customer) return customer;

    if (
      !email.trim() ||
      !name.trim() ||
      !phone.trim() ||
      !customerDocument.trim()
    ) {
      throw new Error("Completa los datos del cliente");
    }

    const created = await createCustomer({
      email,
      name,
      phone,
      document: customerDocument,
    });
    setCustomer(created);
    return created;
  };

  const save = async (values: AppointmentValues) => {
    setLoading(true);
    try {
      const readyCustomer = await ensureCustomer();
      const startsAt = new Date(values.starts_at).toISOString();

      if (appointment) {
        await updateAppointment({
          appointmentId: appointment.id,
          employeeId: values.employee_id,
          customerId: readyCustomer.id,
          serviceId: values.service_id,
          startsAt,
          notes: values.notes,
        });
        toast.success("Cita actualizada");
      } else {
        await createAppointment({
          employeeId: values.employee_id,
          customerId: readyCustomer.id,
          serviceId: values.service_id,
          startsAt,
          notes: values.notes,
        });
        toast.success("Cita creada");
      }

      onOpenChange(false);
      onSuccess();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Error al guardar");
    } finally {
      setLoading(false);
    }
  };

  const activeEmployees = employees.filter((e) => e.active);
  const selectedService = services.find((s) => s.id === serviceId);

  return (
    <DialogContent className="flex max-h-[90vh] flex-col gap-0 overflow-hidden p-0 sm:max-w-lg">
      <DialogHeader className="shrink-0 px-4 pt-4">
        <DialogTitle>
          {appointment ? "Editar cita" : "Nueva cita"}
        </DialogTitle>
        <DialogDescription>
          Agenda un servicio con cliente y profesional.
        </DialogDescription>
      </DialogHeader>

      <form
        onSubmit={handleSubmit(save)}
        className="flex min-h-0 flex-1 flex-col"
      >
        <div className="space-y-4 overflow-y-auto px-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="appt-email">Correo del cliente</Label>
            <div className="flex gap-2">
              <Input
                id="appt-email"
                type="email"
                value={email}
                onChange={(e) => {
                  setEmail(e.target.value);
                  setCustomer(null);
                  setValue("customer_id", "", { shouldValidate: true });
                }}
                placeholder="cliente@correo.com"
              />
              <Button
                type="button"
                variant="outline"
                disabled={lookingUp}
                onClick={() => void lookupCustomer()}
              >
                {lookingUp ? "..." : "Buscar"}
              </Button>
            </div>
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="appt-name">Nombre</Label>
              <Input
                id="appt-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                disabled={!!customer}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="appt-phone">Teléfono</Label>
              <Input
                id="appt-phone"
                value={phone}
                onChange={(e) => setPhone(e.target.value)}
                disabled={!!customer}
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="appt-document">Documento</Label>
            <Input
              id="appt-document"
              value={customerDocument}
              onChange={(e) => setCustomerDocument(e.target.value)}
              disabled={!!customer}
            />
            {errors.customer_id && (
              <p className="text-xs text-destructive">
                {errors.customer_id.message}
              </p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="appt-employee">Empleado</Label>
            <select
              id="appt-employee"
              className="flex h-9 w-full rounded-xl border border-input bg-transparent px-3 text-sm"
              value={employeeId}
              onChange={(e) =>
                setValue("employee_id", e.target.value, {
                  shouldValidate: true,
                })
              }
            >
              <option value="">Selecciona...</option>
              {activeEmployees.map((employee) => (
                <option key={employee.id} value={employee.id}>
                  {employee.name}
                </option>
              ))}
            </select>
            {errors.employee_id && (
              <p className="text-xs text-destructive">
                {errors.employee_id.message}
              </p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="appt-service">Servicio</Label>
            <select
              id="appt-service"
              className="flex h-9 w-full rounded-xl border border-input bg-transparent px-3 text-sm"
              value={serviceId}
              onChange={(e) =>
                setValue("service_id", e.target.value, {
                  shouldValidate: true,
                })
              }
            >
              <option value="">Selecciona...</option>
              {services.map((service) => (
                <option key={service.id} value={service.id}>
                  {service.name} · {formatCurrency(Number(service.price))} ·{" "}
                  {formatDuration(service.duration_minutes)}
                </option>
              ))}
            </select>
            {errors.service_id && (
              <p className="text-xs text-destructive">
                {errors.service_id.message}
              </p>
            )}
            {selectedService && (
              <p className="text-xs text-muted-foreground">
                Duración: {formatDuration(selectedService.duration_minutes)}
              </p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="appt-starts">Inicio</Label>
            <Input
              id="appt-starts"
              type="datetime-local"
              {...register("starts_at")}
            />
            {errors.starts_at && (
              <p className="text-xs text-destructive">
                {errors.starts_at.message}
              </p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="appt-notes">Notas</Label>
            <Textarea
              id="appt-notes"
              rows={2}
              placeholder="Opcional"
              {...register("notes")}
            />
          </div>
        </div>

        <div className="flex shrink-0 justify-end gap-2 border-t bg-muted/50 p-4">
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
          >
            Cancelar
          </Button>
          <Button type="submit" disabled={loading || services.length === 0}>
            {loading ? "Guardando..." : "Guardar cita"}
          </Button>
        </div>
      </form>
    </DialogContent>
  );
}
