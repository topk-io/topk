// usage: bun run utils/verify-sdk-docs-sync.ts check

import { program } from "commander";
import { spawn } from "child_process";
import fs from "fs/promises";
import path from "path";

const paths = {
  topkJs: {
    root: "./topk-js",
    docs: "./docs/sdk/topk-js",
    indexDts: "./topk-js/index.d.ts",
  },
  topkPy: {
    root: "./topk-py",
    docs: "./docs/sdk/topk-py",
    docgen: "./topk-py/docgen/main.py",
    topkSdk: "./topk-py/topk_sdk",
  },
};

// Utility function to run shell commands
async function runCommand(command: string, args: string[] = [], cwd?: string): Promise<{ success: boolean; output: string; error: string }> {
  return new Promise((resolve) => {
    const childProcess = spawn(command, args, {
      cwd: cwd || process.cwd(),
      shell: true,
      stdio: "pipe",
    });

    let output = "";
    let error = "";

    childProcess.stdout?.on("data", (data) => {
      output += data.toString();
    });

    childProcess.stderr?.on("data", (data) => {
      error += data.toString();
    });

    childProcess.on("close", (code) => {
      resolve({
        success: code === 0,
        output,
        error,
      });
    });

    childProcess.on("error", (err) => {
      resolve({
        success: false,
        output,
        error: err.message,
      });
    });
  });
}

// Check if files exist
async function checkPrerequisites() {
  const checks = [
    { path: paths.topkJs.indexDts, name: "topk-js index.d.ts" },
    { path: paths.topkPy.docgen, name: "topk-py docgen script" },
    { path: paths.topkPy.topkSdk, name: "topk-py topk_sdk directory" },
  ];

  for (const check of checks) {
    try {
      await fs.access(check.path);
    } catch {
      throw new Error(`Prerequisite missing: ${check.name} not found at ${check.path}`);
    }
  }
}

// Build topk-js documentation
async function buildTopkJSDocs(): Promise<{ success: boolean; error?: string }> {
  console.log("📚 Generating topk-js documentation with typedoc...");

  // Then run typedoc
  const typedocResult = await runCommand("yarn", ["run", "docs"], paths.topkJs.root);
  if (!typedocResult.success) {
    return {
      success: false,
      error: `Failed to generate topk-js docs with typedoc: ${typedocResult.error}`,
    };
  }

  return { success: true };
}

// Build topk-py documentation
async function buildTopkPyDocs(): Promise<{ success: boolean; error?: string }> {
  console.log("🐍 Generating topk-py documentation...");

  const result = await runCommand("python", [
    paths.topkPy.docgen,
    paths.topkPy.topkSdk,
    paths.topkPy.docs,
  ]);

  if (!result.success) {
    return {
      success: false,
      error: `Failed to generate topk-py docs: ${result.error}`,
    };
  }

  return { success: true };
}

// Get all files in a directory recursively
async function getAllFiles(dir: string): Promise<string[]> {
  const files: string[] = [];

  async function traverse(currentDir: string) {
    const entries = await fs.readdir(currentDir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = path.join(currentDir, entry.name);

      if (entry.isDirectory()) {
        await traverse(fullPath);
      } else {
        files.push(fullPath);
      }
    }
  }

  await traverse(dir);
  return files;
}

// Compare two directories for differences
async function compareDirectories(dir1: string, dir2: string): Promise<{
  identical: boolean;
  differences: string[];
}> {
  const files1 = await getAllFiles(dir1);
  const files2 = await getAllFiles(dir2);

  const differences: string[] = [];

  // Normalize paths for comparison
  const normalizePath = (filePath: string, baseDir: string) => {
    return path.relative(baseDir, filePath).replace(/\\/g, "/");
  };

  const files1Normalized = files1.map(f => normalizePath(f, dir1));
  const files2Normalized = files2.map(f => normalizePath(f, dir2));

  // Check for missing files
  for (const file of files1Normalized) {
    if (!files2Normalized.includes(file)) {
      differences.push(`Missing in ${dir2}: ${file}`);
    }
  }

  for (const file of files2Normalized) {
    if (!files1Normalized.includes(file)) {
      differences.push(`Missing in ${dir1}: ${file}`);
    }
  }

  // Check for different file contents
  for (const file of files1Normalized) {
    if (files2Normalized.includes(file)) {
      const file1Path = path.join(dir1, file);
      const file2Path = path.join(dir2, file);

      try {
        const content1 = await fs.readFile(file1Path, "utf8");
        const content2 = await fs.readFile(file2Path, "utf8");

        if (content1 !== content2) {
          differences.push(`Content differs: ${file}`);
        }
      } catch (err) {
        differences.push(`Error reading ${file}: ${err}`);
      }
    }
  }

  return {
    identical: differences.length === 0,
    differences,
  };
}

