# Releasing

A release publishes the engine, the client every interface shares, the command
line, the agent server, the planner's library and the binary to crates.io,
attaches Linux binaries to a GitHub Release, and deploys the site to GitHub
Pages: the web app at its root and the canvas page beside it at `/ratzilla`.
While the version is below 1.0, a breaking change to the engine's API bumps the
minor version.

1. **Release branch.** On `chore/release-x.y.z`, rename `## [Unreleased]` in
   `CHANGELOG.md` to `## [x.y.z] - YYYY-MM-DD`, open a new empty
   `## [Unreleased]` above it, and set the workspace `version` and every
   `retiretui` crate's `version` under `[workspace.dependencies]` in the root
   `Cargo.toml`, then run `cargo check` so `Cargo.lock` records them - the
   release build is `--locked`. Merge it through a pull request.
2. **Rehearse the deploy.** Once CI is green on the merge commit, run
   `.github/workflows/pages.yml` by hand from `main` and look at the live site:
   the web app at the root, its footer naming the new version, and the canvas
   page at `/ratzilla/`. Nothing after this step can be taken back, and the
   workflow builds without running a test, so this is where a site that does not
   build or does not start is caught. It deploys only once the repository's
   Pages source is "GitHub Actions" and its `github-pages` environment allows
   both `main` and `v*` tags.
3. **Publish the crates** from the merge commit:

   ```sh
   cargo publish --workspace
   ```

   A published version cannot be replaced, and the checks the tag runs come
   after it. Make them by hand first: the `retiretui` crate's version is
   `x.y.z`, and `CHANGELOG.md` has a `## [x.y.z]` section that is not empty.

4. **Tag and push:**

   ```sh
   git tag vx.y.z
   git push origin vx.y.z
   ```

   The tag runs `.github/workflows/release.yml`, which refuses a tag that is not
   `v` and the `retiretui` crate's version, cuts the release notes from the
   changelog's section for it, builds the binaries and creates the GitHub
   Release. It also runs `pages.yml`, which deploys the site again whatever CI
   or the release says of the commit, so tag only the commit rehearsed.
