# Releasing

A release publishes both crates to crates.io and attaches Linux binaries to a
GitHub Release. While the version is below 1.0, a breaking change to the
engine's API bumps the minor version.

1. **Release branch.** On `chore/release-x.y.z`, rename `## [Unreleased]` in
   `CHANGELOG.md` to `## [x.y.z] - YYYY-MM-DD`, open a new empty
   `## [Unreleased]` above it, and set the workspace `version` and the
   `retiretui_engine` dependency's `version` in the root `Cargo.toml`, then run
   `cargo check` so `Cargo.lock` records them - the release build is `--locked`.
   Merge it through a pull request.
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
   creates the GitHub Release.
