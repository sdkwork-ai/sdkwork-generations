// Route contract gate for the generations App API.
//
// `crates/sdkwork-routes-generations-http-shared/build.rs` derives `APP_ROUTES`
// from the OpenAPI authority, and the web framework rejects any request whose
// `method + path` is absent from that table with
// `route is not registered in the gateway route manifest` (404). A route that
// `build_app_routes()` registers but the OpenAPI authority never declares is
// therefore *registered yet unreachable* — it looks wired in the source and
// 404s in production. That is exactly how `/videos/avatar` and
// `/videos/motion_mimicry` shipped broken once.
//
// This gate makes the two sets equal in both directions:
//   - `build_app_routes()` every route must be declared in the OpenAPI authority
//   - every declared operation must resolve to a registered handler
//
// Usage:
//   node tools/check_generations_route_contract.mjs [--root <repo>]
//
// Exits 1 on drift, printing both sides so the fix is unambiguous.

import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));

const APP_OPENAPI = path.join(
  "sdks/sdkwork-generations-app-sdk/openapi/sdkwork-generations-app-api.openapi.json",
);
const HANDLERS =
  "crates/sdkwork-intelligence-generations-service/src/service/handlers.rs";
const APP_ROUTE_PREFIX = "/app/v3/api";

function parseArgs(argv) {
  let root = path.resolve(scriptDir, "..");
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--root") {
      root = path.resolve(argv[index + 1]);
      index += 1;
    }
  }
  return { root };
}

function normalizePath(value) {
  const trimmed = value.trim();
  return trimmed.startsWith("/") ? trimmed : `/${trimmed}`;
}

/** Collects `method + path` pairs declared by the OpenAPI authority. */
function collectDeclaredOperations(document) {
  const declared = new Map();
  const paths = document?.paths ?? {};
  for (const [rawPath, operations] of Object.entries(paths)) {
    const routePath = normalizePath(rawPath);
    if (!routePath.startsWith(APP_ROUTE_PREFIX)) {
      continue;
    }
    for (const [method, operation] of Object.entries(operations)) {
      const verb = method.toLowerCase();
      if (!["get", "post", "put", "patch", "delete"].includes(verb)) {
        continue;
      }
      declared.set(`${verb.toUpperCase()} ${routePath}`, {
        operationId: operation?.operationId ?? "",
      });
    }
  }
  return declared;
}

/** Collects `method + path + handler` triples registered by `build_app_routes`. */
function collectRegisteredRoutes(source) {
  const start = source.indexOf("pub fn build_app_routes");
  if (start < 0) {
    throw new Error(
      "build_app_routes is missing from the generations handlers module",
    );
  }
  const end = source.indexOf("\npub fn ", start + 1);
  const body = end < 0 ? source.slice(start) : source.slice(start, end);

  const registered = new Map();
  const pattern =
    /\.route\(\s*"([^"]+)"\s*,\s*axum::routing::(get|post|put|patch|delete)\(\s*([A-Za-z0-9_]+)\s*\)\s*,?\s*\)/g;
  for (const match of body.matchAll(pattern)) {
    const routePath = normalizePath(match[1]);
    registered.set(`${match[2].toUpperCase()} ${routePath}`, {
      handler: match[3],
    });
  }
  return { registered, body };
}

async function main() {
  const { root } = parseArgs(process.argv.slice(2));

  const openApiPath = path.join(root, APP_OPENAPI);
  const handlersPath = path.join(root, HANDLERS);

  const declared = collectDeclaredOperations(
    JSON.parse(await readFile(openApiPath, "utf8")),
  );
  const source = await readFile(handlersPath, "utf8");
  const { registered, body } = collectRegisteredRoutes(source);

  const problems = [];

  for (const [key, entry] of registered) {
    if (!declared.has(key)) {
      problems.push(
        `${key} is registered by build_app_routes (handler \`${entry.handler}\`) but is not declared in ${APP_OPENAPI}; ` +
          "APP_ROUTES is generated from that authority, so the framework answers 404 " +
          "`route is not registered in the gateway route manifest` before the handler runs.",
      );
    }
    if (!body.includes(`async fn ${entry.handler}(`)) {
      problems.push(
        `${key} references handler \`${entry.handler}\`, which has no \`async fn ${entry.handler}(\` definition.`,
      );
    }
  }

  for (const [key, entry] of declared) {
    if (!registered.has(key)) {
      problems.push(
        `${key} is declared in ${APP_OPENAPI} (operationId \`${entry.operationId}\`) ` +
          "but build_app_routes never registers it; the generated APP_ROUTES entry can never reach a handler.",
      );
    }
    if (!entry.operationId) {
      problems.push(`${key} is missing operationId in ${APP_OPENAPI}.`);
    }
  }

  if (problems.length > 0) {
    console.error("generations-route-contract: FAILED");
    for (const problem of problems) {
      console.error(`  - ${problem}`);
    }
    process.exit(1);
  }

  console.log(
    `generations-route-contract: ok (${declared.size} operations declared = ${registered.size} routes registered)`,
  );
}

main().catch((error) => {
  console.error(`generations-route-contract: FAILED\n  - ${error.message}`);
  process.exit(1);
});
