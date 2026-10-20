"use client";

import { invoke } from "@tauri-apps/api/core";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

import { CustomersManager } from "@/components/admin/customers-manager";
import type { Customer } from "@/types/database";

type ListResult = {
  items: Customer[];
  total: number;
  page: number;
  total_pages: number;
  page_size: number;
  q: string;
};

export default function CustomersPage() {
  const searchParams = useSearchParams();
  const page = Number.parseInt(searchParams.get("page") ?? "1", 10) || 1;
  const q = searchParams.get("q") ?? "";

  const [result, setResult] = useState<ListResult | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<ListResult>("list_customers", { page, pageSize: 9, q }).then(
      (r) => {
        if (!cancelled) setResult(r);
      }
    );
    return () => {
      cancelled = true;
    };
  }, [page, q]);

  if (!result) return null;

  return (
    <CustomersManager
      customers={result.items}
      page={result.page}
      totalPages={result.total_pages}
      total={result.total}
      pageSize={result.page_size}
      q={result.q}
    />
  );
}
