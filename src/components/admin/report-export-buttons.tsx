"use client";

import { FileDown, Printer } from "lucide-react";

import { Button } from "@/components/ui/button";

type ReportExportButtonsProps = {
  onExportExcel: () => void;
  excelDisabled?: boolean;
};

export function ReportExportButtons({
  onExportExcel,
  excelDisabled,
}: ReportExportButtonsProps) {
  return (
    <div className="print-hidden flex flex-wrap gap-2">
      <Button
        type="button"
        variant="outline"
        size="sm"
        disabled={excelDisabled}
        onClick={onExportExcel}
      >
        <FileDown data-icon="inline-start" />
        Exportar Excel
      </Button>
      <Button
        type="button"
        variant="outline"
        size="sm"
        onClick={() => window.print()}
      >
        <Printer data-icon="inline-start" />
        Exportar PDF
      </Button>
    </div>
  );
}
