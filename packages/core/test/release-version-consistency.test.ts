/**
 * GRD-DEVOPS-003: Shared package version, matching core pins, tag vX.Y.Z, and versioned artifact names.
 */
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const REPO_ROOT = path.resolve(__dirname, "../../..");
const BUMP_SCRIPT = path.join(REPO_ROOT, "scripts", "bump-version.py");
const ASSERT_SCRIPT = path.join(REPO_ROOT, "scripts", "assert-release-tag.py");
const CLI_RELEASE_WORKFLOW = path.join(REPO_ROOT, ".github", "workflows", "release-cli.yml");
const VSCODE_RELEASE_WORKFLOW = path.join(REPO_ROOT, ".github", "workflows", "release-vscode.yml");
const RELEASE_GUIDE = path.join(REPO_ROOT, "release.md");
const README = path.join(REPO_ROOT, "README.md");
const CORE_PKG = path.join(REPO_ROOT, "packages", "core", "package.json");
const VSCODE_PKG = path.join(REPO_ROOT, "packages", "vscode", "package.json");
const CARGO_TOML = path.join(REPO_ROOT, "Cargo.toml");

function readJson(filePath: string): Record<string, unknown> {
  return JSON.parse(fs.readFileSync(filePath, "utf-8")) as Record<string, unknown>;
}

function sharedPackageVersion(): string {
  const version = readJson(CORE_PKG).version;
  if (typeof version !== "string" || !version) {
    throw new Error("packages/core/package.json is missing version");
  }
  return version;
}

function makeTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "shallgraph-grd-devops-003-"));
}

function writeJson(filePath: string, data: unknown): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(data, null, 2)}\n`);
}

function fixturePackageJson(name: string, version: string, withCoreDep: boolean): Record<string, unknown> {
  const pkg: Record<string, unknown> = {
    name,
    version,
    description: "shallgraph — fixture",
  };
  if (withCoreDep) {
    pkg.dependencies = { "@shallgraph/core": version };
  }
  return pkg;
}

function seedVersionTree(tmpDir: string, version: string): void {
  fs.mkdirSync(path.join(tmpDir, "scripts"), { recursive: true });
  fs.copyFileSync(BUMP_SCRIPT, path.join(tmpDir, "scripts", "bump-version.py"));
  fs.copyFileSync(ASSERT_SCRIPT, path.join(tmpDir, "scripts", "assert-release-tag.py"));
  fs.chmodSync(path.join(tmpDir, "scripts", "bump-version.py"), 0o755);
  fs.chmodSync(path.join(tmpDir, "scripts", "assert-release-tag.py"), 0o755);

  writeJson(path.join(tmpDir, "packages", "core", "package.json"), fixturePackageJson("@shallgraph/core", version, false));
  writeJson(
    path.join(tmpDir, "packages", "vscode", "package.json"),
    fixturePackageJson("shallgraph-vscode", version, true)
  );
  writeJson(path.join(tmpDir, "package-lock.json"), {
    name: "shallgraph",
    lockfileVersion: 3,
    packages: {
      "packages/core": { name: "@shallgraph/core", version },
      "packages/vscode": { name: "shallgraph-vscode", version, dependencies: { "@shallgraph/core": version } },
    },
  });
  fs.writeFileSync(
    path.join(tmpDir, "Cargo.toml"),
    `[workspace]\nmembers = []\n\n[workspace.package]\nedition = "2021"\nversion = "${version}"\n`
  );
  fs.writeFileSync(
    path.join(tmpDir, "Cargo.lock"),
    `[[package]]\nname = "shallgraph"\nversion = "${version}"\n\n[[package]]\nname = "shallgraph-core"\nversion = "${version}"\n\n[[package]]\nname = "shallgraph-macros"\nversion = "${version}"\n\n[[package]]\nname = "shallgraph-wasm"\nversion = "${version}"\n`
  );
  fs.writeFileSync(
    path.join(tmpDir, "README.md"),
    [
      `Download shallgraph-core-${version}.tgz from`,
      `https://github.com/example/shallgraph/releases/download/v${version}/shallgraph-core-${version}.tgz`,
      "",
    ].join("\n")
  );
  fs.mkdirSync(path.join(tmpDir, "packages", "vscode"), { recursive: true });
  fs.writeFileSync(
    path.join(tmpDir, "packages", "vscode", "README.md"),
    `This produces shallgraph-vscode-${version}.vsix.\n`
  );
}

