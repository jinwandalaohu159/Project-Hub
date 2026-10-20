#!/usr/bin/env node
// One-step release: bump the version, push the tag and let GitHub Actions build and publish
// the Windows, macOS (Apple Silicon + Intel) and Linux installers.
//
//   npm run release                    -> next patch version (0.1.0 -> 0.1.1)
//   npm run release -- minor           -> 0.1.0 -> 0.2.0
//   npm run release -- 1.0.0           -> exact version
//   npm run release -- --notes "..."   -> release notes (or --notes-file CHANGES.md)
//   npm run release -- --local         -> build and publish only this computer's installer
//
// Needs the GitHub CLI signed in as the repository owner (`gh auth login`).

import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

// Must match UPDATE_REPO in src-tauri/src/update.rs.
const REPO = "jinwandalaohu159/Project-Hub";
const OWNER = REPO.split("/")[0];
// Builds every platform's installer; see .github/workflows/release.yml.
const WORKFLOW = "release.yml";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const files = {
  pkg: path.join(root, "package.json"),
  conf: path.join(root, "src-tauri", "tauri.conf.json"),
  cargo: path.join(root, "src-tauri", "Cargo.toml"),
};

function fail(message) {
  console.error(`\n✖ ${message}\n`);
  process.exit(1);
}

function step(message) {
  console.log(`\n▶ ${message}`);
}

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

// npx is a .cmd shim on Windows and needs a shell; everything else runs directly so arguments
// with spaces (titles, paths) stay intact.
function run(cmd, args) {
  const result = spawnSync(cmd, args, { cwd: root, stdio: "inherit", shell: process.platform === "win32" && cmd === "npx" });
  return result.status === 0;
}

function capture(cmd, args) {
  try {
    return execFileSync(cmd, args, { cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  } catch {
    return null;
  }
}

function parseArgs(argv) {
  const options = { bump: "patch", notes: "", local: false };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--notes") options.notes = argv[++i] ?? "";
    else if (arg === "--notes-file") options.notes = fs.readFileSync(path.resolve(argv[++i]), "utf8");
    else if (arg === "--local") options.local = true;
    else if (!arg.startsWith("-")) options.bump = arg;
  }
  return options;
}

function nextVersion(current, bump) {
  if (/^\d+\.\d+\.\d+$/.test(bump)) return bump;
  const [major, minor, patch] = current.split(".").map(Number);
  if (bump === "major") return `${major + 1}.0.0`;
  if (bump === "minor") return `${major}.${minor + 1}.0`;
  if (bump === "patch") return `${major}.${minor}.${patch + 1}`;
  fail(`Unknown version "${bump}". Use patch, minor, major or x.y.z.`);
}

function newerThan(a, b) {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) if (pa[i] !== pb[i]) return pa[i] > pb[i];
  return false;
}

