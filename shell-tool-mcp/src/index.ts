// Launches the Beacon Code exec MCP server binary bundled in this package.

import { spawn } from "node:child_process";
import { accessSync, constants } from "node:fs";
import os from "node:os";
import path from "node:path";
import { resolveBashPath } from "./bashSelection";
import { readOsRelease } from "./osRelease";
import { resolveTargetTriple } from "./platform";

async function main(): Promise<void> {
  const targetTriple = resolveTargetTriple(process.platform, process.arch);
  const vendorRoot = path.resolve(__dirname, "..", "vendor");
  const targetRoot = path.join(vendorRoot, targetTriple);
  const execveWrapperPath = pickFirstExisting(
    [
      path.join(targetRoot, "code-execve-wrapper"),
      path.join(targetRoot, "codex-execve-wrapper"),
    ],
    "execve wrapper",
  );
  const serverPath = pickFirstExisting(
    [
      path.join(targetRoot, "code-exec-mcp-server"),
      path.join(targetRoot, "codex-exec-mcp-server"),
    ],
    "exec MCP server",
  );

  const osInfo = process.platform === "linux" ? readOsRelease() : null;
  const { path: bashPath } = resolveBashPath(
    targetRoot,
    process.platform,
    os.release(),
    osInfo,
  );

  [execveWrapperPath, serverPath, bashPath].forEach((checkPath) => {
    try {
      accessSync(checkPath, constants.F_OK);
    } catch {
      throw new Error(`Required binary missing: ${checkPath}`);
    }
  });

  const args = [
    "--execve",
    execveWrapperPath,
    "--bash",
    bashPath,
    ...process.argv.slice(2),
  ];
  const child = spawn(serverPath, args, {
    stdio: "inherit",
  });

  const forwardSignal = (signal: NodeJS.Signals) => {
    if (child.killed) {
      return;
    }
    try {
      child.kill(signal);
    } catch {
      /* ignore */
    }
  };

  (["SIGINT", "SIGTERM", "SIGHUP"] as const).forEach((sig) => {
    process.on(sig, () => forwardSignal(sig));
  });

  child.on("error", (err) => {
    // eslint-disable-next-line no-console
    console.error(err);
    process.exit(1);
  });

  const childResult = await new Promise<
    | { type: "signal"; signal: NodeJS.Signals }
    | { type: "code"; exitCode: number }
  >((resolve) => {
    child.on("exit", (code, signal) => {
      if (signal) {
        resolve({ type: "signal", signal });
      } else {
        resolve({ type: "code", exitCode: code ?? 1 });
      }
    });
  });

  if (childResult.type === "signal") {
    // This environment running under `node --test` may not allow rethrowing a signal.
    // Wrap in a try to avoid masking the original termination reason.
    try {
      process.kill(process.pid, childResult.signal);
    } catch {
      process.exit(1);
    }
  } else {
    process.exit(childResult.exitCode);
  }
}

function pickFirstExisting(paths: string[], label: string): string {
  for (const candidate of paths) {
    try {
      accessSync(candidate, constants.F_OK);
      return candidate;
    } catch {
      // try next
    }
  }
  throw new Error(`Required ${label} binary missing. Tried: ${paths.join(", ")}`);
}

void main().catch((err) => {
  // eslint-disable-next-line no-console
  console.error(err);
  process.exit(1);
});