describe("GRD-DEVOPS-003: shared release version", () => {
  it("workspace packages, @shallgraph/core pins, and Cargo workspace version match", () => {
    const version = sharedPackageVersion();
    expect(readJson(VSCODE_PKG).version).toBe(version);
    const vscodeDeps = readJson(VSCODE_PKG).dependencies as Record<string, string>;
    expect(vscodeDeps["@shallgraph/core"]).toBe(version);

    const cargo = fs.readFileSync(CARGO_TOML, "utf-8");
    const match = cargo.match(/\[workspace\.package\][^\[]*?version\s*=\s*"([^"]+)"/s);
    expect(match).not.toBeNull();
    expect(match?.[1]).toBe(version);
  });

  it("README documents cargo install and the native Linux binary", () => {
    const readme = fs.readFileSync(README, "utf-8");
    expect(readme).toContain("cargo install --path crates/shallgraph");
    expect(readme).toContain("shallgraph-linux-x86_64");
    expect(readme).not.toMatch(/shallgraph-\d+\.\d+\.\d+\.tgz/);
  });

  it("bump-version.py updates packages, lockfiles, and README install URLs", () => {
    const tmpDir = makeTempDir();
    seedVersionTree(tmpDir, "0.1.0");
    execFileSync("python3", [path.join(tmpDir, "scripts", "bump-version.py"), "9.8.7"], {
      cwd: tmpDir,
      stdio: "pipe",
    });

    expect(readJson(path.join(tmpDir, "packages", "core", "package.json")).version).toBe("9.8.7");
    const corePkgText = fs.readFileSync(path.join(tmpDir, "packages", "core", "package.json"), "utf-8");
    expect(corePkgText).toContain("shallgraph — fixture");
    expect(corePkgText).not.toContain("\\u2014");
    expect(readJson(path.join(tmpDir, "packages", "vscode", "package.json")).version).toBe("9.8.7");
    const lock = readJson(path.join(tmpDir, "package-lock.json"));
    const packages = lock.packages as Record<string, { version?: string; dependencies?: Record<string, string> }>;
    expect(packages["packages/core"].version).toBe("9.8.7");
    expect(packages["packages/vscode"].dependencies?.["@shallgraph/core"]).toBe("9.8.7");
    expect(fs.readFileSync(path.join(tmpDir, "Cargo.toml"), "utf-8")).toContain('version = "9.8.7"');
    expect(fs.readFileSync(path.join(tmpDir, "Cargo.lock"), "utf-8")).toContain('name = "shallgraph"\nversion = "9.8.7"');
    expect(fs.readFileSync(path.join(tmpDir, "Cargo.lock"), "utf-8")).toContain(
      'name = "shallgraph-core"\nversion = "9.8.7"'
    );
    expect(fs.readFileSync(path.join(tmpDir, "Cargo.lock"), "utf-8")).toContain(
      'name = "shallgraph-macros"\nversion = "9.8.7"'
    );
    expect(fs.readFileSync(path.join(tmpDir, "Cargo.lock"), "utf-8")).toContain(
      'name = "shallgraph-wasm"\nversion = "9.8.7"'
    );
    const readme = fs.readFileSync(path.join(tmpDir, "README.md"), "utf-8");
    expect(readme).toContain("/releases/download/v9.8.7/shallgraph-core-9.8.7.tgz");
    expect(readme).not.toContain("0.1.0");
    expect(fs.readFileSync(path.join(tmpDir, "packages", "vscode", "README.md"), "utf-8")).toContain(
      "shallgraph-vscode-9.8.7.vsix"
    );
  });

  it("bump-version.py rewrites README tags that already disagree with package.json", () => {
    const tmpDir = makeTempDir();
    seedVersionTree(tmpDir, "0.1.0");
    fs.writeFileSync(
      path.join(tmpDir, "README.md"),
      [
        'Download "https://github.com/example/shallgraph/releases/download/v0.2.0/shallgraph-core-0.1.0.tgz"',
        "",
      ].join("\n")
    );
    execFileSync("python3", [path.join(tmpDir, "scripts", "bump-version.py"), "9.8.7"], {
      cwd: tmpDir,
      stdio: "pipe",
    });
    const readme = fs.readFileSync(path.join(tmpDir, "README.md"), "utf-8");
    expect(readme).toContain("/releases/download/v9.8.7/shallgraph-core-9.8.7.tgz");
    expect(readme).not.toContain("v0.2.0");
    expect(readme).not.toContain("0.1.0");
  });

  it("assert-release-tag.py accepts a matching tag and required artifact names", () => {
    const tmpDir = makeTempDir();
    seedVersionTree(tmpDir, "1.2.3");
    fs.mkdirSync(path.join(tmpDir, "release"), { recursive: true });
    fs.writeFileSync(path.join(tmpDir, "release", "shallgraph-core-1.2.3.tgz"), "");
    const out = execFileSync("python3", [path.join(tmpDir, "scripts", "assert-release-tag.py"), "v1.2.3", "cli"], {
      cwd: tmpDir,
      encoding: "utf-8",
      stdio: ["pipe", "pipe", "pipe"],
    });
    expect(out).toContain("v1.2.3");
    expect(out).toContain("1.2.3");
  });

  it("assert-release-tag.py rejects a tag that does not match the package version", () => {
    const tmpDir = makeTempDir();
    seedVersionTree(tmpDir, "1.2.3");
    expect(() =>
      execFileSync("python3", [path.join(tmpDir, "scripts", "assert-release-tag.py"), "v9.9.9"], {
        cwd: tmpDir,
        stdio: "pipe",
      })
    ).toThrow();
  });

  it("release workflows invoke assert-release-tag.py before upload", () => {
    const cliWorkflow = fs.readFileSync(CLI_RELEASE_WORKFLOW, "utf-8");
    expect(cliWorkflow).toContain("python3 ./scripts/assert-release-tag.py");
    expect(cliWorkflow).toContain('"${{ github.event.release.tag_name }}" cli');
    expect(cliWorkflow).toContain('"${{ github.event.release.tag_name }}" native');
    const vscodeWorkflow = fs.readFileSync(VSCODE_RELEASE_WORKFLOW, "utf-8");
    expect(vscodeWorkflow).toContain("python3 ./scripts/assert-release-tag.py");
    expect(vscodeWorkflow).toContain('"${{ github.event.release.tag_name }}" vscode');
    expect(vscodeWorkflow).toContain("github.event.release.prerelease");
    expect(vscodeWorkflow).toContain("--pre-release");
  });

  it("release guide documents the bump script as the version-update step", () => {
    const guide = fs.readFileSync(RELEASE_GUIDE, "utf-8");
    expect(guide).toContain("./scripts/bump-version.py X.Y.Z");
    expect(guide).toContain("git tag vX.Y.Z");
    expect(guide).toContain("Publish the GitHub Release");
  });
});
