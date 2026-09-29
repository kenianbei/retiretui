import { Link } from "@tanstack/react-router";
import {
  ChevronDown,
  Download,
  FilePen,
  FilePlus2,
  FolderCog,
  FolderOpen,
  Menu,
  Sparkles,
  Upload,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useFileActions } from "@/files/actions";
import { NEW_PLAN_START } from "@/onboarding/steps";
import { useSession } from "@/session";
import { nameOf } from "@/workspace";

/** What a scenario open in the header is marked as. */
const READ_ONLY = "Scenario · read-only";

/** The open document's name, and what can be done with files. */
export function FileMenu() {
  const session = useSession();
  const actions = useFileActions();
  const name =
    session.document && session.path !== null ? nameOf(session.path) : null;
  const isReadOnly = session.document?.isReadOnly === true;

  return (
    <div className="flex min-w-0 flex-1 items-center gap-2">
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="ghost"
            size="sm"
            title={name ?? undefined}
            className="min-w-0 shrink justify-start px-2 text-base font-medium"
          >
            {name === null ? (
              <>
                <Menu aria-hidden />
                File
              </>
            ) : (
              <>
                <span className="sr-only">File, </span>
                <span className="truncate">{name}</span>
              </>
            )}
            <ChevronDown aria-hidden className="text-muted-foreground" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          {isReadOnly && (
            <>
              <DropdownMenuLabel className="text-muted-foreground font-normal md:hidden">
                {READ_ONLY}
              </DropdownMenuLabel>
              <DropdownMenuSeparator className="md:hidden" />
            </>
          )}
          <DropdownMenuItem asChild>
            <Link to="/new/$step" params={{ step: NEW_PLAN_START }}>
              <FilePen aria-hidden />
              New plan…
            </Link>
          </DropdownMenuItem>
          <DropdownMenuSub>
            <DropdownMenuSubTrigger disabled={session.files.length === 0}>
              <FolderOpen aria-hidden />
              Open
            </DropdownMenuSubTrigger>
            <DropdownMenuSubContent>
              {session.files.map((path) => (
                <DropdownMenuItem
                  key={path}
                  onSelect={() => {
                    session.open(path);
                  }}
                >
                  {nameOf(path)}
                </DropdownMenuItem>
              ))}
            </DropdownMenuSubContent>
          </DropdownMenuSub>
          <DropdownMenuItem
            disabled={session.files.length === 0}
            onSelect={actions.manage}
          >
            <FolderCog aria-hidden />
            Manage plans…
          </DropdownMenuItem>
          <DropdownMenuSub>
            <DropdownMenuSubTrigger>
              <Sparkles aria-hidden />
              Add an example
            </DropdownMenuSubTrigger>
            <DropdownMenuSubContent className="max-w-80">
              {actions.examples.map((example) => (
                <DropdownMenuItem
                  key={example.file}
                  onSelect={() => {
                    actions.add(example.file, example.text);
                  }}
                >
                  {example.words}
                </DropdownMenuItem>
              ))}
            </DropdownMenuSubContent>
          </DropdownMenuSub>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            disabled={session.document === null}
            onSelect={actions.saveAs}
          >
            <FilePlus2 aria-hidden />
            Save as…
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={actions.upload}>
            <Upload aria-hidden />
            Upload a plan…
          </DropdownMenuItem>
          <DropdownMenuItem
            disabled={session.path === null}
            onSelect={() => {
              actions.download();
            }}
          >
            <Download aria-hidden />
            Download {session.path !== null && nameOf(session.path)}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      {isReadOnly && (
        <Badge variant="secondary" className="shrink-0 max-md:hidden">
          {READ_ONLY}
        </Badge>
      )}
    </div>
  );
}
