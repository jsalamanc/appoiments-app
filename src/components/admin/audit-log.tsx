"use client";

import { invoke } from "@tauri-apps/api/core";
import { ChevronDown, ChevronLeft, ChevronRight, FileDown } from "lucide-react";
import { Fragment, useCallback, useEffect, useMemo, useState } from "react";

import { EmptyState } from "@/components/admin/empty-state";
import { SkeletonRegion, TableSkeleton } from "@/components/admin/skeletons";
import { FadeIn } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  AUDIT_ENTITY_TYPES,
  actionLabel,
  actionTone,
  fieldLabel,
  formatRelatedRow,
  isMoneyField,
  type AuditEntry,
  type AuditFilters,
  type ListAuditResult,
} from "@/lib/audit";
import { downloadCsv } from "@/lib/export-csv";
import { formatCurrency, formatDateTime } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useMinLoadingElapsed } from "@/lib/use-min-loading";

type UserOption = { id: string; name: string; username: string };

const PAGE_SIZE = 25;
const EXPORT_PAGE_SIZE = 500;

const selectClass =
  "h-8 w-full rounded-lg border border-input bg-transparent px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30";

function startOfDayIso(date: string) {
  return new Date(`${date}T00:00:00`).toISOString();
}

function endOfDayIso(date: string) {
  return new Date(`${date}T23:59:59.999`).toISOString();
}

function formatValue(field: string, value: unknown): string {
  if (value === null || value === undefined || value === "") return "—";
  if (typeof value === "boolean") return value ? "Sí" : "No";
  if (typeof value === "number" && isMoneyField(field)) {
    return formatCurrency(value);
  }
  if (Array.isArray(value) && value.length === 0) return "—";
  if (Array.isArray(value)) {
    const lines = value.map((row) =>
      row && typeof row === "object"
        ? formatRelatedRow(field, row as Record<string, unknown>, formatCurrency)
        : null
    );
    if (lines.every((line): line is string => line !== null)) {
      return lines.join("\n");
    }
  }
  if (typeof value === "object") return JSON.stringify(value, null, 1);
  return String(value);
}

function actorLabel(entry: AuditEntry) {
  if (!entry.actor_name && !entry.actor_username) return "Sistema / sin sesión";
  return entry.actor_name ?? entry.actor_username ?? "";
}

function ChangesDetail({ entry }: { entry: AuditEntry }) {
  const changes = entry.changes ? Object.entries(entry.changes) : [];

  if (changes.length > 0) {
    return (
      <div className="space-y-2">
        <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          Qué cambió
        </p>
        <ul className="space-y-2">
          {changes.map(([field, change]) => (
            <li
              key={field}
              className="grid gap-1 rounded-xl border border-border/60 bg-card px-3 py-2 text-sm sm:grid-cols-[180px_1fr_1fr]"
            >
              <span className="font-medium">{fieldLabel(field)}</span>
              <span className="whitespace-pre-wrap break-words text-muted-foreground">
                <span className="mr-1 text-xs uppercase">Antes:</span>
                {formatValue(field, change.from)}
              </span>
              <span className="whitespace-pre-wrap break-words">
                <span className="mr-1 text-xs uppercase text-muted-foreground">
                  Después:
                </span>
                {formatValue(field, change.to)}
              </span>
            </li>
          ))}
        </ul>
      </div>
    );
  }

  const snapshot = entry.after ?? entry.before;
  if (snapshot && !Array.isArray(snapshot)) {
    const title = entry.after ? "Datos registrados" : "Datos eliminados";
    return (
      <div className="space-y-2">
        <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          {title}
        </p>
        <dl className="grid gap-x-4 gap-y-1 text-sm sm:grid-cols-[180px_1fr]">
          {Object.entries(snapshot).map(([field, value]) => (
            <Fragment key={field}>
              <dt className="text-muted-foreground">{fieldLabel(field)}</dt>
              <dd className="whitespace-pre-wrap break-words">
                {formatValue(field, value)}
              </dd>
            </Fragment>
          ))}
        </dl>
      </div>
    );
  }

  return (
    <p className="text-sm text-muted-foreground">
      Esta acción no tiene datos adicionales.
    </p>
  );
}

