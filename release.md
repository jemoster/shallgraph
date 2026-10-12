# Release Instructions

This guide describes the minimum steps to cut a new GitHub release that publishes installable artifacts for `shallgraph`.

The `@shallgraph/core` package, `shallgraph-vscode` extension, and Cargo workspace share one version. The GitHub release tag must be `v` plus that version. Packed npm tarball and VSIX names include that version.

## 1) Prepare the release

1. Ensure your working tree is clean and all required checks are green.
2. Set the shared version (updates workspace package files, lockfiles, and README install URLs):

```bash
./scripts/bump-version.py X.Y.Z
```

3. Commit the version bump and merge the release-ready changes to the default branch.

## 2) Create and push a version tag

Tag **exactly** `vX.Y.Z` to match the version from step 1.

```bash
git checkout main
git pull
git tag vX.Y.Z
git push origin vX.Y.Z
```

Use semantic versioning for `vX.Y.Z`.

## 3) Publish the GitHub Release

1. Open GitHub Releases for the repository.
2. Create a new release from tag `vX.Y.Z`.
3. Add concise release notes describing notable changes.
4. Publish the release.

Publishing the release triggers automation that packs artifacts and uploads them. The jobs fail if the tag does not match the shared package version or if packed npm/VSIX filenames do not include that version.

