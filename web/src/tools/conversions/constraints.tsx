import type { Settings } from "@/tools/settings";

/** The constraints the ladders are searched under. */
export const CONSTRAINTS: Settings = {
  heading: "Constraints",
  read: (document) => document.constraintsRead(),
  open: (document) => document.constraints(),
  apply: (document, editor) => {
    document.applyConstraints(editor);
  },
};
