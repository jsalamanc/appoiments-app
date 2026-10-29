"use client";

import { ChevronLeft, ChevronRight, Plus, Search, Trash2 } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

import { FadeIn } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { completeAppointmentWithSale } from "@/lib/appointments";
import { fetchSellableCatalog } from "@/lib/catalog-client";
import {
  createCustomer,
  findCustomerByDocument,
  findCustomerById,
  normalizeDocument,
  normalizeEmail,
} from "@/lib/customers";
import { formatCurrency, formatSaleItemType } from "@/lib/format";
import {
  cartLineTotal,
  cartTotal,
  createSale,
  type PaymentLineInput,
} from "@/lib/sales";
import type {
  Customer,
  Package,
  PaymentMethod,
  Product,
  SaleItemType,
  Service,
} from "@/types/database";

const PAYMENT_METHOD_OPTIONS: { value: PaymentMethod; label: string }[] = [
  { value: "cash", label: "Efectivo" },
  { value: "transfer", label: "Transferencia" },
  { value: "card", label: "Tarjeta" },
];

function round2(value: number) {
  return Math.round(value * 100) / 100;
}

type CatalogOption = {
  item_type: SaleItemType;
  item_id: string;
  name: string;
  unit_price: number;
  /** Solo productos; servicios/paquetes no usan inventario. */
  stock?: number;
};

type CartLine = CatalogOption & { quantity: number; key: string };
type CatalogTypeFilter = "all" | SaleItemType;

const CATALOG_PAGE_SIZE = 3;

const BASE_TYPE_FILTERS: { value: CatalogTypeFilter; label: string }[] = [
  { value: "all", label: "Todos" },
  { value: "product", label: "Productos" },
  { value: "service", label: "Servicios" },
  { value: "package", label: "Paquetes" },
];

type SaleFormPrefill = {
  customerId?: string;
  serviceId?: string;
  appointmentId?: string;
};

export function SaleForm({ prefill }: { prefill?: SaleFormPrefill }) {
  const router = useRouter();

  const [products, setProducts] = useState<Product[]>([]);
  const [services, setServices] = useState<Service[]>([]);
  const [packages, setPackages] = useState<Package[]>([]);
  const [draftCount, setDraftCount] = useState(0);
  const [totalCount, setTotalCount] = useState(0);
  const [catalogLoading, setCatalogLoading] = useState(true);
  const [catalogError, setCatalogError] = useState<string | null>(null);

  const [email, setEmail] = useState("");
  const [name, setName] = useState("");
  const [phone, setPhone] = useState("");
  const [customerDocument, setCustomerDocument] = useState("");
  const [customer, setCustomer] = useState<Customer | null>(null);
  const [customerExists, setCustomerExists] = useState(false);
  const [lookupDone, setLookupDone] = useState(false);
  const [lookingUp, setLookingUp] = useState(false);
  const [creatingCustomer, setCreatingCustomer] = useState(false);

  const [catalogSearch, setCatalogSearch] = useState("");
  const [catalogFilter, setCatalogFilter] = useState<CatalogTypeFilter>("all");
  const [catalogPage, setCatalogPage] = useState(1);
  const [addQuantities, setAddQuantities] = useState<Record<string, number>>({});
  const [cart, setCart] = useState<CartLine[]>([]);
  const [discountAmount, setDiscountAmount] = useState(0);
  const [singleMethod, setSingleMethod] = useState<PaymentMethod>("cash");
  const [splitPayment, setSplitPayment] = useState(false);
  const [paymentLines, setPaymentLines] = useState<PaymentLineInput[]>([
    { method: "cash", amount: 0 },
    { method: "card", amount: 0 },
  ]);
  const [submitting, setSubmitting] = useState(false);
  const [appointmentId, setAppointmentId] = useState<string | null>(
    prefill?.appointmentId ?? null
  );
  const [prefillDone, setPrefillDone] = useState(false);
  const [modulesEnabled, setModulesEnabled] = useState({
    services: true,
    packages: true,
  });

  useEffect(() => {
    let active = true;

    async function loadCatalog() {
      setCatalogLoading(true);
      try {
        const catalog = await fetchSellableCatalog();
        if (!active) return;
        setProducts(catalog.products);
        setServices(catalog.services);
        setPackages(catalog.packages);
        setDraftCount(catalog.draftCount);
        setTotalCount(catalog.totalCount);
        setCatalogError(catalog.error);
        setModulesEnabled(catalog.modules);
        setCatalogFilter((current) => {
          if (current === "service" && !catalog.modules.services) return "all";
          if (current === "package" && !catalog.modules.packages) return "all";
          return current;
        });
      } catch (err) {
        if (!active) return;
        setCatalogError(
          err instanceof Error ? err.message : "No se pudo cargar el catálogo"
        );
      } finally {
        if (active) setCatalogLoading(false);
      }
    }

    void loadCatalog();
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (catalogLoading || prefillDone) return;
    if (!prefill?.customerId && !prefill?.serviceId) {
      setPrefillDone(true);
      return;
    }

    let active = true;

    async function applyPrefill() {
      try {
        if (prefill?.customerId) {
          const found = await findCustomerById(prefill.customerId);
          if (!active) return;
          if (found) {
            setCustomer(found);
            setCustomerExists(true);
            setLookupDone(true);
            setEmail(found.email ?? "");
            setName(found.name);
            setPhone(found.phone);
            setCustomerDocument(found.document ?? "");
          }
        }

        if (prefill?.serviceId) {
          const service = services.find((s) => s.id === prefill.serviceId);
          if (service) {
            const key = `service:${service.id}`;
            setCart([
              {
                item_type: "service",
                item_id: service.id,
                name: service.name,
                unit_price: Number(service.price),
                quantity: 1,
                key,
              },
            ]);
          }
        }

        if (prefill?.appointmentId) {
          setAppointmentId(prefill.appointmentId);
          toast.message("Venta desde cita", {
            description: "Cliente y servicio precargados. Confirma el pago.",
          });
        }
      } catch (err) {
        toast.error(
          err instanceof Error ? err.message : "No se pudo precargar la cita"
        );
      } finally {
        if (active) setPrefillDone(true);
      }
    }

    void applyPrefill();
    return () => {
      active = false;
    };
  }, [catalogLoading, prefill, prefillDone, services]);

  const typeFilters = useMemo(
    () =>
      BASE_TYPE_FILTERS.filter((filter) => {
        if (filter.value === "service") return modulesEnabled.services;
        if (filter.value === "package") return modulesEnabled.packages;
        return true;
      }),
    [modulesEnabled]
  );

  const catalogOptions = useMemo<CatalogOption[]>(() => {
    const productOptions = products.map((p) => ({
      item_type: "product" as const,
      item_id: p.id,
      name: p.name,
      unit_price: Number(p.price),
      stock: p.stock,
    }));
    const serviceOptions = modulesEnabled.services
      ? services.map((s) => ({
          item_type: "service" as const,
          item_id: s.id,
          name: s.name,
          unit_price: Number(s.price),
        }))
      : [];
    const packageOptions = modulesEnabled.packages
      ? packages.map((p) => ({
          item_type: "package" as const,
          item_id: p.id,
          name: p.name,
          unit_price: Number(p.price),
        }))
      : [];
    return [...productOptions, ...serviceOptions, ...packageOptions];
  }, [products, services, packages, modulesEnabled]);

  const cartQuantityByProduct = useMemo(() => {
    const map = new Map<string, number>();
    for (const line of cart) {
      if (line.item_type !== "product") continue;
      map.set(line.item_id, (map.get(line.item_id) ?? 0) + line.quantity);
    }
    return map;
  }, [cart]);

  const filteredOptions = useMemo(() => {
    const query = catalogSearch.trim().toLowerCase();
    return catalogOptions.filter((option) => {
      const matchesType =
        catalogFilter === "all" || option.item_type === catalogFilter;
      const matchesSearch =
        !query || option.name.toLowerCase().includes(query);
      return matchesType && matchesSearch;
    });
  }, [catalogOptions, catalogFilter, catalogSearch]);

  const catalogTotalPages = Math.max(
    1,
    Math.ceil(filteredOptions.length / CATALOG_PAGE_SIZE)
  );
  const safeCatalogPage = Math.min(catalogPage, catalogTotalPages);
  const paginatedOptions = filteredOptions.slice(
    (safeCatalogPage - 1) * CATALOG_PAGE_SIZE,
    safeCatalogPage * CATALOG_PAGE_SIZE
  );

  const subtotal = cartTotal(cart);
  const total = round2(Math.max(0, subtotal - discountAmount));
  const canSell = catalogOptions.length > 0;

  const paymentLinesTotal = round2(
    paymentLines.reduce((sum, line) => sum + (line.amount || 0), 0)
  );
  const paymentsToSend: PaymentLineInput[] = splitPayment
    ? paymentLines.filter((line) => line.amount > 0)
    : [{ method: singleMethod, amount: total }];
  const paymentsReady = splitPayment
    ? paymentLinesTotal === total && paymentsToSend.length > 0
    : true;

  useEffect(() => {
    setCatalogPage(1);
  }, [catalogSearch, catalogFilter]);

  const lookupCustomer = async () => {
    const normalized = normalizeDocument(customerDocument);
    if (normalized.length < 5) {
      toast.error("Ingresa un documento válido (mínimo 5 caracteres)");
      return;
    }

    setLookingUp(true);
    try {
      const found = await findCustomerByDocument(normalized);
      setLookupDone(true);
      setCustomerDocument(normalized);

      if (found) {
        setCustomer(found);
        setCustomerExists(true);
        setName(found.name);
        setPhone(found.phone);
        setEmail(found.email ?? "");
        toast.success("Cliente encontrado");
      } else {
        setCustomer(null);
        setCustomerExists(false);
        setName("");
        setPhone("");
        setEmail("");
        toast.message("Cliente nuevo", {
          description: "Completa nombre, teléfono y correo para crearlo.",
        });
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Error al buscar");
    } finally {
      setLookingUp(false);
    }
  };

  const ensureCustomerReady = async () => {
    if (customer) return customer;

    if (!lookupDone) {
      throw new Error("Primero busca el documento del cliente");
    }

    if (!name.trim() || !phone.trim()) {
      throw new Error("Completa nombre y teléfono del cliente");
    }

    // Issue #2, Fase 5: correo/documento son opcionales — una venta rápida
    // (cliente de paso) no debe obligar a inventarlos.
    const normalizedEmail = email.trim() ? normalizeEmail(email) : null;
    if (normalizedEmail && !normalizedEmail.includes("@")) {
      throw new Error("Ingresa un correo válido");
    }

    setCreatingCustomer(true);
    try {
      const created = await createCustomer({
        email: normalizedEmail,
        name,
        phone,
        document: customerDocument,
      });
      setCustomer(created);
      setCustomerExists(true);
      return created;
    } finally {
      setCreatingCustomer(false);
    }
  };

  const getAddQuantity = (key: string) => {
    const value = addQuantities[key];
    return Number.isFinite(value) && value >= 1 ? value : 1;
  };

  const addToCart = (option: CatalogOption) => {
    const key = `${option.item_type}:${option.item_id}`;
    const quantity = getAddQuantity(key);

    if (option.item_type === "product") {
      const available = option.stock ?? 0;
      const inCart = cartQuantityByProduct.get(option.item_id) ?? 0;
      if (available <= 0) {
        toast.error(`“${option.name}” no tiene stock`);
        return;
      }
      if (inCart + quantity > available) {
        toast.error(
          `Stock insuficiente de “${option.name}” (disponible: ${available})`
        );
        return;
      }
    }

    setCart((prev) => {
      const existing = prev.find((line) => line.key === key);
      if (existing) {
        return prev.map((line) =>
          line.key === key
            ? { ...line, quantity: line.quantity + quantity }
            : line
        );
      }
      return [...prev, { ...option, quantity, key }];
    });

    setAddQuantities((prev) => ({ ...prev, [key]: 1 }));
    toast.success(`${option.name} agregado`);
  };

  const registerSale = async () => {
    if (cart.length === 0) {
      toast.error("Agrega al menos un ítem");
      return;
    }
    if (splitPayment && !paymentsReady) {
      toast.error(
        `La suma de los medios de pago (${formatCurrency(paymentLinesTotal)}) debe ser igual al total (${formatCurrency(total)})`
      );
      return;
    }

    setSubmitting(true);
    try {
      const readyCustomer = await ensureCustomerReady();
      const saleId = await createSale({
        customerId: readyCustomer.id,
        items: cart.map((line) => ({
          item_type: line.item_type,
          item_id: line.item_id,
          name: line.name,
          unit_price: line.unit_price,
          quantity: line.quantity,
        })),
        payments: paymentsToSend,
        discountAmount: discountAmount > 0 ? discountAmount : undefined,
      });

      if (appointmentId) {
        try {
          await completeAppointmentWithSale(appointmentId, saleId);
        } catch (linkErr) {
          toast.error(
            linkErr instanceof Error
              ? `Venta creada, pero no se pudo completar la cita: ${linkErr.message}`
              : "Venta creada, pero no se pudo completar la cita"
          );
        }
      }

      toast.success("Venta registrada");
      router.push(`/admin/sales/history?highlight=${saleId}`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo registrar");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn>
        <p className="text-sm text-muted-foreground">Ventas</p>
        <h1 className="font-heading text-4xl font-semibold">Nueva venta</h1>
        <p className="mt-1 text-muted-foreground">
          Busca o crea el cliente, arma el carrito y registra el pago.
        </p>
      </FadeIn>

      {catalogLoading && (
        <FadeIn className="rounded-3xl border border-dashed border-border bg-card shadow-sm p-6 text-sm text-muted-foreground">
          Cargando catálogo publicado...
        </FadeIn>
      )}

      {!catalogLoading && catalogError && (
        <FadeIn className="rounded-3xl border border-destructive/30 bg-destructive/5 p-6 text-sm text-destructive">
          Error al cargar catálogo: {catalogError}
        </FadeIn>
      )}

      {!catalogLoading && !canSell && (
        <FadeIn className="rounded-3xl border border-dashed border-border bg-card shadow-sm p-6 text-sm text-muted-foreground">
          {totalCount === 0 ? (
            <>
              Aún no tienes productos, servicios o paquetes.{" "}
              <Link href="/admin/services" className="underline">
                Crea y publica uno
              </Link>{" "}
              para empezar a vender.
            </>
          ) : (
            <>
              Tienes {draftCount} ítem{draftCount === 1 ? "" : "s"} en borrador.
              Publícalos desde Productos, Servicios o Paquetes para poder
              venderlos aquí.
            </>
          )}
        </FadeIn>
      )}

      <FadeIn
        delay={0.05}
        className="space-y-4 rounded-3xl border border-border/70 bg-card shadow-sm p-5"
      >
        <div className="flex items-center justify-between gap-3">
          <h2 className="font-heading text-2xl font-semibold">1. Cliente</h2>
          {customerExists && customer && (
            <Badge variant="secondary">Cliente existente</Badge>
          )}
          {lookupDone && !customerExists && (
            <Badge variant="outline">Cliente nuevo</Badge>
          )}
        </div>

        <div className="grid gap-3 sm:grid-cols-[1fr_auto]">
          <div className="space-y-2">
            <Label htmlFor="sale-document">Documento del comprador</Label>
            <Input
              id="sale-document"
              placeholder="CC / CE / NIT"
              value={customerDocument}
              onChange={(e) => {
                setCustomerDocument(e.target.value);
                setLookupDone(false);
                setCustomer(null);
                setCustomerExists(false);
                setName("");
                setPhone("");
                setEmail("");
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  void lookupCustomer();
                }
              }}
            />
          </div>
          <div className="flex items-end">
            <Button
              type="button"
              variant="outline"
              disabled={lookingUp}
              onClick={() => void lookupCustomer()}
            >
              <Search data-icon="inline-start" />
              {lookingUp ? "Buscando..." : "Buscar"}
            </Button>
          </div>
        </div>

        <div className="grid gap-3 sm:grid-cols-3">
          <div className="space-y-2">
            <Label htmlFor="sale-name">Nombre</Label>
            <Input
              id="sale-name"
              value={name}
              disabled={!lookupDone || customerExists}
              onChange={(e) => setName(e.target.value)}
              placeholder="Nombre completo"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="sale-phone">Teléfono</Label>
            <Input
              id="sale-phone"
              value={phone}
              disabled={!lookupDone || customerExists}
              onChange={(e) => setPhone(e.target.value)}
              placeholder="3001234567"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="sale-email">Correo</Label>
            <Input
              id="sale-email"
              type="email"
              value={email}
              disabled={!lookupDone || customerExists}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="cliente@correo.com"
            />
          </div>
        </div>
      </FadeIn>

      <FadeIn
        delay={0.1}
        className="space-y-4 rounded-3xl border border-border/70 bg-card shadow-sm p-5"
      >
        <h2 className="font-heading text-2xl font-semibold">2. Carrito</h2>

        <div className="space-y-4">
          <div className="flex flex-col gap-3 sm:flex-row">
            <div className="relative flex-1">
              <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={catalogSearch}
                onChange={(e) => setCatalogSearch(e.target.value)}
                placeholder="Buscar por nombre..."
                className="pl-8"
                disabled={!canSell && !catalogLoading}
              />
            </div>
          </div>

          <div className="flex flex-wrap gap-2">
            {typeFilters.map((filter) => (
              <button
                key={filter.value}
                type="button"
                disabled={!canSell && !catalogLoading}
                onClick={() => setCatalogFilter(filter.value)}
                className={`rounded-full px-3 py-1.5 text-sm transition disabled:cursor-not-allowed disabled:opacity-50 ${
                  catalogFilter === filter.value
                    ? "bg-primary text-primary-foreground"
                    : "bg-muted/70 text-muted-foreground hover:bg-muted hover:text-foreground"
                }`}
              >
                {filter.label}
              </button>
            ))}
          </div>

          {catalogLoading ? (
            <p className="rounded-2xl border border-dashed border-border px-4 py-6 text-sm text-muted-foreground">
              Cargando catálogo...
            </p>
          ) : !canSell ? (
            <p className="rounded-2xl border border-dashed border-border px-4 py-6 text-sm text-muted-foreground">
              {draftCount > 0
                ? "Solo hay borradores. Publícalos para venderlos."
                : "Publica al menos un producto, servicio o paquete para vender."}
            </p>
          ) : filteredOptions.length === 0 ? (
            <p className="rounded-2xl border border-dashed border-border px-4 py-6 text-sm text-muted-foreground">
              No hay resultados con ese buscador o filtro.
            </p>
          ) : (
            <>
              <div className="space-y-2">
                {paginatedOptions.map((option) => {
                  const key = `${option.item_type}:${option.item_id}`;
                  const outOfStock =
                    option.item_type === "product" && (option.stock ?? 0) <= 0;
                  return (
                    <div
                      key={key}
                      className="flex flex-col gap-3 rounded-2xl border border-border/70 bg-background/50 px-4 py-3 sm:flex-row sm:items-center sm:justify-between"
                    >
                      <div>
                        <div className="flex flex-wrap items-center gap-2">
                          <p className="font-medium">{option.name}</p>
                          <Badge variant="outline">
                            {formatSaleItemType(option.item_type)}
                          </Badge>
                          {outOfStock && (
                            <Badge variant="destructive">Sin stock</Badge>
                          )}
                        </div>
                        <p className="text-sm text-muted-foreground">
                          {formatCurrency(option.unit_price)}
                          {option.item_type === "product"
                            ? ` · Stock: ${option.stock ?? 0}`
                            : ""}
                        </p>
                      </div>
                      <div className="flex items-center gap-2">
                        <Input
                          type="number"
                          min={1}
                          max={
                            option.item_type === "product"
                              ? Math.max(option.stock ?? 0, 1)
                              : undefined
                          }
                          className="w-20"
                          value={getAddQuantity(key)}
                          onChange={(e) => {
                            const next = Number(e.target.value);
                            setAddQuantities((prev) => ({
                              ...prev,
                              [key]:
                                Number.isFinite(next) && next >= 1 ? next : 1,
                            }));
                          }}
                        />
                        <Button
                          type="button"
                          size="sm"
                          disabled={
                            option.item_type === "product" &&
                            (option.stock ?? 0) <= 0
                          }
                          onClick={() => addToCart(option)}
                        >
                          <Plus data-icon="inline-start" />
                          Agregar
                        </Button>
                      </div>
                    </div>
                  );
                })}
              </div>

              <div className="flex flex-col gap-3 rounded-2xl border border-border/60 bg-muted/20 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
                <p className="text-sm text-muted-foreground">
                  Mostrando{" "}
                  {(safeCatalogPage - 1) * CATALOG_PAGE_SIZE + 1}
                  –
                  {Math.min(
                    safeCatalogPage * CATALOG_PAGE_SIZE,
                    filteredOptions.length
                  )}{" "}
                  de {filteredOptions.length}
                </p>
                <div className="flex items-center gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={safeCatalogPage <= 1}
                    onClick={() =>
                      setCatalogPage((page) => Math.max(1, page - 1))
                    }
                  >
                    <ChevronLeft data-icon="inline-start" />
                    Anterior
                  </Button>
                  <span className="min-w-16 text-center text-sm text-muted-foreground">
                    {safeCatalogPage} / {catalogTotalPages}
                  </span>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={safeCatalogPage >= catalogTotalPages}
                    onClick={() =>
                      setCatalogPage((page) =>
                        Math.min(catalogTotalPages, page + 1)
                      )
                    }
                  >
                    Siguiente
                    <ChevronRight data-icon="inline-end" />
                  </Button>
                </div>
              </div>
            </>
          )}
        </div>

        {cart.length === 0 ? (
          <p className="rounded-2xl border border-dashed border-border px-4 py-8 text-center text-sm text-muted-foreground">
            El carrito está vacío.
          </p>
        ) : (
          <div className="space-y-2">
            {cart.map((line) => (
              <div
                key={line.key}
                className="flex flex-col gap-3 rounded-2xl border border-border/60 bg-muted/30 px-4 py-3 sm:flex-row sm:items-center sm:justify-between"
              >
                <div>
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="font-medium">{line.name}</p>
                    <Badge variant="outline">
                      {formatSaleItemType(line.item_type)}
                    </Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">
                    {formatCurrency(line.unit_price)} × {line.quantity}
                  </p>
                </div>
                <div className="flex items-center gap-3">
                  <Input
                    type="number"
                    min={1}
                    max={
                      line.item_type === "product"
                        ? Math.max(line.stock ?? 0, 1)
                        : undefined
                    }
                    className="w-20"
                    value={line.quantity}
                    onChange={(e) => {
                      const next = Number(e.target.value);
                      let quantity =
                        Number.isFinite(next) && next >= 1 ? next : 1;
                      if (
                        line.item_type === "product" &&
                        line.stock != null &&
                        quantity > line.stock
                      ) {
                        quantity = line.stock;
                        toast.error(
                          `Stock máximo de “${line.name}”: ${line.stock}`
                        );
                      }
                      setCart((prev) =>
                        prev.map((item) =>
                          item.key === line.key
                            ? { ...item, quantity }
                            : item
                        )
                      );
                    }}
                  />
                  <p className="min-w-24 text-right font-medium">
                    {formatCurrency(
                      cartLineTotal(line.unit_price, line.quantity)
                    )}
                  </p>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    onClick={() =>
                      setCart((prev) =>
                        prev.filter((item) => item.key !== line.key)
                      )
                    }
                  >
                    <Trash2 />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}

        <div className="space-y-2 rounded-2xl bg-lavender/40 px-4 py-3">
          <div className="flex items-center justify-between gap-3">
            <Label htmlFor="sale-discount" className="text-sm text-muted-foreground">
              Descuento
            </Label>
            <Input
              id="sale-discount"
              type="number"
              min={0}
              max={subtotal}
              step="0.01"
              className="w-32 text-right"
              value={discountAmount || ""}
              onChange={(e) => {
                const next = Number(e.target.value);
                const safe = Number.isFinite(next) && next >= 0 ? next : 0;
                setDiscountAmount(Math.min(safe, subtotal));
              }}
            />
          </div>
          {discountAmount > 0 && (
            <div className="flex items-center justify-between text-sm text-muted-foreground">
              <span>Subtotal</span>
              <span>{formatCurrency(subtotal)}</span>
            </div>
          )}
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Total</span>
            <span className="font-heading text-3xl font-semibold">
              {formatCurrency(total)}
            </span>
          </div>
        </div>
      </FadeIn>

      <FadeIn
        delay={0.15}
        className="space-y-4 rounded-3xl border border-border/70 bg-card shadow-sm p-5"
      >
        <div className="flex items-center justify-between gap-3">
          <h2 className="font-heading text-2xl font-semibold">3. Pago</h2>
          <button
            type="button"
            onClick={() => setSplitPayment((v) => !v)}
            className="text-sm text-muted-foreground underline-offset-4 hover:text-foreground hover:underline"
          >
            {splitPayment ? "Usar un solo medio de pago" : "Dividir en varios medios de pago"}
          </button>
        </div>

        {!splitPayment ? (
          <div className="grid gap-3 sm:grid-cols-3">
            {PAYMENT_METHOD_OPTIONS.map((option) => (
              <button
                key={option.value}
                type="button"
                onClick={() => setSingleMethod(option.value)}
                className={`rounded-2xl border px-4 py-4 text-left transition ${
                  singleMethod === option.value
                    ? "border-primary bg-primary/10"
                    : "border-border bg-background/50 hover:bg-muted/50"
                }`}
              >
                <p className="font-medium">{option.label}</p>
                <p className="text-sm text-muted-foreground">
                  {option.value === "cash"
                    ? "Pago recibido en caja"
                    : option.value === "transfer"
                      ? "Pago por transferencia bancaria"
                      : "Pago con tarjeta/datáfono"}
                </p>
              </button>
            ))}
          </div>
        ) : (
          <div className="space-y-3">
            {paymentLines.map((line, index) => (
              <div key={index} className="flex items-center gap-2">
                <select
                  value={line.method}
                  onChange={(e) => {
                    const method = e.target.value as PaymentMethod;
                    setPaymentLines((prev) =>
                      prev.map((l, i) => (i === index ? { ...l, method } : l))
                    );
                  }}
                  className="flex h-9 rounded-lg border border-input bg-transparent px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                >
                  {PAYMENT_METHOD_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
                <Input
                  type="number"
                  min={0}
                  step="0.01"
                  className="flex-1"
                  value={line.amount || ""}
                  onChange={(e) => {
                    const next = Number(e.target.value);
                    const amount = Number.isFinite(next) && next >= 0 ? next : 0;
                    setPaymentLines((prev) =>
                      prev.map((l, i) => (i === index ? { ...l, amount } : l))
                    );
                  }}
                />
                {paymentLines.length > 1 && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    onClick={() =>
                      setPaymentLines((prev) => prev.filter((_, i) => i !== index))
                    }
                  >
                    <Trash2 />
                  </Button>
                )}
              </div>
            ))}
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() =>
                setPaymentLines((prev) => [...prev, { method: "cash", amount: 0 }])
              }
            >
              <Plus data-icon="inline-start" />
              Agregar medio de pago
            </Button>
            <p
              className={
                paymentsReady
                  ? "text-sm text-muted-foreground"
                  : "text-sm text-destructive"
              }
            >
              {formatCurrency(paymentLinesTotal)} de {formatCurrency(total)}
              {!paymentsReady && " — la suma debe coincidir con el total"}
            </p>
          </div>
        )}

        <Button
          className="w-full sm:w-auto"
          size="lg"
          disabled={
            submitting ||
            creatingCustomer ||
            !canSell ||
            cart.length === 0 ||
            (splitPayment && !paymentsReady)
          }
          onClick={() => void registerSale()}
        >
          {submitting || creatingCustomer
            ? "Registrando..."
            : "Registrar venta"}
        </Button>
      </FadeIn>
    </div>
  );
}
