import {
  claimWords,
  type Bases,
  type RothOwner,
} from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import { ROTH_CONVERSIONS, SSA_BENEFITS } from "@/nav";
import { Rows, ToolRow } from "@/overview/lists";
import type { Basis } from "@/overview/words";
import { useClaims, useLadders } from "@/searches";
import { useSession } from "@/session";

const WORDS = claimWords();
const SEARCHING = "Searching…";

/** A Roth owner's best ladder, leading to the tool aimed at their account. */
function LadderRow({
  plan,
  owner,
  basis,
}: {
  plan: string;
  owner: RothOwner;
  basis: Basis;
}) {
  const session = useSession();
  const { reading } = session;
  const constraints = useMemo(
    () => reading.document?.constraintsText ?? "",
    [reading],
  );
  const found = useLadders(plan, constraints, owner.destination, true);
  const said = found.data
    ? found.data.better[basis]
    : found.error
      ? WORDS.refused
      : SEARCHING;
  return (
    <ToolRow
      page={ROTH_CONVERSIONS}
      onClick={() => {
        session.change((document) => {
          document.aimAt(owner.destination);
        });
      }}
    >
      {owner.name}: {said}
    </ToolRow>
  );
}

/** The household's best claims, leading to the SSA Benefits tool. */
function ClaimsRow({
  found,
  basis,
}: {
  found: { data?: { better: Bases<string> }; error: Error | null };
  basis: Basis;
}) {
  if (found.error) return null;
  return (
    <ToolRow page={SSA_BENEFITS}>
      {found.data?.better[basis] ?? SEARCHING}
    </ToolRow>
  );
}

/**
 * Could do better: each Roth owner's best ladder and the household's best
 * claims against the plan as it stands, searched as their tools search
 * them, each leading to its tool.
 */
export function Better({
  plan,
  held,
  basis,
}: {
  plan: string;
  held: readonly string[];
  basis: Basis;
}) {
  const { reading } = useSession();
  const owners = useMemo(() => reading.document?.rothOwners() ?? [], [reading]);
  const claims = useClaims(plan, held, true);
  const isNothing = owners.length === 0 && claims.error !== null;
  return (
    <section aria-labelledby="better" className="max-w-3xl space-y-3">
      <h2 id="better" className="text-lg font-semibold">
        {WORDS.could_do_better}
      </h2>
      <Rows>
        {owners.map((owner) => (
          <LadderRow
            key={owner.destination}
            plan={plan}
            owner={owner}
            basis={basis}
          />
        ))}
        <ClaimsRow found={claims} basis={basis} />
        {isNothing && (
          <li className="text-muted-foreground px-4 py-2.5 text-sm">
            {WORDS.nothing_to_search}
          </li>
        )}
      </Rows>
    </section>
  );
}
