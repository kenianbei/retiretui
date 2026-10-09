import { useNavigate } from "@tanstack/react-router";
import type { Editor } from "@wasm/retiretui_wasm.js";
import { useMemo, useState } from "react";

import { messageOf } from "@/lib/utils";
import type { PlanSearch } from "@/plan/search";
import { FormSheet } from "@/plan/sheet";
import { useSession } from "@/session";

interface ItemFormProps {
  slug: string;
  edit: number | "new";
  /** The field to scroll to, where a link named one. */
  field: string | undefined;
}

/** The editor over the item `edit` names, or `null` where there is none. */
function useEditor(slug: string, edit: number | "new"): Editor | null {
  const { document } = useSession();
  return useMemo(() => {
    if (!document) return null;
    if (edit === "new") return document.create(slug);
    try {
      return document.edit(slug, edit);
    } catch {
      return null;
    }
  }, [document, slug, edit]);
}

/** The item open in its form, over the page. */
export function ItemForm({ slug, edit, field }: ItemFormProps) {
  const session = useSession();
  const navigate = useNavigate({ from: "/plan/$page" });
  const editor = useEditor(slug, edit);
  const [refusal, setRefusal] = useState<string | null>(null);

  if (!editor) return null;

  const close = () => {
    void navigate({
      search: { item: edit === "new" ? undefined : edit },
      resetScroll: false,
    });
  };

  /** Stores the item and closes the form, or shows why it was refused. */
  const apply = () => {
    try {
      const stored = session.apply(editor);
      setRefusal(null);
      const item = stored ?? (edit === "new" ? undefined : edit);
      void navigate({ search: { item }, resetScroll: false });
    } catch (thrown) {
      setRefusal(messageOf(thrown));
    }
  };

  return (
    <FormSheet
      editor={editor}
      isStaying={(next) =>
        next.pathname === `/plan/${slug}` &&
        (next.search as PlanSearch).edit === edit
      }
      field={field}
      refusal={refusal}
      apply={apply}
      close={close}
    />
  );
}
