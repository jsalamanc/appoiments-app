import { z } from "zod";

const money = z
  .number({ error: "Precio inválido" })
  .min(0, "Precio inválido");

const quantity = z
  .number({ error: "Cantidad inválida" })
  .int("Debe ser un número entero")
  .min(1, "Cantidad mínima 1");

export const loginSchema = z.object({
  email: z.email("Ingresa un correo válido"),
  password: z.string().min(6, "Mínimo 6 caracteres"),
});

export const registerSchema = z
  .object({
    centerName: z.string().min(2, "Nombre del centro requerido"),
    email: z.email("Ingresa un correo válido"),
    password: z.string().min(6, "Mínimo 6 caracteres"),
    confirmPassword: z.string().min(6, "Confirma tu contraseña"),
  })
  .refine((data) => data.password === data.confirmPassword, {
    message: "Las contraseñas no coinciden",
    path: ["confirmPassword"],
  });

export const catalogStatusSchema = z.enum(["draft", "published"]);

export const productSchema = z.object({
  name: z.string().min(2, "Nombre requerido"),
  price: money,
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

export const customerSchema = z.object({
  email: z.email("Ingresa un correo válido"),
  name: z.string().min(2, "Nombre requerido"),
  phone: z.string().min(7, "Teléfono requerido"),
  document: z.string().min(5, "Documento requerido"),
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
export type RegisterValues = z.infer<typeof registerSchema>;
export type ProductValues = z.infer<typeof productSchema>;
export type ServiceValues = z.infer<typeof serviceSchema>;
export type PackageValues = z.infer<typeof packageSchema>;
export type CustomerValues = z.infer<typeof customerSchema>;
export type SaleValues = z.infer<typeof saleSchema>;
export type SaleCartItemValues = z.infer<typeof saleCartItemSchema>;
export type EmployeeValues = z.infer<typeof employeeSchema>;
export type AppointmentValues = z.infer<typeof appointmentSchema>;
