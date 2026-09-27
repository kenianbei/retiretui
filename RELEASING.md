# Releasing

A release publishes the engine, the client every interface shares, the command
line, the agent server, the planner's library and the binary to crates.io,
attaches Linux binaries to a GitHub Release, and deploys the browser page to
GitHub Pages. While the version is below 1.0, a breaking change to the engine's
API bumps the minor version.

1. **Release branch.** On `chore/release-x.y.z`, rename `## [Unreleased]` in
   `CHANGELOG.md` to `## [x.y.z] - YYYY-MM-DD`, open a new empty
   `## [Unreleased]` above it, and set the workspace `version` and every
   `retiretui` crate's `version` under `[workspace.dependencies]` in the root
   `Cargo.toml`, then run `cargo check` so `Cargo.lock` records them - the
   release build is `--locked`. Merge it through a pull request.
2. **Publish the crates** from the merge commit:

   ```sh
   cargo publish --workspace
   ```

3. **Tag and push:**

   ```sh
   git tag vx.y.z
   git push origin vx.y.z
   ```

   The tag runs `.github/workflows/release.yml`, which builds the binaries and
   creates the GitHub Release, and `.github/workflows/pages.yml`, which builds
   the browser page and deploys it. The page deploys only once the repository's
   Pages source is "GitHub Actions" and its `github-pages` environment allows
   `v*` tags; `pages.yml` can also be run by hand from `main`.