function setVersion(version) {
  const pkg = JSON.parse(fs.readFileSync(files.pkg, "utf8"));
  pkg.version = version;
  fs.writeFileSync(files.pkg, `${JSON.stringify(pkg, null, 2)}\n`);

  const conf = JSON.parse(fs.readFileSync(files.conf, "utf8"));
  conf.version = version;
  fs.writeFileSync(files.conf, `${JSON.stringify(conf, null, 2)}\n`);

  const cargo = fs.readFileSync(files.cargo, "utf8");
  const updated = cargo.replace(/(\[package\][^[]*?\nversion\s*=\s*")[^"]+(")/, `$1${version}$2`);
  if (updated === cargo && !cargo.includes(`version = "${version}"`)) fail("Could not update the version in Cargo.toml");
  fs.writeFileSync(files.cargo, updated);
}

function bundleSpec() {
  const arch = process.arch === "arm64" ? "aarch64" : "x64";
  if (process.platform === "win32") return { bundle: "nsis", dir: "nsis", match: (f) => f.endsWith("-setup.exe"), suffix: `${arch}-setup.exe` };
  if (process.platform === "darwin") return { bundle: "dmg", dir: "dmg", match: (f) => f.endsWith(".dmg"), suffix: `${arch}.dmg` };
  return { bundle: "appimage", dir: "appimage", match: (f) => f.endsWith(".AppImage"), suffix: `${arch === "x64" ? "amd64" : arch}.AppImage` };
}

const options = parseArgs(process.argv.slice(2));

step("Checking GitHub");
if (!capture("gh", ["--version"])) fail("GitHub CLI (gh) is not installed: https://cli.github.com/");
const login = capture("gh", ["api", "user", "--jq", ".login"]);
if (!login) fail("GitHub CLI is not signed in. Run: gh auth login");
if (login.toLowerCase() !== OWNER.toLowerCase()) fail(`GitHub CLI is signed in as ${login}, but releases go to ${REPO}. Run: gh auth switch`);
const repoInfo = capture("gh", ["repo", "view", REPO, "--json", "visibility,isEmpty"]);
if (!repoInfo) fail(`Repository ${REPO} does not exist yet. Add this folder in Project Hub and upload it as a public repository named "${REPO.split("/")[1]}" first.`);
const repo = JSON.parse(repoInfo);
if (repo.isEmpty) fail(`Repository ${REPO} is empty. Upload the source with Project Hub's "Upload" button first.`);
if (repo.visibility !== "PUBLIC") fail(`Repository ${REPO} is ${repo.visibility.toLowerCase()}. Other computers can only download updates from a public repository.`);

const current = JSON.parse(fs.readFileSync(files.conf, "utf8")).version;
const version = nextVersion(current, options.bump);
// Re-running with the current version retries a release whose upload failed.
if (!newerThan(version, current) && version !== current) fail(`${version} is not newer than the current ${current}.`);
const tag = `v${version}`;
if (capture("gh", ["release", "view", tag, "--repo", REPO, "--json", "tagName"])) {
  fail(
    `Release ${tag} already exists on GitHub.\n  To build its installers again: gh workflow run ${WORKFLOW} --repo ${REPO} -f tag=${tag}`,
  );
}

// Building every platform means handing the tag to GitHub Actions: macOS and Linux installers
// cannot be produced on Windows, and vice versa.
if (!options.local) {
  const branch = capture("git", ["rev-parse", "--abbrev-ref", "HEAD"]);
  if (!branch || branch === "HEAD") fail("Not on a branch. Check out main first.");
  if (capture("git", ["status", "--porcelain"])) fail("There are uncommitted changes. Commit or stash them first, so the release commit only contains the version bump.");
  if (capture("git", ["tag", "-l", tag])) fail(`Tag ${tag} already exists locally. Remove it with: git tag -d ${tag}`);

  step(`Version ${current} → ${version}`);
  setVersion(version);

  const notes = options.notes.trim() || `Project Hub ${tag}`;
  step(`Pushing ${tag} to ${REPO}`);
  const committed =
    run("git", ["add", "package.json", "src-tauri/tauri.conf.json", "src-tauri/Cargo.toml"]) &&
    run("git", ["commit", "-m", `Project Hub ${version}`]) &&
    run("git", ["tag", "-a", tag, "-m", notes]);
  if (!committed) {
    setVersion(current);
    fail(`Could not create the release commit; version restored to ${current}.`);
  }
  if (!run("git", ["push", "origin", branch, "--follow-tags"])) {
    fail(`Push failed. The commit and tag ${tag} are ready locally; retry with: git push origin ${branch} --follow-tags`);
  }

  step("GitHub Actions is building the Windows, macOS and Linux installers (about 10-15 minutes)");
  console.log(`  https://github.com/${REPO}/actions/workflows/${WORKFLOW}`);
  let runId = null;
  for (let i = 0; i < 20 && !runId; i++) {
    sleep(3000);
    const list = capture("gh", ["run", "list", "--repo", REPO, "--workflow", WORKFLOW, "--limit", "10", "--json", "databaseId,headBranch,event"]);
    runId = JSON.parse(list || "[]").find((item) => item.headBranch === tag)?.databaseId ?? null;
  }
  if (!runId) {
    console.log("\n  The build has not shown up yet; follow it on the page above.\n");
    process.exit(0);
  }
  if (!run("gh", ["run", "watch", String(runId), "--repo", REPO, "--exit-status"])) {
    fail(`The build failed. See https://github.com/${REPO}/actions/runs/${runId} , then rerun it with: gh workflow run ${WORKFLOW} --repo ${REPO} -f tag=${tag}`);
  }
  console.log(`\n✔ Released ${tag} for Windows, macOS and Linux. Installed copies will offer the update on their next check.\n`);
  process.exit(0);
}

step(`Version ${current} → ${version}`);
setVersion(version);

const spec = bundleSpec();
step(`Building the ${spec.bundle} installer for this computer only (this takes a few minutes)`);
if (!run("npx", ["tauri", "build", "--bundles", spec.bundle])) {
  setVersion(current);
  fail(`Build failed; version restored to ${current}.`);
}

const bundleDir = path.join(root, "src-tauri", "target", "release", "bundle", spec.dir);
const built = fs.readdirSync(bundleDir).filter(spec.match).filter((f) => f.includes(version));
if (built.length === 0) fail(`No installer for ${version} found in ${bundleDir}`);

// GitHub turns spaces into dots in file names; use a stable name the app can recognise.
const outDir = path.join(root, "release");
fs.mkdirSync(outDir, { recursive: true });
const assetName = `Project-Hub_${version}_${spec.suffix}`;
const assetPath = path.join(outDir, assetName);
fs.copyFileSync(path.join(bundleDir, built[0]), assetPath);

const notes = options.notes.trim() || `Project Hub ${tag}`;
const notesFile = path.join(os.tmpdir(), `project-hub-notes-${version}.md`);
fs.writeFileSync(notesFile, notes);

step(`Publishing ${tag} to https://github.com/${REPO}/releases`);
const published = run("gh", ["release", "create", tag, assetPath, "--repo", REPO, "--title", `Project Hub ${tag}`, "--notes-file", notesFile, "--latest"]);
fs.rmSync(notesFile, { force: true });
if (!published) fail(`Upload failed. The installer is at ${assetPath}; rerun "npm run release -- ${version} --local" to retry.`);

console.log(`\n✔ Released ${tag} (${process.platform} only). Installed copies will offer the update on their next check.\n`);
