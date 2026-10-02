interface Window {
  /** Set by `index.html`, so that a failure is said where no module runs. */
  startup: {
    /** Says what was thrown in place of the app; the first failure said stands. */
    threw: (thrown: unknown) => void;
    /** The app has mounted: what goes wrong from here is no longer a failure to start. */
    done: () => void;
  };
}
