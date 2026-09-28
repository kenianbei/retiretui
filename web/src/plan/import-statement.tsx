import { FileUp } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { useFilePicker } from "@/files/picker";
import { cn, messageOf } from "@/lib/utils";
import { useSession } from "@/session";

/** The earnings of the person at `index`, called `name`, from an ssa.gov statement. */
export function ImportStatement({
  index,
  name,
}: {
  index: number;
  name: string;
}) {
  const session = useSession();
  const [outcome, setOutcome] = useState<{
    said: string;
    isRefused: boolean;
  } | null>(null);
  const picker = useFilePicker(
    ".xml",
    "Social Security statement",
    async (file) => {
      try {
        const xml = await file.text();
        const said = session.change((document) =>
          document.importEarnings(index, name, xml),
        );
        if (said !== undefined) setOutcome({ said, isRefused: false });
      } catch (thrown) {
        setOutcome({ said: messageOf(thrown), isRefused: true });
      }
    },
  );

  return (
    <>
      <Button size="sm" variant="outline" onClick={picker.open}>
        <FileUp aria-hidden />
        Import statement
      </Button>
      {picker.element}
      {outcome && (
        <p
          role={outcome.isRefused ? "alert" : "status"}
          className={cn(
            "w-full text-sm",
            outcome.isRefused ? "text-destructive" : "text-muted-foreground",
          )}
        >
          {outcome.said}
        </p>
      )}
    </>
  );
}