export function AuditLog() {
  const [users, setUsers] = useState<UserOption[]>([]);
  const [search, setSearch] = useState("");
  const [entityType, setEntityType] = useState("");
  const [actorId, setActorId] = useState("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [page, setPage] = useState(1);
  const [rawResult, setResult] = useState<ListAuditResult | null>(null);
  const minElapsed = useMinLoadingElapsed();
  const result = minElapsed ? rawResult : null;
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);

  const filters = useMemo<AuditFilters>(
    () => ({
      search: search.trim() || undefined,
      entity_type: entityType || undefined,
      actor_user_id: actorId || undefined,
      from: from ? startOfDayIso(from) : undefined,
      to: to ? endOfDayIso(to) : undefined,
    }),
    [search, entityType, actorId, from, to]
  );

  useEffect(() => {
    invoke<UserOption[]>("list_users")
      .then(setUsers)
      .catch(() => setUsers([]));
  }, []);

  const onFilter =
    (setter: (value: string) => void) =>
    (e: { target: { value: string } }) => {
      setter(e.target.value);
      setPage(1);
    };

  const load = useCallback(async () => {
    try {
      const data = await invoke<ListAuditResult>("list_audit_log", {
        filters: { ...filters, page, page_size: PAGE_SIZE },
      });
      setResult(data);
      setError(null);
    } catch (e) {
      setError(typeof e === "string" ? e : "No se pudo cargar la auditoría");
    }
  }, [filters, page]);

  useEffect(() => {
    const timer = setTimeout(load, search ? 250 : 0);
    return () => clearTimeout(timer);
  }, [load, search]);

  const exportCsv = async () => {
    const data = await invoke<ListAuditResult>("list_audit_log", {
      filters: { ...filters, page: 1, page_size: EXPORT_PAGE_SIZE },
    });
    downloadCsv(
      "auditoria",
      [
        "Fecha",
        "Usuario",
        "Rol",
        "Acción",
        "Entidad",
        "Id de entidad",
        "Resumen",
        "Cambios",
      ],
      data.items.map((e) => [
        e.occurred_at,
        e.actor_username ?? "",
        e.actor_role ?? "",
        actionLabel(e.action),
        e.entity_type,
        e.entity_id ?? "",
        e.summary,
        e.changes ? JSON.stringify(e.changes) : "",
      ])
    );
  };

  return (
    <div className="mx-auto max-w-6xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Seguridad</p>
          <h1 className="font-heading text-4xl font-semibold">Auditoría</h1>
          <p className="mt-1 text-muted-foreground">
            Quién hizo qué y qué cambió exactamente. Los registros no se pueden
            editar ni borrar.
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={!result || result.total === 0}
          onClick={exportCsv}
        >
          <FileDown data-icon="inline-start" />
          Exportar Excel
        </Button>
      </FadeIn>

      <FadeIn
        delay={0.05}
        className="grid gap-3 rounded-3xl border border-border/70 bg-card p-4 shadow-sm sm:grid-cols-2 lg:grid-cols-5"
      >
        <div className="space-y-1.5 lg:col-span-2">
          <Label htmlFor="audit-search">Buscar</Label>
          <Input
            id="audit-search"
            placeholder="Resumen, usuario o id…"
            value={search}
            onChange={onFilter(setSearch)}
          />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="audit-entity">Tipo</Label>
          <select
            id="audit-entity"
            className={selectClass}
            value={entityType}
            onChange={onFilter(setEntityType)}
          >
            <option value="">Todos</option>
            {AUDIT_ENTITY_TYPES.map((t) => (
              <option key={t.value} value={t.value}>
                {t.label}
              </option>
            ))}
          </select>
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="audit-actor">Usuario</Label>
          <select
            id="audit-actor"
            className={selectClass}
            value={actorId}
            onChange={onFilter(setActorId)}
          >
            <option value="">Todos</option>
            {users.map((u) => (
              <option key={u.id} value={u.id}>
                {u.name} ({u.username})
              </option>
            ))}
          </select>
        </div>
        <div className="grid grid-cols-2 gap-2">
          <div className="space-y-1.5">
            <Label htmlFor="audit-from">Desde</Label>
            <Input
              id="audit-from"
              type="date"
              value={from}
              onChange={onFilter(setFrom)}
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="audit-to">Hasta</Label>
            <Input
              id="audit-to"
              type="date"
              value={to}
              onChange={onFilter(setTo)}
            />
          </div>
        </div>
      </FadeIn>

      {error && (
        <p className="rounded-2xl border border-destructive/40 bg-destructive/5 px-4 py-3 text-sm text-destructive">
          {error}
        </p>
      )}

      {!result && !error && (
        <SkeletonRegion label="Cargando auditoría…">
          <TableSkeleton rows={8} cols={4} />
        </SkeletonRegion>
      )}

      {result && result.total === 0 && !error ? (
        <EmptyState
          title="Sin registros"
          description="No hay acciones registradas con estos filtros."
        />
      ) : (
        result && (
          <FadeIn
            delay={0.08}
            className="overflow-hidden rounded-3xl border border-border/70 bg-card shadow-md"
          >
            <div className="overflow-x-auto">
              <table className="w-full min-w-[720px] text-left text-sm">
                <thead className="border-b border-border/70 bg-black/[0.02] text-muted-foreground">
                  <tr>
                    <th className="px-4 py-3 font-medium">Fecha</th>
                    <th className="px-4 py-3 font-medium">Usuario</th>
                    <th className="px-4 py-3 font-medium">Acción</th>
                    <th className="px-4 py-3 font-medium">Detalle</th>
                    <th className="w-10 px-2 py-3" />
                  </tr>
                </thead>
                <tbody>
                  {result.items.map((entry) => {
                    const open = expanded === entry.id;
                    return (
                      <Fragment key={entry.id}>
                        <tr
                          className="cursor-pointer border-b border-border/50 align-top last:border-0 hover:bg-black/[0.02]"
                          onClick={() => setExpanded(open ? null : entry.id)}
                        >
                          <td className="whitespace-nowrap px-4 py-3 text-muted-foreground">
                            {formatDateTime(entry.occurred_at)}
                          </td>
                          <td className="px-4 py-3">
                            <p className="font-medium">{actorLabel(entry)}</p>
                            {entry.actor_role && (
                              <p className="text-xs text-muted-foreground">
                                {entry.actor_role === "owner"
                                  ? "Dueño"
                                  : "Staff"}
                              </p>
                            )}
                          </td>
                          <td className="px-4 py-3">
                            <Badge variant={actionTone(entry.action)}>
                              {actionLabel(entry.action)}
                            </Badge>
                          </td>
                          <td className="px-4 py-3">{entry.summary}</td>
                          <td className="px-2 py-3 text-muted-foreground">
                            <ChevronDown
                              className={cn(
                                "size-4 transition-transform",
                                open && "rotate-180"
                              )}
                            />
                          </td>
                        </tr>
                        {open && (
                          <tr className="border-b border-border/50 bg-black/[0.02]">
                            <td colSpan={5} className="space-y-3 px-4 py-4">
                              <p className="text-xs text-muted-foreground">
                                Id del registro: {entry.entity_id ?? "—"}
                              </p>
                              <ChangesDetail entry={entry} />
                            </td>
                          </tr>
                        )}
                      </Fragment>
                    );
                  })}
                </tbody>
              </table>
            </div>
            <div className="flex items-center justify-between gap-3 border-t border-border/70 px-4 py-3">
              <p className="text-sm text-muted-foreground">
                {result.total} registro{result.total === 1 ? "" : "s"} · página{" "}
                {result.page} de {result.total_pages}
              </p>
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  disabled={result.page <= 1}
                  onClick={() => setPage((p) => Math.max(1, p - 1))}
                >
                  <ChevronLeft data-icon="inline-start" />
                  Anterior
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  disabled={result.page >= result.total_pages}
                  onClick={() => setPage((p) => p + 1)}
                >
                  Siguiente
                  <ChevronRight data-icon="inline-end" />
                </Button>
              </div>
            </div>
          </FadeIn>
        )
      )}
    </div>
  );
}
