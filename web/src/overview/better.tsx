import { Link } from "@tanstack/react-router";
import {
  claimWords,
  type Bases,
  type RothOwner,
} from "@wasm/retiretui_wasm.js";
import { useMemo, type ReactNode } from "react";

import { ROTH_CONVERSIONS, SSA_BENEFITS } from "@/nav";
import type { Basis } from "@/overview/words";
import { useClaims, useLadders } from "@/searches";
import { useSession } from "@/session";

const WORDS = claimWords();
const SEARCHING = "Searching…";

function Row({ children }: { children: ReactNode }) {
  return <li className="px-4 py-3">{children}</li>;
}

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
    <Row>
      <Link
        to="/tools/$page"
        params={{ page: ROTH_CONVERSIONS }}
        search={(kept) => ({ basis: kept.basis, held: kept.held })}
        onClick={() => {
          session.change((document) => {
            document.aimAt(owner.destination);
          });
        }}
        className="underline-offset-4 hover:underline"
      >
        {owner.name}: {said}
      </Link>
    </Row>
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
    <Row>
      <Link
        to="/tools/$page"
        params={{ page: SSA_BENEFITS }}
        search={(kept) => ({ basis: kept.basis, held: kept.held })}
        className="underline-offset-4 hover:underline"
      >
        {found.data?.better[basis] ?? SEARCHING}
      </Link>
    </Row>
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
    <section aria-labelledby="better" className="space-y-3">
      <h2 id="better" className="text-lg font-semibold">
        {WORDS.could_do_better}
      </h2>
      <ul className="bg-card divide-y rounded-md border">
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
          <Row>
            <span className="text-muted-foreground">
              {WORDS.nothing_to_search}
            </span>
          </Row>
        )}
      </ul>
    </section>
  );
}
