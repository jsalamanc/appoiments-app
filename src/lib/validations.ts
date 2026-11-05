import { z } from "zod";

// Issue #2, Fase 6: el backend guarda el dinero como entero (pesos, sin
// centavos — ver migración 0007_money_as_integers.sql), así que mandar un
// valor fraccionario ya no es solo impreciso, directamente falla al
// deserializar el comando de Tauri (`i64` no acepta `99.5`).
const money = z
  .number({ error: "Precio inválido" })
  .int("No se admiten centavos, solo pesos enteros")
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

// El User Pool de Cognito está configurado con `username_attributes =
// ["email"]` (ver infra/modules/cognito/main.tf del backend): el correo ES
// el usuario, Cognito no tiene un "usuario" separado. Por eso estos schemas
// piden `email` únicamente — nunca un campo "usuario" aparte.
//
// Cognito exige mayúscula, minúscula y número en la contraseña (política del
// User Pool) — se valida acá también para dar el error antes de llamar a la
// API en vez de solo después.
export const cognitoSignupSchema = z
  .object({
    email: z.email("Ingresa un correo válido"),
    password: z
      .string()
      .min(8, "Mínimo 8 caracteres")
      .regex(/[a-z]/, "Debe incluir una minúscula")
      .regex(/[A-Z]/, "Debe incluir una mayúscula")
      .regex(/[0-9]/, "Debe incluir un número"),
    confirmPassword: z.string().min(1, "Confirma tu contraseña"),
  })
  .refine((data) => data.password === data.confirmPassword, {
    message: "Las contraseñas no coinciden",
    path: ["confirmPassword"],
  });

export const cognitoConfirmSchema = z.object({
  code: z.string().min(1, "Ingresa el código que te enviamos por correo"),
});

export const cognitoLoginSchema = z.object({
  email: z.email("Ingresa un correo válido"),
  password: z.string().min(1, "Ingresa tu contraseña"),
});

export const cognitoForgotPasswordSchema = z.object({
  email: z.email("Ingresa un correo válido"),
});

export const cognitoResetPasswordSchema = z
  .object({
    code: z.string().min(1, "Ingresa el código que te enviamos por correo"),
    newPassword: z
      .string()
      .min(8, "Mínimo 8 caracteres")
      .regex(/[a-z]/, "Debe incluir una minúscula")
      .regex(/[A-Z]/, "Debe incluir una mayúscula")
      .regex(/[0-9]/, "Debe incluir un número"),
    confirmPassword: z.string().min(1, "Confirma tu nueva contraseña"),
  })
  .refine((data) => data.newPassword === data.confirmPassword, {
    message: "Las contraseñas no coinciden",
    path: ["confirmPassword"],
  });

// Recuperar la contraseña del dueño local reusando la cuenta de Cognito que
// configuró el negocio como prueba de identidad (ver
// commands::auth::auth_recover_owner_password) — no hay un "olvidé mi
// contraseña" propio del login local, a propósito, para no duplicar lo que
// ya construimos para Cognito.
export const recoverOwnerPasswordSchema = z
  .object({
    cognitoEmail: z.email("Ingresa un correo válido"),
    cognitoPassword: z.string().min(1, "Ingresa tu contraseña de Cognito"),
    localUsername: z.string().min(1, "Ingresa el usuario dueño a recuperar"),
    newPassword: z
      .string()
      .min(6, "Mínimo 6 caracteres"),
    confirmPassword: z.string().min(1, "Confirma tu nueva contraseña"),
  })
  .refine((data) => data.newPassword === data.confirmPassword, {
    message: "Las contraseñas no coinciden",
    path: ["confirmPassword"],
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
export type CognitoSignupValues = z.infer<typeof cognitoSignupSchema>;
export type CognitoConfirmValues = z.infer<typeof cognitoConfirmSchema>;
export type CognitoLoginValues = z.infer<typeof cognitoLoginSchema>;
export type CognitoForgotPasswordValues = z.infer<typeof cognitoForgotPasswordSchema>;
export type CognitoResetPasswordValues = z.infer<typeof cognitoResetPasswordSchema>;
export type RecoverOwnerPasswordValues = z.infer<typeof recoverOwnerPasswordSchema>;
export type ProductValues = z.infer<typeof productSchema>;
export type ServiceValues = z.infer<typeof serviceSchema>;
export type PackageValues = z.infer<typeof packageSchema>;
export type CustomerValues = z.infer<typeof customerSchema>;
export type SaleValues = z.infer<typeof saleSchema>;
export type SaleCartItemValues = z.infer<typeof saleCartItemSchema>;
export type EmployeeValues = z.infer<typeof employeeSchema>;
export type AppointmentValues = z.infer<typeof appointmentSchema>;
