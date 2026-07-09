import { createSignal, For, createMemo } from "solid-js";
import {
  createSolidTable,
  getCoreRowModel,
  getSortedRowModel,
  flexRender,
  type ColumnDef,
  type SortingState,
  type ColumnSort,
} from "@tanstack/solid-table";

type Badge = {
  set_id: string;
  id: string;
};

export type ChatMessage = {
  message_id: string;
  timestamp_ms: number;
  channel_id: string;
  channel_login: string;
  user_id: string;
  user_login: string;
  user_name: string;
  badges: Badge[];
  color: string;
  raw_message: string;
  normalized_message?: string | null;
  security_flags: string[];
};

type MessagesResponse = {
  messages: ChatMessage[];
};

type Props = {
  apiBase: string;
  limit?: number;
};

function formatTime(ts: number) {
  const d = new Date(ts);
  return d.toLocaleString([], {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function sortIcon(isSorted: boolean | undefined, dir: "asc" | "desc" | false) {
  if (!isSorted) return " ↕";
  return dir === "asc" ? " ↑" : " ↓";
}

export default function MessageTable(props: Props) {
  const [data, setData] = createSignal<ChatMessage[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [offset, setOffset] = createSignal(0);
  const [sorting, setSorting] = createSignal<SortingState>([]);
  const [flaggedOnly, setFlaggedOnly] = createSignal(false);
  const pageSize = props.limit ?? 50;

  const columns: ColumnDef<ChatMessage>[] = [
    {
      accessorKey: "timestamp_ms",
      header: "Time",
      cell: (info) => formatTime(info.getValue<number>()),
    },
    {
      accessorKey: "user_login",
      header: "User",
      cell: (info) => {
        const row = info.row.original;
        const color = row.color || "#91f5b8";
        return (
          <span>
            <strong style={{ color }}>{row.user_name}</strong>{" "}
            <span class="table-user-login">@{row.user_login}</span>
          </span>
        );
      },
    },
    {
      accessorFn: (row) => row.normalized_message ?? row.raw_message,
      id: "message",
      header: "Message",
      cell: (info) => info.getValue<string>(),
    },
    {
      accessorKey: "security_flags",
      header: "Flags",
      cell: (info) => {
        const flags = info.getValue<string[]>();
        if (!flags || flags.length === 0) {
          return null;
        }
        return <span class="table-flags flagged">{flags.join(", ")}</span>;
      },
      sortingFn: (rowA, rowB, columnId) => {
        const a = (rowA.getValue<string[]>(columnId) ?? []).length;
        const b = (rowB.getValue<string[]>(columnId) ?? []).length;
        return a - b;
      },
    },
  ];

  const table = createSolidTable({
    get data() {
      return data();
    },
    columns,
    state: {
      get sorting() {
        return sorting();
      },
    },
    onSortingChange: setSorting,
    getSortedRowModel: getSortedRowModel(),
    getCoreRowModel: getCoreRowModel(),
  });

  const fetchPage = async () => {
    setLoading(true);
    setError(null);
    try {
      const params = new URLSearchParams({
        limit: String(pageSize),
        offset: String(offset()),
        flagged_only: String(flaggedOnly()),
      });
      const res = await fetch(`${props.apiBase}/api/messages?${params}`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const json = (await res.json()) as MessagesResponse;
      setData(json.messages ?? []);
    } catch (err) {
      setError(String(err));
      setData([]);
    } finally {
      setLoading(false);
    }
  };

  const sortedRowCount = createMemo(() => table.getRowModel().rows.length);
  const hasNext = createMemo(() => sortedRowCount() === pageSize);
  const hasPrev = createMemo(() => offset() > 0);

  const nextPage = () => {
    if (!hasNext()) return;
    setOffset((o) => o + pageSize);
    fetchPage();
  };
  const prevPage = () => {
    if (!hasPrev()) return;
    setOffset(Math.max(0, offset() - pageSize));
    fetchPage();
  };

  const toggleFlagged = () => {
    setFlaggedOnly((v) => !v);
    setOffset(0);
    setTimeout(fetchPage, 0);
  };

  return (
    <div class="table-container">
      <div class="table-controls">
        <button class="primary-button" onClick={fetchPage} disabled={loading()}>
          {loading() ? "Loading..." : "Refresh"}
        </button>
        <label class="table-filter">
          <input
            type="checkbox"
            checked={flaggedOnly()}
            onChange={toggleFlagged}
          />
          Flagged only
        </label>
        <span class="table-page-info">
          offset {offset()} • {data().length} rows
        </span>
      </div>

      {error() && <p class="status error">{error()}</p>}

      <table class="message-table">
        <thead>
          <For each={table.getHeaderGroups()}>
            {(headerGroup) => (
              <tr>
                <For each={headerGroup.headers}>
                  {(header) => {
                    const sort = () =>
                      sorting().find((s) => s.id === header.id) as
                        | ColumnSort
                        | undefined;
                    return (
                      <th
                        colSpan={header.colSpan}
                        onClick={header.column.getToggleSortingHandler()}
                        class={
                          header.column.getCanSort() ? "sortable" : undefined
                        }
                      >
                        {flexRender(
                          header.column.columnDef.header,
                          header.getContext(),
                        )}
                        {sortIcon(
                          !!sort(),
                          (sort()?.desc ? "desc" : "asc") as
                            | "asc"
                            | "desc"
                            | false,
                        )}
                      </th>
                    );
                  }}
                </For>
              </tr>
            )}
          </For>
        </thead>
        <tbody>
          <For each={table.getRowModel().rows}>
            {(row) => {
              const isFlagged = row.original.security_flags.length > 0;
              return (
                <tr class={isFlagged ? "flagged" : undefined}>
                  <For each={row.getVisibleCells()}>
                    {(cell) => (
                      <td>
                        {flexRender(
                          cell.column.columnDef.cell,
                          cell.getContext(),
                        )}
                      </td>
                    )}
                  </For>
                </tr>
              );
            }}
          </For>
        </tbody>
      </table>

      {data().length === 0 && !loading() && (
        <p class="empty">No messages found. Click Refresh to load.</p>
      )}

      <div class="table-controls">
        <button class="primary-button" onClick={prevPage} disabled={!hasPrev()}>
          Prev
        </button>
        <button class="primary-button" onClick={nextPage} disabled={!hasNext()}>
          Next
        </button>
      </div>
    </div>
  );
}