// Main check function
async function checkDocsSync(): Promise<void> {
  try {
    console.log("🔍 Checking prerequisites...");
    await checkPrerequisites();

    console.log("📦 Creating temporary directories for generated docs...");
    const tempDir = "./temp-docs";
    const tempTopkJSDir = path.join(tempDir, "topk-js");
    const tempTopkPyDir = path.join(tempDir, "topk-py");

    // Clean up any existing temp directory to start fresh
    console.log("🧹 Cleaning up any existing temporary files...");
    try {
      await fs.rm(tempDir, { recursive: true, force: true });
    } catch {
      // Ignore if directory doesn't exist
    }

    await fs.mkdir(tempDir, { recursive: true });
    await fs.mkdir(tempTopkJSDir, { recursive: true });
    await fs.mkdir(tempTopkPyDir, { recursive: true });

    try {
      // First, copy existing docs to temp directory for comparison
      console.log("📋 Copying existing docs to temp directory...");
      await runCommand("cp", ["-r", paths.topkJs.docs + "/*", tempTopkJSDir]);
      await runCommand("cp", ["-r", paths.topkPy.docs + "/*", tempTopkPyDir]);

      // Now build fresh docs (this will overwrite the existing docs)
      const topkJsResult = await buildTopkJSDocs();
      if (!topkJsResult.success) {
        throw new Error(topkJsResult.error);
      }

      const topkPyResult = await buildTopkPyDocs();
      if (!topkPyResult.success) {
        throw new Error(topkPyResult.error);
      }

      // Compare fresh generated docs with the backed-up existing docs
      console.log("🔍 Comparing fresh generated docs with existing docs...");

      const topkJSDiff = await compareDirectories(paths.topkJs.docs, tempTopkJSDir);
      const topkPyDiff = await compareDirectories(paths.topkPy.docs, tempTopkPyDir);

      console.log("\n📊 Results:");
      console.log("=".repeat(50));

      if (topkJSDiff.identical) {
        console.log("✅ topk-js docs are in sync");
      } else {
        console.log("❌ topk-js docs are NOT in sync");
        console.log("Differences:");
        topkJSDiff.differences.forEach(diff => console.log(`  - ${diff}`));
      }

      if (topkPyDiff.identical) {
        console.log("✅ topk-py docs are in sync");
      } else {
        console.log("❌ topk-py docs are NOT in sync");
        console.log("Differences:");
        topkPyDiff.differences.forEach(diff => console.log(`  - ${diff}`));
      }

      const allInSync = topkJSDiff.identical && topkPyDiff.identical;

      if (allInSync) {
        console.log("\n🎉 All SDK docs are in sync!");
      } else {
        console.log("\n⚠️  Some SDK docs are out of sync. Please regenerate the docs.");
      }
    } finally {
      // Clean up temp directory
      console.log("🧹 Cleaning up temporary files...");
      try {
        await fs.rm(tempDir, { recursive: true, force: true });
      } catch {
        // Ignore cleanup errors
      }
    }

  } catch (error) {
    console.error("❌ Error:", error.message);
    return false;
  }
}

// CLI setup
program
  .name("verify-sdk-docs-sync")
  .description("CLI to verify that SDK docs for topk-js and topk-py are in sync")
  .version("1.0.0");

program
  .command("check")
  .description("Check if SDK docs are in sync")
  .action(async () => {
    const allInSync = await checkDocsSync();
    process.exit(allInSync ? 0 : 1);
  });

program.parse();
