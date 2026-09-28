import { FileUp } from "lucide-react";
import { useRef, useState, type ChangeEvent } from "react";

import { Button } from "@/components/ui/button";
import { messageOf } from "@/lib/utils";
import { useSession } from "@/session";

/** The person at `index`'s earnings recorded from an ssa.gov statement. */
export function ImportStatement({ index }: { index: number }) {
  const session = useSession();
  const picker = useRef<HTMLInputElement>(null);
  const [outcome, setOutcome] = useState<{
    said: string;
    isRefused: boolean;
  } | null>(null);

  const record = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    try {
      const said = session.importEarnings(index, await file.text());
      if (said !== undefined) setOutcome({ said, isRefused: false });
    } catch (thrown) {
      setOutcome({ said: messageOf(thrown), isRefused: true });
    }
  };

  return (
    <>
      <Button
        size="sm"
        variant="outline"
        onClick={() => picker.current?.click()}
      >
        <FileUp aria-hidden />
        Import statement
      </Button>
      <input
        ref={picker}
        type="file"
        accept=".xml"
        hidden
        aria-label="Social Security statement"
        onChange={(event) => void record(event)}
      />
      {outcome && (
        <p
          role={outcome.isRefused ? "alert" : "status"}
          className={
            outcome.isRefused
              ? "text-destructive w-full text-sm"
              : "text-muted-foreground w-full text-sm"
          }
        >
          {outcome.said}
        </p>
      )}
    </>
  );
}
