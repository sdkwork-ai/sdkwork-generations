import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const testDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(testDir, "..");
const workspaceRoot = path.resolve(repoRoot, "..");
const appRoot = path.join(repoRoot, "apps", "sdkwork-generations-pc");
const migratedPackageRoot = path.join(
  appRoot,
  "packages",
  "sdkwork-generations-pc-workspace",
);
const oldAppbasePackageRoot = path.join(
  workspaceRoot,
  "sdkwork-appbase",
  "packages",
  "pc-react",
  "content",
  "sdkwork-generation-pc-react",
);

function readJson(absolutePath) {
  assert.ok(existsSync(absolutePath), `${path.relative(repoRoot, absolutePath)} must exist`);
  return JSON.parse(readFileSync(absolutePath, "utf8"));
}

function readText(absolutePath) {
  assert.ok(existsSync(absolutePath), `${path.relative(repoRoot, absolutePath)} must exist`);
  return readFileSync(absolutePath, "utf8");
}

test("generations PC app root follows the SDKWork PC application shape", () => {
  assert.ok(existsSync(appRoot), "apps/sdkwork-generations-pc must exist");

  for (const relativePath of [
    "AGENTS.md",
    "CODEX.md",
    ".sdkwork/README.md",
    ".sdkwork/skills/README.md",
    ".sdkwork/plugins/README.md",
    "agent.app.config.json",
    "package.json",
    "pnpm-workspace.yaml",
    "src/App.tsx",
    "src/main.tsx",
    "src/bootstrap/routes.ts",
    "src/bootstrap/sdkClients.ts",
    "specs/README.md",
    "specs/component.spec.json",
  ]) {
    assert.ok(existsSync(path.join(appRoot, relativePath)), `${relativePath} must exist`);
  }

  const appManifest = readJson(path.join(appRoot, "agent.app.config.json"));
  assert.equal(appManifest.schemaVersion, 3);
  assert.equal(appManifest.kind, "agent.app");
  assert.equal(appManifest.app?.key, "sdkwork-generations-pc");

  const componentSpec = readJson(path.join(appRoot, "specs", "component.spec.json"));
  assert.equal(componentSpec.component.name, "sdkwork-generations-pc");
  assert.equal(componentSpec.component.type, "pc-app");
  assert.equal(componentSpec.component.root, "sdkwork-generations/apps/sdkwork-generations-pc");
});

test("generation workspace package is an app-side PC module, not console or admin", () => {
  assert.ok(existsSync(migratedPackageRoot), "workspace package must exist in the PC app packages directory");

  const packageJson = readJson(path.join(migratedPackageRoot, "package.json"));
  assert.equal(packageJson.name, "@sdkwork/generations-pc-workspace");
  assert.equal(packageJson.sdkwork?.workspace, "sdkwork-generations-pc");
  assert.equal(packageJson.sdkwork?.architecture, "pc-react");
  assert.equal(packageJson.sdkwork?.surface, "app");
  assert.equal(packageJson.sdkwork?.domain, "intelligence");
  assert.equal(packageJson.sdkwork?.capability, "workspace");

  assert.ok(
    !packageJson.name.includes("-pc-console-") && !packageJson.name.includes("-pc-admin-"),
    "migrated package must remain a user-facing app package",
  );

  const packageSpec = readJson(path.join(migratedPackageRoot, "specs", "component.spec.json"));
  assert.equal(packageSpec.component.name, "@sdkwork/generations-pc-workspace");
  assert.equal(
    packageSpec.component.root,
    "sdkwork-generations/apps/sdkwork-generations-pc/packages/sdkwork-generations-pc-workspace",
  );
  assert.equal(packageSpec.component.domain, "intelligence");
  assert.equal(packageSpec.component.capability, "workspace");

  const packageReadme = readText(path.join(migratedPackageRoot, "README.md"));
  assert.match(packageReadme, /Surface: app/u);
  assert.match(packageReadme, /sdkwork-generations-app-sdk/u);
});

test("appbase no longer retains the migrated generation PC React package", () => {
  assert.ok(
    !existsSync(oldAppbasePackageRoot),
    "sdkwork-appbase must not retain packages/pc-react/content/sdkwork-generation-pc-react",
  );
});
