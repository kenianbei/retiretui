interface Window {
  /** Set by `index.html`, so that a failure is said where no module runs. */
  startup: {
    /** Says what was thrown in place of the app, unless a cause is already named. */
    threw: (thrown: unknown) => void;
    /** The app's script has run: an error seen on the way was not its own. */
    started: () => void;
  };
}
