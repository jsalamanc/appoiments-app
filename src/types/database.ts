// Tipos de datos locales (SQLite vía comandos Tauri). Ya no hay
// multi-tenant: no hay `center_id`, ni `centers`/`plans`/`center_members`.

export type Database = {
  public: {
    Tables: {
      business_profile: {
        Row: {
          name: string;
          created_at: string;
          updated_at: string;
        };
      };
      users: {
        Row: {
          id: string;
          name: string;
          username: string;
          role: "owner" | "staff";
          active: boolean;
          created_at: string;
          updated_at: string;
        };
      };
      products: {
        Row: {
          id: string;
          name: string;
          price: number;
          photo_path: string | null;
          status: "draft" | "published";
          stock: number;
          created_at: string;
          updated_at: string;
        };
      };
      inventory_movements: {
        Row: {
          id: string;
          product_id: string;
          movement_type: "initial" | "adjustment" | "sale" | "return";
          quantity_delta: number;
          stock_after: number;
          sale_id: string | null;
          note: string | null;
          created_at: string;
        };
      };
      services: {
        Row: {
          id: string;
          name: string;
          description: string | null;
          price: number;
          duration_minutes: number;
          photo_path: string | null;
          status: "draft" | "published";
          created_at: string;
          updated_at: string;
        };
      };
      packages: {
        Row: {
          id: string;
          name: string;
          description: string | null;
          price: number;
          photo_path: string | null;
          status: "draft" | "published";
          created_at: string;
          updated_at: string;
        };
      };
      package_items: {
        Row: {
          id: string;
          package_id: string;
          item_type: "product" | "service";
          product_id: string | null;
          service_id: string | null;
          quantity: number;
          created_at: string;
        };
      };
      customers: {
        Row: {
          id: string;
          name: string;
          phone: string;
          email: string;
          document: string;
          created_at: string;
          updated_at: string;
        };
      };
      employees: {
        Row: {
          id: string;
          name: string;
          color: string;
          active: boolean;
          created_at: string;
          updated_at: string;
        };
      };
      appointments: {
        Row: {
          id: string;
          employee_id: string;
          customer_id: string;
          service_id: string | null;
          service_name_snapshot: string;
          starts_at: string;
          ends_at: string;
          status: "scheduled" | "completed" | "cancelled" | "no_show";
          notes: string | null;
          sale_id: string | null;
          created_at: string;
          updated_at: string;
        };
      };
      sales: {
        Row: {
          id: string;
          customer_id: string;
          user_id: string | null;
          payment_method: "cash" | "transfer" | "card" | "mixed";
          total: number;
          discount_amount: number;
          sold_at: string;
          created_at: string;
          appointment_id: string | null;
          voided_at: string | null;
          void_reason: string | null;
        };
      };
      sale_items: {
        Row: {
          id: string;
          sale_id: string;
          item_type: "product" | "service" | "package";
          product_id: string | null;
          service_id: string | null;
          package_id: string | null;
          name_snapshot: string;
          unit_price: number;
          quantity: number;
          line_total: number;
          created_at: string;
        };
      };
      sale_payments: {
        Row: {
          id: string;
          sale_id: string;
          method: "cash" | "transfer" | "card";
          amount: number;
          created_at: string;
        };
      };
    };
  };
};

export type BusinessProfile = Database["public"]["Tables"]["business_profile"]["Row"];
export type AppUser = Database["public"]["Tables"]["users"]["Row"];
export type Product = Database["public"]["Tables"]["products"]["Row"];
export type Service = Database["public"]["Tables"]["services"]["Row"];
export type Package = Database["public"]["Tables"]["packages"]["Row"];
export type PackageItem = Database["public"]["Tables"]["package_items"]["Row"];
export type InventoryMovement =
  Database["public"]["Tables"]["inventory_movements"]["Row"];
export type Customer = Database["public"]["Tables"]["customers"]["Row"];
export type Employee = Database["public"]["Tables"]["employees"]["Row"];
export type Appointment = Database["public"]["Tables"]["appointments"]["Row"];
export type Sale = Database["public"]["Tables"]["sales"]["Row"];
export type SaleItem = Database["public"]["Tables"]["sale_items"]["Row"];
export type SalePayment = Database["public"]["Tables"]["sale_payments"]["Row"];
export type PaymentMethod = SalePayment["method"];
export type SaleItemType = SaleItem["item_type"];
export type InventoryMovementType = InventoryMovement["movement_type"];
export type AppointmentStatus = Appointment["status"];

export type PackageWithItems = Package & {
  package_items: (PackageItem & {
    products: Pick<Product, "id" | "name" | "price"> | null;
    services: Pick<Service, "id" | "name" | "price" | "duration_minutes"> | null;
  })[];
};

export type SaleWithDetails = Sale & {
  customers: Pick<Customer, "id" | "name" | "email" | "phone" | "document">;
  sale_items: SaleItem[];
};

export type AppointmentWithDetails = Appointment & {
  employees: Pick<Employee, "id" | "name" | "color" | "active">;
  customers: Pick<Customer, "id" | "name" | "email" | "phone" | "document">;
  services: Pick<
    Service,
    "id" | "name" | "price" | "duration_minutes" | "status"
  > | null;
};
