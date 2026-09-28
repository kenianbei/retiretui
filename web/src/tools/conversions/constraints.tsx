import { Link, useNavigate } from "@tanstack/react-router";
import { Pencil } from "lucide-react";
import { useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { messageOf } from "@/lib/utils";
import { ReadRows } from "@/plan/read-out";
import { FormSheet } from "@/plan/sheet";
import { useSession } from "@/session";
import type { ToolSearch } from "@/tools/search";

/** The constraints the ladders are searched under, read out, and Edit. */
export function Constraints() {
  const { reading } = useSession();
  const rows = useMemo(
    () => reading.document?.constraintsRead() ?? [],
    [reading],
  );
  return (
    <section aria-labelledby="constraints" className="space-y-3">
      <div className="flex items-center gap-2">
        <h2 id="constraints" className="mr-auto text-lg font-semibold">
          Constraints
        </h2>
        <Button size="sm" asChild>
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

/** The constraints open in their form, held whole once applied. */
export function ConstraintsForm() {
  const session = useSession();
  const navigate = useNavigate({ from: "/tools/$page" });
  const { document } = session;
  const editor = useMemo(() => document?.constraints() ?? null, [document]);
  const [refusal, setRefusal] = useState<string | null>(null);

  if (!editor) return null;

  const close = () => {
    void navigate({
      search: (held) => ({ ...held, edit: undefined }),
    });
  };

  const apply = () => {
    try {
      session.applyConstraints(editor);
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
