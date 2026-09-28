import * as React from "react"
import { X } from "lucide-react"
import { Dialog as DialogPrimitive } from "radix-ui"

import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

const Dialog = DialogPrimitive.Root
const DialogTitle = DialogPrimitive.Title
const DialogDescription = DialogPrimitive.Description

function DialogContent({
  className,
  children,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Content>) {
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Overlay className="fixed inset-0 z-40 bg-black/40" />
      <DialogPrimitive.Content
        data-slot="dialog-content"
        className={cn("bg-background fixed z-50 shadow-xl", className)}
        {...props}
      >
        {children}
      </DialogPrimitive.Content>
    </DialogPrimitive.Portal>
  )
}

function DialogHeader({
  title,
  className,
}: {
  title: React.ReactNode
  className?: string
}) {
  return (
    <div className={cn("flex items-center gap-3", className)}>
      <DialogPrimitive.Title className="mr-auto truncate text-lg font-semibold">
        {title}
      </DialogPrimitive.Title>
      <DialogPrimitive.Close asChild>
        <Button variant="ghost" size="icon" aria-label="Close">
          <X aria-hidden />
        </Button>
      </DialogPrimitive.Close>
    </div>
  )
}

export { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle }
