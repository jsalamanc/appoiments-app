export type AuditEntry = {
  id: string;
  occurred_at: string;
  actor_user_id: string | null;
  actor_name: string | null;
  actor_username: string | null;
  actor_role: string | null;
  action: string;
  entity_type: string;
  entity_id: string | null;
  summary: string;
  before: Record<string, unknown> | unknown[] | null;
  after: Record<string, unknown> | unknown[] | null;
  changes: Record<string, { from: unknown; to: unknown }> | null;
};

export type ListAuditResult = {
  items: AuditEntry[];
  total: number;
  page: number;
  total_pages: number;
  page_size: number;
};

export type AuditFilters = {
  from?: string;
  to?: string;
  actor_user_id?: string;
  entity_type?: string;
  action?: string;
  search?: string;
  page?: number;
  page_size?: number;
};

export const AUDIT_ENTITY_TYPES: { value: string; label: string }[] = [
  { value: "sale", label: "Ventas" },
  { value: "product", label: "Productos e inventario" },
  { value: "service", label: "Servicios" },
  { value: "package", label: "Paquetes" },
  { value: "customer", label: "Clientes" },
  { value: "employee", label: "Empleados" },
  { value: "appointment", label: "Citas" },
  { value: "cash_session", label: "Caja" },
  { value: "user", label: "Usuarios" },
  { value: "auth", label: "Inicios de sesión" },
  { value: "business_profile", label: "Negocio y módulos" },
  { value: "business_hours", label: "Horario" },
  { value: "storage", label: "Almacenamiento" },
  { value: "data", label: "Exportación de datos" },
];

const ACTION_LABELS: Record<string, string> = {
  "sale.create": "Venta registrada",
  "sale.void": "Venta anulada",
  "sale.link_appointment": "Venta vinculada a cita",
  "product.create": "Producto creado",
  "product.update": "Producto modificado",
  "product.delete": "Producto deshabilitado",
  "product.stock_adjust": "Ajuste de inventario",
  "service.create": "Servicio creado",
  "service.update": "Servicio modificado",
  "service.delete": "Servicio deshabilitado",
  "package.create": "Paquete creado",
  "package.update": "Paquete modificado",
  "package.delete": "Paquete deshabilitado",
  "customer.create": "Cliente creado",
  "customer.update": "Cliente modificado",
  "employee.create": "Empleado creado",
  "employee.update": "Empleado modificado",
  "employee.delete": "Empleado eliminado",
  "employee.availability_update": "Disponibilidad modificada",
  "employee.commission_update": "Comisiones modificadas",
  "appointment.create": "Cita creada",
  "appointment.update": "Cita modificada",
  "appointment.set_status": "Estado de cita cambiado",
  "appointment.complete_with_sale": "Cita completada con venta",
  "cash_session.open": "Caja abierta",
  "cash_session.close": "Caja cerrada",
  "user.create": "Usuario creado",
  "user.set_active": "Usuario activado/desactivado",
  "user.change_password": "Contraseña cambiada",
  "user.recover_owner_password": "Contraseña del dueño recuperada",
  "auth.login": "Inicio de sesión",
  "auth.login_failed": "Inicio de sesión fallido",
  "auth.logout": "Cierre de sesión",
  "business_profile.update": "Negocio modificado",
  "business_profile.modules_update": "Módulos modificados",
  "business_hours.update": "Horario modificado",
  "storage.switch_mode": "Almacenamiento cambiado",
  "data.export": "Respaldo exportado",
};

export function actionLabel(action: string) {
  return ACTION_LABELS[action] ?? action;
}

export function actionTone(
  action: string
): "default" | "secondary" | "destructive" {
  if (
    action.endsWith(".delete") ||
    action === "sale.void" ||
    action === "auth.login_failed"
  ) {
    return "destructive";
  }
  if (action.endsWith(".create") || action === "auth.login") return "default";
  return "secondary";
}

