import { GitCompareArrows } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { Options, type OptionRow } from "@/tools/options";
import { nameOf } from "@/workspace";

/** A plan compared: the document, or a compared file. */
export interface PlanEntry {
  path: string;
  name: string;
  /** The plan's figures under the table's headers after its name, or why there are none. */
  cells: string[] | string;
}

/** Which column a phone's row shows beside a plan's name: Ends with. */
const ENDS_WITH = 1;

interface PlansProps {
  headers: string[];
  entries: PlanEntry[];
  highlighted: PlanEntry | undefined;
  highlight: (entry: PlanEntry) => void;
  caption: string;
}

/** A row per plan, its figures or why it has none, the highlighted one marked. */
export function Plans({
  headers,
  entries,
  highlighted,
  highlight,
  caption,
}: PlansProps) {
  const rows: OptionRow<PlanEntry>[] = entries.map((entry) => {
    const cells =
      typeof entry.cells === "string"
        ? [entry.cells, ...headers.slice(2).map(() => "")]
        : entry.cells;
    return {
      key: entry.path,
      cells: [entry.name, ...cells],
      narrow: entry.name,
      option: entry,
    };
  });
  return (
    <div className="space-y-2">
      <p className="text-muted-foreground text-xs">{caption}</p>
      <Options
        label={caption}
        columns={headers}
        rows={rows}
        narrowFigure={ENDS_WITH}
        highlighted={highlighted}
        highlight={highlight}
      />
    </div>
  );
}

interface ActionsProps {
  isDocument: boolean;
  isBaseline: boolean;
  canOpen: boolean;
  onOpen: () => void;
  onBaseline: () => void;
  onRemove: () => void;
}

/** The files beside the document to compare it with, ticked while they are. */
export function CompareWith({
  offered,
  compared,
  onCompared,
  isPrimary = false,
}: {
  offered: readonly string[];
  compared: readonly string[];
  onCompared: (paths: string[]) => void;
  /** Whether it is what the page is for while one plan is shown. */
  isPrimary?: boolean;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant={isPrimary ? "default" : "outline"}
          disabled={offered.length === 0}
        >
          <GitCompareArrows aria-hidden />
          Compare with
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        {offered.map((path) => (
          <DropdownMenuCheckboxItem
            key={path}
            checked={compared.includes(path)}
            onSelect={(event) => {
              event.preventDefault();
            }}
            onCheckedChange={(isChecked) => {
              onCompared(
                isChecked
                  ? [...compared, path]
                  : compared.filter((each) => each !== path),
              );
            }}
          >
            {nameOf(path)}
          </DropdownMenuCheckboxItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** What can be done with the highlighted plan. */
export function PlanActions({
  isDocument,
  isBaseline,
  canOpen,
  onOpen,
  onBaseline,
  onRemove,
}: ActionsProps) {
  return (
    <div className="flex flex-wrap gap-2">
      {!isDocument && (
        <Button onClick={onOpen} disabled={!canOpen}>
          Open
        </Button>
      )}
      {!isBaseline && (
        <Button variant="outline" onClick={onBaseline}>
          Make baseline
        </Button>
      )}
      {!isDocument && (
        <Button variant="outline" onClick={onRemove}>
          Stop comparing
        </Button>
      )}
    </div>
  );
}

/** What the highlighted plan changes of the baseline, or why that is not said. */
export function Changes({
  title,
  lines,
  isNote,
}: {
  title: string;
  lines: string[];
  /** Whether `lines` say something of the plans rather than list changes. */
  isNote: boolean;
}) {
  return (
    <Card className="gap-2 py-4">
      <CardHeader className="px-4">
        <CardTitle className="text-base">{title}</CardTitle>
      </CardHeader>
      <CardContent className="px-4">
        <ul
          className={cn(
            "max-h-80 space-y-1 overflow-auto text-sm",
            isNote && "text-muted-foreground",
          )}
        >
          {lines.map((line, at) => (
            <li key={at}>{line}</li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
}
