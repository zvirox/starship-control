import React from "react";

export interface DataTableColumn<T> {
  key: string;
  header: string;
  align?: "left" | "right";
  className?: string;
  render: (row: T, index: number) => React.ReactNode;
}

export interface DataTableProps<T> {
  columns: DataTableColumn<T>[];
  rows: T[];
  rowKey: (row: T, index: number) => string | number;
  emptyMessage?: string;
  onRowClick?: (row: T) => void;
  className?: string;
}

export function DataTable<T>({
  columns,
  rows,
  rowKey,
  emptyMessage = "No records found",
  onRowClick,
  className = "",
}: DataTableProps<T>) {
  return (
    <div className={`w-full overflow-x-auto ${className}`}>
      <table className="w-full text-left border-collapse text-[13px]">
        <thead>
          <tr className="border-b border-[var(--border)] bg-[var(--bg-surface-2)]">
            {columns.map((col) => (
              <th
                key={col.key}
                className={`py-2.5 px-3 text-[12px] font-medium uppercase tracking-wider text-[var(--text-muted)] ${
                  col.align === "right" ? "text-right" : "text-left"
                } ${col.className ?? ""}`}
              >
                {col.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="divide-y divide-[var(--border)]">
          {rows.length === 0 ? (
            <tr>
              <td
                colSpan={columns.length}
                className="py-10 px-3 text-center text-[13px] text-[var(--text-muted)]"
              >
                {emptyMessage}
              </td>
            </tr>
          ) : (
            rows.map((row, index) => (
              <tr
                key={rowKey(row, index)}
                onClick={onRowClick ? () => onRowClick(row) : undefined}
                className={`transition-colors duration-150 ease-out hover:bg-[var(--bg-surface-2)] ${
                  onRowClick ? "cursor-pointer" : ""
                }`}
              >
                {columns.map((col) => (
                  <td
                    key={col.key}
                    className={`py-2.5 px-3 text-[var(--text)] ${
                      col.align === "right" ? "text-right" : "text-left"
                    } ${col.className ?? ""}`}
                  >
                    {col.render(row, index)}
                  </td>
                ))}
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}
