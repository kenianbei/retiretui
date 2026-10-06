import type { ReactNode } from "react";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

/** A card under its title, a heading of the page's second level or its third. */
export function Titled({
  title,
  isLeading = false,
  children,
}: {
  title: string;
  /** Whether it leads the others, its title their heading. */
  isLeading?: boolean;
  children: ReactNode;
}) {
  const Heading = isLeading ? "h2" : "h3";
  return (
    <Card className="gap-3 py-4">
      <CardHeader className="px-4">
        <CardTitle>
          <Heading>{title}</Heading>
        </CardTitle>
      </CardHeader>
      <CardContent className="@container space-y-3 px-4">
        {children}
      </CardContent>
    </Card>
  );
}
