import { z } from "zod";

const money = z
  .number({ error: "Precio inválido" })
  .min(0, "Precio inválido");

const quantity = z
  .number({ error: "Cantidad inválida" })
  .int("Debe ser un número entero")
  .min(1, "Cantidad mínima 1");

export const loginSchema = z.object({
  username: z.string().min(2, "Ingresa tu usuario"),
  password: z.string().min(6, "Mínimo 6 caracteres"),
  rememberMe: z.boolean(),
});

export const onboardingSchema = z
  .object({
    businessName: z.string().min(2, "Nombre del negocio requerido"),
    ownerName: z.string().min(2, "Tu nombre es requerido"),
    ownerUsername: z.string().min(2, "Elige un usuario"),
    ownerPassword: z.string().min(6, "Mínimo 6 caracteres"),
    confirmPassword: z.string().min(6, "Confirma tu contraseña"),
  })
  .refine((data) => data.ownerPassword === data.confirmPassword, {
    message: "Las contraseñas no coinciden",
    path: ["confirmPassword"],
  });

export const catalogStatusSchema = z.enum(["draft", "published"]);

export const productSchema = z.object({
  name: z.string().min(2, "Nombre requerido"),
  price: money,
  cost: money,
  stock: z
    .number({ error: "Stock inválido" })
    .int("Debe ser un número entero")
    .min(0, "Stock mínimo 0"),
  status: catalogStatusSchema,
});

export const serviceSchema = z.object({
  name: z.string().min(2, "Nombre requerido"),
  description: z.string().optional(),
  price: money,
  duration_minutes: z
    .number({ error: "Duración inválida" })
    .int("Debe ser un número entero")
    .min(5, "Mínimo 5 minutos"),
  status: catalogStatusSchema,
});

export const packageItemSchema = z.object({
  item_type: z.enum(["product", "service"]),
  item_id: z.string().min(1, "Selecciona un ítem"),
  quantity,
});

export const packageSchema = z.object({
  name: z.string().min(2, "Nombre requerido"),
  description: z.string().optional(),
  price: money,
  status: catalogStatusSchema,
  items: z
    .array(packageItemSchema)
    .min(1, "Agrega al menos un producto o servicio"),
});

/** Issue #2, Fase 5: email/document son opcionales (venta rápida, cliente
 * de paso) — `.or(z.literal(""))` deja pasar el campo vacío del formulario
 * sin fallar el `.email()`, y el backend ya trata una cadena vacía como
 * ausente (ver `repositories::customers::normalize_optional`). */
export const customerSchema = z.object({
  email: z.email("Ingresa un correo válido").or(z.literal("")).optional(),
  name: z.string().min(2, "Nombre requerido"),
  phone: z.string().min(7, "Teléfono requerido"),
  document: z.string().optional(),
});

export const saleCartItemSchema = z.object({
  item_type: z.enum(["product", "service", "package"]),
  item_id: z.string().min(1),
  name: z.string().min(1),
  unit_price: money,
  quantity,
});

export const saleSchema = z.object({
  payment_method: z.enum(["cash", "transfer"]),
  items: z.array(saleCartItemSchema).min(1, "Agrega al menos un ítem"),
});

export const employeeSchema = z.object({
  name: z.string().min(2, "Nombre requerido"),
  color: z
    .string()
    .regex(/^#[0-9A-Fa-f]{6}$/, "Color inválido"),
  active: z.boolean(),
});

export const appointmentSchema = z.object({
  employee_id: z.string().min(1, "Selecciona un empleado"),
  customer_id: z.string().min(1, "Selecciona un cliente"),
  service_id: z.string().min(1, "Selecciona un servicio"),
  starts_at: z.string().min(1, "Fecha y hora requeridas"),
  notes: z.string(),
});

export type LoginValues = z.infer<typeof loginSchema>;
export type OnboardingValues = z.infer<typeof onboardingSchema>;
export type ProductValues = z.infer<typeof productSchema>;
export type ServiceValues = z.infer<typeof serviceSchema>;
export type PackageValues = z.infer<typeof packageSchema>;
export type CustomerValues = z.infer<typeof customerSchema>;
export type SaleValues = z.infer<typeof saleSchema>;
export type SaleCartItemValues = z.infer<typeof saleCartItemSchema>;
export type EmployeeValues = z.infer<typeof employeeSchema>;
export type AppointmentValues = z.infer<typeof appointmentSchema>;
