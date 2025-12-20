import { spawn } from "node:child_process";

import readline from "node:readline";

import { SandboxMode } from "./threadOptions";
import path from "node:path";
import { fileURLToPath } from "node:url";

export type CodexExecArgs = {
  input: string;

  baseUrl?: string;
  apiKey?: string;
  threadId?: string | null;
  // --model
  model?: string;
  // --sandbox
  sandboxMode?: SandboxMode;
  // --cd
  workingDirectory?: string;
  // --skip-git-repo-check
  skipGitRepoCheck?: boolean;
};

export class CodexExec {
  private executablePath: string;
  constructor(executablePath: string | null = null) {
    this.executablePath = executablePath || findCodexPath();
  }

  async *run(args: CodexExecArgs): AsyncGenerator<string> {
    const commandArgs: string[] = ["exec", "--json"];

    if (args.model) {
      commandArgs.push("--model", args.model);
    }

    if (args.sandboxMode) {
      commandArgs.push("--sandbox", args.sandboxMode);
    }

    if (args.workingDirectory) {
      commandArgs.push("--cd", args.workingDirectory);
    }

    if (args.skipGitRepoCheck) {
      commandArgs.push("--skip-git-repo-check");
    }

    if (args.threadId) {
      commandArgs.push("resume", args.threadId);
    }

    const env = {
      ...process.env,
    };
    if (args.baseUrl) {
      env.OPENAI_BASE_URL = args.baseUrl;
    }
    if (args.apiKey) {
      env.CODEX_API_KEY = args.apiKey;
    }

    const child = spawn(this.executablePath, commandArgs, {
      env,
    });

    const stderrChunks: Buffer[] = [];
    if (child.stderr) {
      child.stderr.on("data", (data) => {
        stderrChunks.push(data);
      });
    }

    const exitPromise = new Promise<void>((resolve, reject) => {
      child.once("error", reject);
      child.once("exit", (code, signal) => {
        if (signal) {
          reject(new Error(`Beacon Code exec terminated by signal ${signal}`));
          return;
        }
        if (code === 0) {
          resolve();
          return;
        }
        const stderrBuffer = Buffer.concat(stderrChunks);
        reject(
          new Error(
            `Beacon Code exec exited with code ${code}: ${stderrBuffer.toString("utf8")}`,
          ),
        );
      });
    });
    void exitPromise.catch(() => {});

    if (!child.stdin) {
      child.kill();
      throw new Error("Child process has no stdin");
    }
    child.stdin.write(args.input);
    child.stdin.end();

    if (!child.stdout) {
      child.kill();
      throw new Error("Child process has no stdout");
    }

    const rl = readline.createInterface({
      input: child.stdout,
      crlfDelay: Infinity,
    });

    try {
      for await (const line of rl) {
        // `line` is a string (Node sets default encoding to utf8 for readline)
        yield line as string;
      }
      await exitPromise;
    } finally {
      rl.close();
      child.removeAllListeners();
      child.stderr?.removeAllListeners();
      try {
        if (!child.killed) child.kill();
      } catch {
        // ignore
      }
    }
  }
}

const scriptFileName = fileURLToPath(import.meta.url);
const scriptDirName = path.dirname(scriptFileName);

function findCodexPath() {
  const { platform, arch } = process;

  let targetTriple = null;
  switch (platform) {
    case "linux":
    case "android":
      switch (arch) {
        case "x64":
          targetTriple = "x86_64-unknown-linux-musl";
          break;
        case "arm64":
          targetTriple = "aarch64-unknown-linux-musl";
          break;
        default:
          break;
      }
      break;
    case "darwin":
      switch (arch) {
        case "x64":
          targetTriple = "x86_64-apple-darwin";
          break;
        case "arm64":
          targetTriple = "aarch64-apple-darwin";
          break;
        default:
          break;
      }
      break;
    case "win32":
      switch (arch) {
        case "x64":
          targetTriple = "x86_64-pc-windows-msvc";
          break;
        case "arm64":
          targetTriple = "aarch64-pc-windows-msvc";
          break;
        default:
          break;
      }
      break;
    default:
      break;
  }

  if (!targetTriple) {
    throw new Error(`Unsupported platform: ${platform} (${arch})`);
  }

  const vendorRoot = path.join(scriptDirName, "..", "vendor");
  const archRoot = path.join(vendorRoot, targetTriple);
  const codexBinaryName = process.platform === "win32" ? "codex.exe" : "codex";
  const binaryPath = path.join(archRoot, "codex", codexBinaryName);

  return binaryPath;
}
