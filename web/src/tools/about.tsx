/** What a tool is for, in the client's line, under its heading. */
export function ToolAbout({ about }: { about: string }) {
  return <p className="text-muted-foreground w-full max-w-prose">{about}</p>;
}