The VS Code release job also publishes the versioned VSIX to the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=shallgraph.shallgraph-vscode) and to [Open VSX](https://open-vsx.org/extension/shallgraph/shallgraph-vscode). Cursor installs third-party extensions from Open VSX after its marketplace proxy finishes review. Re-running a release for a version that is already published is skipped on both registries.

### Visual Studio Marketplace secret

Store an Azure DevOps personal access token as the `VSCE_PAT` repository secret on `jemoster/shallgraph`. Create the token at `https://dev.azure.com/{your-org}/_usersSettings/tokens` with **Organization** set to **All accessible organizations**, **Scopes** set to **Custom defined**, then **Show all scopes** and **Marketplace** → **Manage**. Marketplace **Publish** is not sufficient; see [Get a Personal Access Token](https://code.visualstudio.com/api/working-with-extensions/publishing-extension#get-a-personal-access-token). The publisher ID in `packages/vscode/package.json` (`shallgraph`) must already exist on the marketplace.

### Open VSX secret

Cursor does not read the Visual Studio Marketplace. Publishing to Open VSX is what makes the extension available in Cursor. Do these steps once, before the next GitHub Release. GitHub is the Open VSX login and the owner of the access token. The Eclipse Foundation runs the registry and will not accept a publish until an Eclipse account linked to that GitHub login has signed the Open VSX Publisher Agreement. Without that signature, publishing fails with "You must log in with an Eclipse Foundation account and sign a Publisher Agreement".

1. Create an [Eclipse account](https://accounts.eclipse.org/user/register). The form asks for name, email, and password only. It does not ask for a GitHub username. Do not sign the Eclipse Contributor Agreement. It does not allow publishing.
2. Sign in to [open-vsx.org](https://open-vsx.org) with the GitHub account that should publish. Open [the profile page](https://open-vsx.org/user-settings/profile) and choose **Log in with Eclipse**. Authorize the Eclipse account from step 1. That step links the two identities. **Show Publisher Agreement** appears only after the link succeeds. Read it to the bottom and choose Agree. If the link fails, sign in at [Edit my account](https://accounts.eclipse.org/user/edit), set **GitHub ID** under Account Settings to that same GitHub username, save, and repeat **Log in with Eclipse**.
3. Create an access token at [open-vsx.org/user-settings/tokens](https://open-vsx.org/user-settings/tokens). Store it as the GitHub Actions secret `OVSX_PAT` on `jemoster/shallgraph` (Settings → Secrets and variables → Actions). `VSCE_PAT` is still required for the Visual Studio Marketplace step.
4. Create the `shallgraph` namespace once. The publisher id does not match a GitHub username, so the registry will not create it on first upload:

```bash
npx ovsx create-namespace shallgraph -p "$OVSX_PAT"
```

5. After the first successful publish, claim namespace ownership so Open VSX does not show the unverified-publisher warning. Open a **Namespace** issue in [EclipseFdn/open-vsx.org](https://github.com/EclipseFdn/open-vsx.org/issues/new/choose). The GitHub account needs at least one year of history. Because `shallgraph` is also a Visual Studio Marketplace publisher and this repository is public, the claim needs a commit URL in `jemoster/shallgraph` authored by that GitHub account. See [Managing Namespaces](https://github.com/EclipseFdn/open-vsx.org/wiki/Managing-Namespaces).

Cursor's own verification badge is separate and optional. It needs a website on its own domain that links to the Open VSX listing, the Open VSX homepage field pointed at that site, and a post in Cursor's Extension Verification forum. A GitHub README is not accepted. Publishing to Open VSX is enough for the extension to be offered in Cursor after the proxy review. No extra Cursor account is required.

## 4) Verify artifacts and install path

1. Confirm release automation completed successfully.
2. Confirm required artifacts are attached to the release, including the `@shallgraph/core` tarball `shallgraph-core-X.Y.Z.tgz`, the VS Code extension `shallgraph-vscode-X.Y.Z.vsix`, and the Linux x86_64 native CLI binary (`shallgraph-linux-x86_64`) produced by workflow automation.
3. Confirm the VS Code extension version is listed on the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=shallgraph.shallgraph-vscode) and on [Open VSX](https://open-vsx.org/extension/shallgraph/shallgraph-vscode). Confirm **ShallGraph** can be installed from the Visual Studio Code Extensions view and, after Cursor's marketplace review, from the Cursor Extensions view.
4. On Linux x86_64, confirm the native binary can be downloaded from the same release, marked executable, and run as `shallgraph`.

## Testing release generation

Use these steps before (or instead of) publishing a GitHub Release.

### Local packaging (same scripts as CI)

```bash
./scripts/package.sh
./scripts/package-native-cli.sh
```

Confirm `release/` contains `shallgraph-core-X.Y.Z.tgz` named with the shared version and `shallgraph-linux-x86_64`. On Linux x86_64, run `./release/shallgraph-linux-x86_64 --help`.

### Download the native binary from a branch or pull request

The Tests workflow packages the native CLI and uploads it as a workflow artifact named `shallgraph-linux-x86_64`.

From the GitHub UI: open the Actions run for the branch, open the `cargo test` job, and download the `shallgraph-linux-x86_64` artifact.

From the CLI (replace the run id with the latest Tests run for the branch):

```bash
gh run list --workflow=test.yml --branch <branch> --limit 1
gh run download <run-id> --name shallgraph-linux-x86_64
chmod +x shallgraph-linux-x86_64
./shallgraph-linux-x86_64 --help
```

### Dry-run the CLI release workflow

Actions → **Release CLI artifacts** → **Run workflow**. Manual `workflow_dispatch` runs the same packaging as a published release, then uploads `cli-tarballs` and `shallgraph-linux-x86_64` as workflow artifacts. It does not create or modify a GitHub Release.

### Beta of the next version

The Visual Studio Marketplace rejects semver pre-release identifiers such as `0.10.0-beta.1`. A beta uses the next `X.Y.Z` and a GitHub Release marked as a pre-release. The tag is still `vX.Y.Z`. The VS Code release job packages that VSIX with `--pre-release` and publishes it as a pre-release, so Marketplace and Open VSX do not replace the current stable version.

### Full GitHub Release path

Publish a GitHub Release from a tag. Mark it as a pre-release for a beta. That is the only trigger that attaches assets with `gh release upload` and publishes the VSIX to the Visual Studio Marketplace and to Open VSX. Confirm the release page includes `shallgraph-core-X.Y.Z.tgz`, `shallgraph-vscode-X.Y.Z.vsix`, and `shallgraph-linux-x86_64`. Confirm both registry listings match that same version.
