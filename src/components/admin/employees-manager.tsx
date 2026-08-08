"use client";

import { Pencil, Plus, Trash2, Users } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { DeleteConfirm } from "@/components/admin/delete-confirm";
import { EmployeeFormDialog } from "@/components/admin/employee-form-dialog";
import { EmptyState } from "@/components/admin/empty-state";
import { FadeIn, StaggerChildren, StaggerItem } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { deleteEmployee, listEmployees } from "@/lib/employees";
import type { Employee } from "@/types/database";

export function EmployeesManager() {
  const router = useRouter();
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [loading, setLoading] = useState(true);
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<Employee | null>(null);
  const [toDelete, setToDelete] = useState<Employee | null>(null);
  const [deleting, setDeleting] = useState(false);

  const load = async () => {
    setLoading(true);
    try {
      setEmployees(await listEmployees());
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Error al cargar");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  const refresh = () => {
    void load();
    router.refresh();
  };

  const handleDelete = async () => {
    if (!toDelete) return;
    setDeleting(true);
    try {
      await deleteEmployee(toDelete.id);
      toast.success("Empleado eliminado");
      setToDelete(null);
      refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo eliminar");
    } finally {
      setDeleting(false);
    }
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Agenda</p>
          <h1 className="font-heading text-4xl font-semibold">Empleados</h1>
          <p className="mt-1 text-muted-foreground">
            Crea calendarios por profesional para agendar citas.
          </p>
        </div>
        <Button
          onClick={() => {
            setSelected(null);
            setOpen(true);
          }}
        >
          <Plus data-icon="inline-start" />
          Nuevo empleado
        </Button>
      </FadeIn>

      {loading ? (
        <p className="text-sm text-muted-foreground">Cargando empleados...</p>
      ) : employees.length === 0 ? (
        <EmptyState
          title="Aún no hay empleados"
          description="Agrega al menos un empleado para empezar a agendar citas."
          actionLabel="Crear empleado"
          onAction={() => {
            setSelected(null);
            setOpen(true);
          }}
        />
      ) : (
        <StaggerChildren className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {employees.map((employee) => (
            <StaggerItem key={employee.id}>
              <article className="space-y-3 rounded-3xl border border-border/70 bg-card shadow-sm p-4">
                <div className="flex items-start justify-between gap-2">
                  <div className="flex items-center gap-3">
                    <span
                      className="flex size-10 items-center justify-center rounded-2xl"
                      style={{ backgroundColor: `${employee.color}33` }}
                    >
                      <Users
                        className="size-4"
                        style={{ color: employee.color }}
                      />
                    </span>
                    <div>
                      <h3 className="font-heading text-xl font-semibold">
                        {employee.name}
                      </h3>
                      <Badge variant={employee.active ? "secondary" : "outline"}>
                        {employee.active ? "Activo" : "Inactivo"}
                      </Badge>
                    </div>
                  </div>
                  <span
                    className="mt-1 size-3 rounded-full"
                    style={{ backgroundColor: employee.color }}
                    title={employee.color}
                  />
                </div>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      setSelected(employee);
                      setOpen(true);
                    }}
                  >
                    <Pencil data-icon="inline-start" />
                    Editar
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setToDelete(employee)}
                  >
                    <Trash2 data-icon="inline-start" />
                    Eliminar
                  </Button>
                </div>
              </article>
            </StaggerItem>
          ))}
        </StaggerChildren>
      )}

      <EmployeeFormDialog
        open={open}
        onOpenChange={setOpen}
        employee={selected}
        onSuccess={refresh}
      />

      <DeleteConfirm
        open={!!toDelete}
        onOpenChange={(value) => !value && setToDelete(null)}
        title="Eliminar empleado"
        description={`¿Seguro que quieres eliminar a “${toDelete?.name}”?`}
        loading={deleting}
        onConfirm={handleDelete}
      />
    </div>
  );
}
