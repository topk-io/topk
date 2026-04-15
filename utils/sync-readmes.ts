// usage: bun run utils/sync-readmes.ts

import fs from "fs/promises";
import path from "path";

const ROOT = path.resolve(import.meta.dir, "..");

type Source = {
  readme: string;
  output: string;
  frontmatter: Record<string, string>;
};

const SOURCES: Source[] = [
  {
    readme: path.join(ROOT, "topk-py", "README.md"),
    output: path.join(ROOT, "docs", "sdk", "topk-py", "overview.mdx"),
    frontmatter: {
      title: "Python SDK",
      description: "Python library for the TopK API",
    },
  },
  {
    readme: path.join(ROOT, "topk-js", "README.md"),
    output: path.join(ROOT, "docs", "sdk", "topk-js", "overview.mdx"),
    frontmatter: {
      title: "JavaScript SDK",
      description: "TypeScript/JavaScript library for the TopK API",
    },
  },
  {
    readme: path.join(ROOT, "topk-cli", "README.md"),
    output: path.join(ROOT, "docs", "cli.mdx"),
    frontmatter: {
      title: "CLI",
      description:
        "Manage datasets, upload files, search, and ask questions from the command line.",
      icon: "terminal",
    },
  },
];

function buildFrontmatter(fields: Record<string, string>): string {
  const lines = ["---"];
  for (const [key, value] of Object.entries(fields)) {
    lines.push(`${key}: "${value}"`);
  }
  lines.push("---");
  return lines.join("\n");
}

function stripReadmeHeader(content: string): string {
  const lines = content.split("\n");
  const out: string[] = [];
  let i = 0;

  // Skip leading image, h1, and "Full documentation" notice (plus their trailing blank lines)
  while (i < lines.length) {
    const line = lines[i];
    if (
      line.startsWith("![") ||
      line.startsWith("# ") ||
      line.trimStart().startsWith("**Full documentation is available")
    ) {
      i++;
      // Skip the blank line that follows
      if (lines[i]?.trim() === "") i++;
      continue;
    }
    break;
  }

  out.push(...lines.slice(i));
  return out.join("\n").replace(/^\n+/, "");
}

async function syncReadme(source: Source): Promise<void> {
  try {
    await fs.access(source.readme);
  } catch {
    console.log(`Skipping ${source.readme} (not found)`);
    return;
  }

  const content = await fs.readFile(source.readme, "utf-8");
  const body = stripReadmeHeader(content);
  const frontmatter = buildFrontmatter(source.frontmatter);
  const output = `${frontmatter}\n\n${body}\n`;

  await fs.writeFile(source.output, output, "utf-8");
  console.log(
    `Synced ${path.relative(ROOT, source.readme)} → ${path.relative(ROOT, source.output)}`
  );
}

for (const source of SOURCES) {
  await syncReadme(source);
}
