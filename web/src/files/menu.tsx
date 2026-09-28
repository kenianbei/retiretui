import { Link } from "@tanstack/react-router";
import {
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

/** The open document's name, and what can be done with files. */
export function FileMenu() {
  const session = useSession();
  const actions = useFileActions();

  return (
    <div className="flex min-w-0 flex-1 items-center justify-between gap-3">
      <div className="flex min-w-0 items-center gap-2">
        {session.document && session.path !== null && (
          <span className="truncate font-medium">{nameOf(session.path)}</span>
        )}
        {session.document?.isReadOnly && (
          <Badge variant="secondary">Scenario · read-only</Badge>
        )}
      </div>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="sm">
            <Menu aria-hidden />
            File
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
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
    </div>
  );
}
