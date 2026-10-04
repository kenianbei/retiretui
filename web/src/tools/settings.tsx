import { Link, useNavigate } from "@tanstack/react-router";
import type { Document, Editor, ReadRow } from "@wasm/retiretui_wasm.js";
import { Pencil } from "lucide-react";
import { useId, useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { messageOf } from "@/lib/utils";
import { ReadRows } from "@/plan/read-out";
import { FormSheet } from "@/plan/sheet";
import { useSession } from "@/session";
import type { ToolSearch } from "@/tools/search";

/** A tool's own settings, held beside the draft: how they are read, opened and held. */
export interface Settings {
  /** What the section is headed. */
  heading: string;
  read: (document: Document) => ReadRow[];
  open: (document: Document) => Editor;
  apply: (document: Document, editor: Editor) => void;
}

/** A tool's settings read out, and Edit. */
export function SettingsRead({ settings }: { settings: Settings }) {
  const { reading } = useSession();
  const heading = useId();
  const rows = useMemo(
    () => (reading.document ? settings.read(reading.document) : []),
    [reading, settings],
  );
  return (
    <section aria-labelledby={heading} className="space-y-3">
      <div className="flex items-center gap-2">
        <h2 id={heading} className="mr-auto text-lg font-semibold">
          {settings.heading}
        </h2>
        <Button variant="outline" size="sm" asChild>
          <Link
            from="/tools/$page"
            to="."
            search={(held) => ({ ...held, edit: true })}
          >
            <Pencil aria-hidden />
            Edit
          </Link>
        </Button>
      </div>
      <ReadRows rows={rows} />
    </section>
  );
}

/** A tool's settings open in their form, held whole once applied. */
export function SettingsForm({ settings }: { settings: Settings }) {
  const session = useSession();
  const navigate = useNavigate({ from: "/tools/$page" });
  const { document } = session;
  const editor = useMemo(
    () => (document ? settings.open(document) : null),
    [document, settings],
  );
  const [refusal, setRefusal] = useState<string | null>(null);

  if (!editor) return null;

  const close = () => {
    void navigate({
      search: (held) => ({ ...held, edit: undefined }),
    });
  };

  const apply = () => {
    try {
      session.change((document) => {
        settings.apply(document, editor);
      });
      setRefusal(null);
      close();
    } catch (thrown) {
      setRefusal(messageOf(thrown));
    }
  };

  return (
    <FormSheet
      editor={editor}
      isStaying={(next) => (next.search as ToolSearch).edit === true}
      refusal={refusal}
      apply={apply}
      close={close}
    />
  );
}
