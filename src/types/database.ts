export type Json =
  | string
  | number
  | boolean
  | null
  | { [key: string]: Json | undefined }
  | Json[];

export type Database = {
  public: {
    Tables: {
      centers: {
        Row: {
          id: string;
          owner_id: string;
          name: string;
          slug: string;
          modules: {
            employees: boolean;
            appointments: boolean;
            services: boolean;
            packages: boolean;
            public_booking: boolean;
            reports: boolean;
          };
          plan_id: string;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          owner_id: string;
          name: string;
          slug: string;
          modules?: {
            employees: boolean;
            appointments: boolean;
            services: boolean;
            packages: boolean;
            public_booking: boolean;
            reports: boolean;
          };
          plan_id?: string;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          owner_id?: string;
          name?: string;
          slug?: string;
          modules?: {
            employees: boolean;
            appointments: boolean;
            services: boolean;
            packages: boolean;
            public_booking: boolean;
            reports: boolean;
          };
          plan_id?: string;
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [];
      };
      plans: {
        Row: {
          id: string;
          name: string;
          price_cents: number;
          currency: string;
          benefits: Json;
          allowed_modules: {
            employees: boolean;
            appointments: boolean;
            services: boolean;
            packages: boolean;
            public_booking: boolean;
            reports: boolean;
          };
          max_products: number | null;
          sort_order: number;
          active: boolean;
          created_at: string;
        };
        Insert: {
          id: string;
          name: string;
          price_cents?: number;
          currency?: string;
          benefits?: Json;
          allowed_modules?: {
            employees: boolean;
            appointments: boolean;
            services: boolean;
            packages: boolean;
            public_booking: boolean;
            reports: boolean;
          };
          max_products?: number | null;
          sort_order?: number;
          active?: boolean;
          created_at?: string;
        };
        Update: {
          id?: string;
          name?: string;
          price_cents?: number;
          currency?: string;
          benefits?: Json;
          allowed_modules?: {
            employees: boolean;
            appointments: boolean;
            services: boolean;
            packages: boolean;
            public_booking: boolean;
            reports: boolean;
          };
          max_products?: number | null;
          sort_order?: number;
          active?: boolean;
          created_at?: string;
        };
        Relationships: [];
      };
      center_members: {
        Row: {
          id: string;
          center_id: string;
          user_id: string;
          role: "owner" | "reception";
          created_at: string;
        };
        Insert: {
          id?: string;
          center_id: string;
          user_id: string;
          role: "owner" | "reception";
          created_at?: string;
        };
        Update: {
          id?: string;
          center_id?: string;
          user_id?: string;
          role?: "owner" | "reception";
          created_at?: string;
        };
        Relationships: [];
      };
      center_invites: {
        Row: {
          id: string;
          center_id: string;
          email: string | null;
          token: string;
          role: "reception";
          created_by: string;
          expires_at: string;
          accepted_at: string | null;
          accepted_by: string | null;
          created_at: string;
        };
        Insert: {
          id?: string;
          center_id: string;
          email?: string | null;
          token: string;
          role?: "reception";
          created_by: string;
          expires_at?: string;
          accepted_at?: string | null;
          accepted_by?: string | null;
          created_at?: string;
        };
        Update: {
          id?: string;
          center_id?: string;
          email?: string | null;
          token?: string;
          role?: "reception";
          created_by?: string;
          expires_at?: string;
          accepted_at?: string | null;
          accepted_by?: string | null;
          created_at?: string;
        };
        Relationships: [];
      };
      products: {
        Row: {
          id: string;
          center_id: string;
          name: string;
          price: number;
          photo_url: string | null;
          status: "draft" | "published";
          stock: number;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          center_id: string;
          name: string;
          price: number;
          photo_url?: string | null;
          status?: "draft" | "published";
          stock?: number;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          center_id?: string;
          name?: string;
          price?: number;
          photo_url?: string | null;
          status?: "draft" | "published";
          stock?: number;
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [];
      };
      inventory_movements: {
        Row: {
          id: string;
          center_id: string;
          product_id: string;
          movement_type: "initial" | "adjustment" | "sale";
          quantity_delta: number;
          stock_after: number;
          sale_id: string | null;
          note: string | null;
          created_at: string;
        };
        Insert: {
          id?: string;
          center_id: string;
          product_id: string;
          movement_type: "initial" | "adjustment" | "sale";
          quantity_delta: number;
          stock_after: number;
          sale_id?: string | null;
          note?: string | null;
          created_at?: string;
        };
        Update: {
          id?: string;
          center_id?: string;
          product_id?: string;
          movement_type?: "initial" | "adjustment" | "sale";
          quantity_delta?: number;
          stock_after?: number;
          sale_id?: string | null;
          note?: string | null;
          created_at?: string;
        };
        Relationships: [];
      };
      services: {
        Row: {
          id: string;
          center_id: string;
          name: string;
          description: string | null;
          price: number;
          duration_minutes: number;
          photo_url: string | null;
          status: "draft" | "published";
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          center_id: string;
          name: string;
          description?: string | null;
          price: number;
          duration_minutes: number;
          photo_url?: string | null;
          status?: "draft" | "published";
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          center_id?: string;
          name?: string;
          description?: string | null;
          price?: number;
          duration_minutes?: number;
          photo_url?: string | null;
          status?: "draft" | "published";
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [];
      };
      packages: {
        Row: {
          id: string;
          center_id: string;
          name: string;
          description: string | null;
          price: number;
          photo_url: string | null;
          status: "draft" | "published";
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          center_id: string;
          name: string;
          description?: string | null;
          price: number;
          photo_url?: string | null;
          status?: "draft" | "published";
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          center_id?: string;
          name?: string;
          description?: string | null;
          price?: number;
          photo_url?: string | null;
          status?: "draft" | "published";
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [];
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
        Insert: {
          id?: string;
          package_id: string;
          item_type: "product" | "service";
          product_id?: string | null;
          service_id?: string | null;
          quantity?: number;
          created_at?: string;
        };
        Update: {
          id?: string;
          package_id?: string;
          item_type?: "product" | "service";
          product_id?: string | null;
          service_id?: string | null;
          quantity?: number;
          created_at?: string;
        };
        Relationships: [];
      };
      customers: {
        Row: {
          id: string;
          center_id: string;
          name: string;
          phone: string;
          email: string;
          document: string;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          center_id: string;
          name: string;
          phone: string;
          email: string;
          document: string;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          center_id?: string;
          name?: string;
          phone?: string;
          email?: string;
          document?: string;
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [];
      };
      employees: {
        Row: {
          id: string;
          center_id: string;
          name: string;
          color: string;
          active: boolean;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          center_id: string;
          name: string;
          color?: string;
          active?: boolean;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          center_id?: string;
          name?: string;
          color?: string;
          active?: boolean;
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [];
      };
      appointments: {
        Row: {
          id: string;
          center_id: string;
          employee_id: string;
          customer_id: string;
          service_id: string;
          starts_at: string;
          ends_at: string;
          status: "scheduled" | "completed" | "cancelled" | "no_show";
          notes: string | null;
          sale_id: string | null;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          center_id: string;
          employee_id: string;
          customer_id: string;
          service_id: string;
          starts_at: string;
          ends_at: string;
          status?: "scheduled" | "completed" | "cancelled" | "no_show";
          notes?: string | null;
          sale_id?: string | null;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          center_id?: string;
          employee_id?: string;
          customer_id?: string;
          service_id?: string;
          starts_at?: string;
          ends_at?: string;
          status?: "scheduled" | "completed" | "cancelled" | "no_show";
          notes?: string | null;
          sale_id?: string | null;
          created_at?: string;
          updated_at?: string;
        };
        Relationships: [];
      };
      sales: {
        Row: {
          id: string;
          center_id: string;
          customer_id: string;
          payment_method: "cash" | "transfer";
          total: number;
          sold_at: string;
          created_at: string;
          appointment_id: string | null;
        };
        Insert: {
          id?: string;
          center_id: string;
          customer_id: string;
          payment_method: "cash" | "transfer";
          total: number;
          sold_at?: string;
          created_at?: string;
          appointment_id?: string | null;
        };
        Update: {
          id?: string;
          center_id?: string;
          customer_id?: string;
          payment_method?: "cash" | "transfer";
          total?: number;
          sold_at?: string;
          created_at?: string;
          appointment_id?: string | null;
        };
        Relationships: [];
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
        Insert: {
          id?: string;
          sale_id: string;
          item_type: "product" | "service" | "package";
          product_id?: string | null;
          service_id?: string | null;
          package_id?: string | null;
          name_snapshot: string;
          unit_price: number;
          quantity: number;
          line_total: number;
          created_at?: string;
        };
        Update: {
          id?: string;
          sale_id?: string;
          item_type?: "product" | "service" | "package";
          product_id?: string | null;
          service_id?: string | null;
          package_id?: string | null;
          name_snapshot?: string;
          unit_price?: number;
          quantity?: number;
          line_total?: number;
          created_at?: string;
        };
        Relationships: [];
      };
    };
    Views: Record<string, never>;
    Functions: {
      current_center_id: {
        Args: Record<string, never>;
        Returns: string;
      };
      current_member_role: {
        Args: Record<string, never>;
        Returns: string;
      };
      is_center_owner: {
        Args: Record<string, never>;
        Returns: boolean;
      };
      create_center_invite: {
        Args: { p_email?: string | null };
        Returns: {
          id: string;
          token: string;
          expires_at: string;
        }[];
      };
      revoke_center_member: {
        Args: { p_member_id: string };
        Returns: undefined;
      };
      get_invite_preview: {
        Args: { p_token: string };
        Returns: {
          center_name: string;
          email: string | null;
          role: string;
          expires_at: string;
          valid: boolean;
        }[];
      };
      get_public_booking_catalog: {
        Args: { p_slug: string };
        Returns: Json;
      };
      get_public_busy_ranges: {
        Args: {
          p_slug: string;
          p_employee_id: string;
          p_from: string;
          p_to: string;
        };
        Returns: Json;
      };
      create_public_booking: {
        Args: {
          p_slug: string;
          p_employee_id: string;
          p_service_id: string;
          p_starts_at: string;
          p_customer_name: string;
          p_customer_email: string;
          p_customer_phone: string;
          p_customer_document: string;
          p_notes?: string | null;
        };
        Returns: string;
      };
      set_product_stock: {
        Args: {
          p_product_id: string;
          p_stock: number;
          p_note?: string | null;
        };
        Returns: number;
      };
      create_sale_with_inventory: {
        Args: {
          p_customer_id: string;
          p_payment_method: string;
          p_items: Json;
        };
        Returns: string;
      };
      create_appointment: {
        Args: {
          p_employee_id: string;
          p_customer_id: string;
          p_service_id: string;
          p_starts_at: string;
          p_notes?: string | null;
        };
        Returns: string;
      };
      update_appointment: {
        Args: {
          p_appointment_id: string;
          p_employee_id: string;
          p_customer_id: string;
          p_service_id: string;
          p_starts_at: string;
          p_notes?: string | null;
        };
        Returns: string;
      };
    };
    Enums: Record<string, never>;
    CompositeTypes: Record<string, never>;
  };
};

export type Center = Database["public"]["Tables"]["centers"]["Row"];
export type Plan = Database["public"]["Tables"]["plans"]["Row"];
export type PlanId = "free" | "starter" | "growth";
export type CenterMember = Database["public"]["Tables"]["center_members"]["Row"];
export type CenterInvite = Database["public"]["Tables"]["center_invites"]["Row"];
export type MemberRole = CenterMember["role"];
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
export type PaymentMethod = Sale["payment_method"];
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
  services: Pick<Service, "id" | "name" | "price" | "duration_minutes" | "status">;
};