const FIELD_LABELS: Record<string, string> = {
  name: "Nombre",
  name_snapshot: "Nombre",
  description: "Descripción",
  price: "Precio",
  cost: "Costo",
  stock: "Stock",
  status: "Estado",
  photo_path: "Foto",
  phone: "Teléfono",
  email: "Correo",
  document: "Documento",
  color: "Color",
  active: "Activo",
  role: "Rol",
  username: "Usuario",
  total: "Total",
  discount_amount: "Descuento",
  payment_method: "Medio de pago",
  voided_at: "Anulada el",
  void_reason: "Motivo de anulación",
  customer_id: "Cliente (id)",
  employee_id: "Empleado (id)",
  service_id: "Servicio (id)",
  sale_id: "Venta (id)",
  appointment_id: "Cita (id)",
  starts_at: "Inicio",
  ends_at: "Fin",
  notes: "Notas",
  opening_amount: "Monto inicial",
  counted_amount: "Monto contado",
  expected_amount: "Monto esperado",
  difference: "Diferencia",
  closed_at: "Cerrada el",
  opened_by: "Abierta por (id)",
  closed_by: "Cerrada por (id)",
  is_open: "Abierto",
  opens_at: "Abre",
  closes_at: "Cierra",
  module_employees: "Módulo empleados",
  module_appointments: "Módulo citas",
  module_services: "Módulo servicios",
  module_packages: "Módulo paquetes",
  module_reports: "Módulo reportes",
  sale_items: "Ítems de la venta",
  sale_payments: "Pagos",
  inventory_movements: "Movimientos de inventario",
  package_items: "Ítems del paquete",
  employee_commission_rates: "Comisiones",
  employee_availability: "Disponibilidad",
  rows: "Filas",
  deleted_at: "Deshabilitado el",
  product_name: "Producto",
  customer_name: "Cliente",
  product_id: "Producto (id)",
  movement_type: "Tipo de movimiento",
  quantity_delta: "Cambio de stock",
  stock_after: "Stock resultante",
};

export function fieldLabel(field: string) {
  return FIELD_LABELS[field] ?? field;
}

const MONEY_FIELDS = new Set([
  "price",
  "cost",
  "total",
  "discount_amount",
  "unit_price",
  "line_total",
  "opening_amount",
  "counted_amount",
  "expected_amount",
  "difference",
]);

export function isMoneyField(field: string) {
  return MONEY_FIELDS.has(field);
}

const PAYMENT_LABELS: Record<string, string> = {
  cash: "Efectivo",
  transfer: "Transferencia",
  card: "Tarjeta",
};

const MOVEMENT_LABELS: Record<string, string> = {
  initial: "Stock inicial",
  adjustment: "Ajuste",
  sale: "Venta",
  return: "Devolución",
};

/** Texto de una fila hija guardada dentro de la foto (ítems de venta,
 * pagos, movimientos de inventario) para mostrarla en una línea legible. */
export function formatRelatedRow(
  field: string,
  row: Record<string, unknown>,
  money: (value: number) => string
): string | null {
  if (field === "sale_items") {
    return `${row.quantity}× ${row.name_snapshot} — ${money(Number(row.line_total))}`;
  }
  if (field === "sale_payments") {
    const method = PAYMENT_LABELS[String(row.method)] ?? String(row.method);
    return `${method}: ${money(Number(row.amount))}`;
  }
  if (field === "inventory_movements") {
    const delta = Number(row.quantity_delta);
    const sign = delta > 0 ? "+" : "";
    const kind = MOVEMENT_LABELS[String(row.movement_type)] ?? String(row.movement_type);
    const name = row.product_name ?? row.product_id;
    return `${name}: ${sign}${delta} (${kind}, stock ${row.stock_after})`;
  }
  if (field === "package_items") {
    return `${row.quantity}× ${row.product_id ?? row.service_id}`;
  }
  return null;
}
