import { useNavigate, useSearch } from "@tanstack/react-router";
import {
  type EstimateRow,
  type EstimateView,
  Estimator,
  estimateWords,
} from "@wasm/retiretui_wasm.js";
import { FileUp, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { useFilePicker } from "@/files/picker";
import { messageOf } from "@/lib/utils";
import { useSession } from "@/session";
import { ToolAbout } from "@/tools/about";
import { type Chosen, SearchActions } from "@/tools/act";
import {
  AskDialog,
  type Asking,
  BehindTable,
  type RecordActs,
} from "@/tools/estimate/records";
import { type OptionRow, Options } from "@/tools/options";
import type { ToolSearch } from "@/tools/search";
import { type Settings, SettingsForm, SettingsRead } from "@/tools/settings";
import { offeredName } from "@/workspace";
import { IN_PLACE } from "@/year/search";

const WORDS = estimateWords();

/** The exports read this session, kept across the pages shown. */
const ESTIMATOR = new Estimator();

/** The column a phone's row shows beside a row's name. */
const PROPOSED = 2;

/** How the plan's `[estimate]` table is read out, opened and held. */
const ESTIMATE_SETTINGS: Settings = {
  id: "settings",
  heading: "Settings",
  read: (document) => document.estimateSettingsRead(),
  open: (document) => document.estimateSettings(),
  apply: (document, editor) => {
    document.apply(editor);
  },
};

/** Each row of the estimate, keyed by its place, its nesting a mark. */
function rowsOf(view: EstimateView): OptionRow<EstimateRow>[] {
  return view.rows.map((row, at) => {
    const name = row.cells[0].trimStart();
    return {
      key: String(at),
      cells: [name, ...row.cells.slice(1)],
      narrow: name,
      isNested: row.is_nested,
      option: row,
    };
  });
}

/** The proposals, as they are taken or written. */
function chosenProposals(path: string | null, question: string): Chosen {
  return {
    noun: "these proposals",
    question,
    described:
      "The proposals are written as a scenario over this plan, to a file of this name in your workspace.",
    offered: offeredName(path, "estimate"),
    take: (document) => ESTIMATOR.take(document),
    scenario: (document, out) => ESTIMATOR.scenario(document, out),
  };
}

/**
 * The Spending Estimate tool: a bank's exports read for the session, what
 * they say the plan spends proposed for its expenses, the highlighted
 * proposal read out and edited, and the proposals taken or written as a
 * scenario.
 */
export function EstimatePage({ title }: { title: string }) {
  const search: ToolSearch = useSearch({ from: "/tools/$page" });
  const navigate = useNavigate({ from: "/tools/$page" });
  const { reading, path } = useSession();
  const [view, setView] = useState<EstimateView | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const refresh = useCallback(() => {
    if (reading.document) setView(ESTIMATOR.estimate(reading.document));
  }, [reading]);
  useEffect(refresh, [refresh]);
  const picker = useFilePicker(WORDS.accept, "Bank export", async (file) => {
    try {
      ESTIMATOR.addExport(file.name, await file.text());
      setProblem(null);
    } catch (thrown) {
      setProblem(messageOf(thrown));
    }
    refresh();
  });
  const remove = (name: string) => {
    ESTIMATOR.removeExport(name);
    refresh();
  };
  const rows = useMemo(() => (view ? rowsOf(view) : []), [view]);
  const at = Math.min(search.row ?? 0, rows.length - 1);
  const highlighted = rows[at]?.option;
  const proposal: Settings = useMemo(
    () => ({
      id: "row",
      heading: highlighted?.title ?? "Row",
      isEditable: highlighted?.is_editable ?? false,
      read: () => ESTIMATOR.readRow(at),
      open: (document) => ESTIMATOR.openRow(document, at),
      apply: (document, editor) => {
        ESTIMATOR.applyRow(document, editor, at);
        refresh();
      },
    }),
    [highlighted, at, refresh],
  );
  const [said, setSaid] = useState<string | null>(null);
  const [asking, setAsking] = useState<Asking | null>(null);
  const acts: RecordActs = useMemo(
    () => ({
      categories: WORDS.categories,
      bucket: view?.bucket ?? "",
      act: (record, action) => {
        try {
          setSaid(ESTIMATOR.act(at, record, action));
          setProblem(null);
        } catch (thrown) {
          setProblem(messageOf(thrown));
        }
        refresh();
      },
      ask: setAsking,
      reach: (words) => ESTIMATOR.reach(words),
    }),
    [at, refresh, view?.bucket],
  );
  const behind = useMemo(
    () => (view && highlighted ? ESTIMATOR.behindRow(at) : null),
    [view, highlighted, at],
  );

  return (
    <div className="space-y-6">
      <div className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
        <ToolAbout about={WORDS.about} />
      </div>
      <section aria-labelledby="files" className="space-y-3">
        <div className="flex flex-wrap items-center gap-3">
          <h2 id="files" className="mr-auto text-lg font-semibold">
            Files
          </h2>
          <Button size="sm" variant="outline" onClick={picker.open}>
            <FileUp aria-hidden />
            Add export
          </Button>
          {picker.element}
        </div>
        {view && view.files.length > 0 ? (
          <ul className="bg-card divide-y rounded-md border text-sm">
            {view.files.map(([name, rows]) => (
              <li
                key={name}
                className="flex items-center justify-between gap-3 px-4 py-2"
              >
                <span>
                  {name} <span className="text-muted-foreground">{rows}</span>
                </span>
                <Button
                  size="icon-sm"
                  variant="ghost"
                  aria-label={`Remove ${name}`}
                  onClick={() => {
                    remove(name);
                  }}
                >
                  <X aria-hidden />
                </Button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-muted-foreground">{WORDS.nothing_read}</p>
        )}
        {problem && (
          <p role="alert" className="text-destructive text-sm">
            {problem}
          </p>
        )}
      </section>
      {view && rows.length > 0 && (
        <section aria-labelledby="proposals" className="min-w-0 space-y-3">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <h2 id="proposals" className="text-lg font-semibold">
              Proposals
            </h2>
            {view.question && (
              <SearchActions
                chosen={chosenProposals(path, view.question)}
                isCurrent
              />
            )}
          </div>
          <p className="text-muted-foreground text-sm">{view.summary}</p>
          {said && (
            <p role="status" className="text-sm">
              {said}
            </p>
          )}
          <Options
            label="Proposals"
            columns={WORDS.columns}
            rows={rows}
            narrowFigure={PROPOSED}
            words={1}
            highlighted={highlighted}
            highlight={(row) => {
              const place = rows.findIndex((each) => each.option === row);
              void navigate({
                search: (kept) => ({ ...kept, row: place }),
                ...IN_PLACE,
              });
            }}
          />
          {highlighted && <SettingsRead settings={proposal} />}
          {behind && behind.rows.length > 0 && (
            <BehindTable behind={behind} acts={acts} />
          )}
          {asking && (
            <AskDialog
              key={`${asking.kind}-${String(asking.record)}`}
              asking={asking}
              close={() => {
                setAsking(null);
              }}
              acts={acts}
            />
          )}
        </section>
      )}
      <SettingsRead settings={ESTIMATE_SETTINGS} />
      {search.edit === proposal.id && highlighted && (
        <SettingsForm settings={proposal} />
      )}
      {search.edit === ESTIMATE_SETTINGS.id && (
        <SettingsForm settings={ESTIMATE_SETTINGS} />
      )}
    </div>
  );
}
