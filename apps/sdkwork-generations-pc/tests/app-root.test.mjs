import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const testDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(testDir, "..");

function readJson(relativePath) {
  return JSON.parse(readFileSync(path.join(appRoot, relativePath), "utf8"));
}

function readText(relativePath) {
  return readFileSync(path.join(appRoot, relativePath), "utf8");
}

test("sdkwork-generations-pc composes the user-facing workspace package", () => {
  const appManifest = readJson("agent.app.config.json");
  const appPackage = readJson("package.json");
  const workspacePackage = readJson("packages/sdkwork-generations-pc-workspace/package.json");

  assert.equal(appManifest.app.key, "sdkwork-generations-pc");
  assert.equal(appPackage.dependencies["@sdkwork/generations-pc-workspace"], "workspace:*");
  assert.equal(workspacePackage.name, "@sdkwork/generations-pc-workspace");
  assert.equal(workspacePackage.sdkwork.surface, "app");
  assert.equal(workspacePackage.sdkwork.domain, "intelligence");
  assert.equal(workspacePackage.sdkwork.capability, "workspace");
});

test("sdkwork-generations-pc declares Drive SDK composition dependencies", () => {
  const appPackage = readJson("package.json");
  const componentSpec = readJson("specs/component.spec.json");
  const workspacePackage = readJson("packages/sdkwork-generations-pc-workspace/package.json");
  const workspaceYaml = readText("pnpm-workspace.yaml");

  assert.equal(
    appPackage.dependencies["@sdkwork/generations-app-sdk"],
    "workspace:*",
  );
  assert.equal(appPackage.dependencies["@sdkwork/drive-app-sdk"], "workspace:*");
  assert.equal(appPackage.dependencies["@sdkwork/sdk-common"], "^1.0.3");
  assert.equal(appPackage.dependencies["@sdkwork/ui-pc-react"], "workspace:*");
  assert.equal(workspacePackage.dependencies["@sdkwork/ui-pc-react"], "workspace:*");
  assert.equal(workspacePackage.peerDependencies?.["@sdkwork/ui-pc-react"], undefined);
  assert.ok(
    componentSpec.contracts.sdkClients.includes("sdkwork-generations-app-sdk"),
    "component spec must declare the product app SDK client",
  );
  assert.ok(
    componentSpec.contracts.sdkClients.includes("sdkwork-drive-app-sdk"),
    "component spec must declare the Drive app SDK dependency",
  );
  assert.match(
    workspaceYaml,
    /sdkwork-drive\/sdks\/sdkwork-drive-app-sdk\/sdkwork-drive-app-sdk-typescript/u,
  );
  assert.doesNotMatch(
    workspaceYaml,
    /sdkwork-generations-app-sdk\/sdkwork-generations-app-sdk-typescript\/generated\/server-openapi/u,
  );
  assert.match(
    workspaceYaml,
    /sdkwork-iam\/sdks\/sdkwork-iam-app-sdk\/sdkwork-iam-app-sdk-typescript/u,
  );
  assert.doesNotMatch(
    workspaceYaml,
    /sdkwork-iam\/sdks\/sdkwork-iam-app-sdk\/sdkwork-iam-app-sdk-typescript\/generated\/server-openapi/u,
  );
  assert.match(workspaceYaml, /sdkwork-ui\/sdkwork-ui-pc-react/u);
});
