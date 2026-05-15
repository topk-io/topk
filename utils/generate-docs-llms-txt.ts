import { Command } from "commander";
import docs from "../docs/docs.json" with { type: "json" };

const BASE_URL = "https://docs.topk.io";
const DOCS_DIR = new URL("../docs", import.meta.url).pathname;

const program = new Command();

program
  .name("sdk-generate-docs-llms-txt")
  .description("CLI to generate llms.txt and llms-full.txt in the docs")
  .version("0.1.0");

type SlugEntry = { type: "slug"; slug: string; titlePrefix?: string; useFirstParagraph?: boolean };
type ExternalEntry = { type: "external"; title: string; url: string; description?: string };
type Entry = SlugEntry | ExternalEntry;
type Section = { heading: string; entries: Entry[] };
type PageGroup = { group?: string; pages: string[] };

function normalizeTabPages(tabPages: unknown): PageGroup[] {
  if (Array.isArray(tabPages))
    return tabPages.map((item) =>
      typeof item === "string" ? { pages: [item] } : (item as PageGroup)
    );
  if (typeof tabPages === "object" && tabPages !== null && "pages" in tabPages)
    return [tabPages as PageGroup];
  return [];
}

function slugToUrl(slug: string): string {
  return `${BASE_URL}/${slug.replace(/\/index$/, "")}`;
}

async function readFrontmatter(slug: string): Promise<{ title?: string; description?: string }> {
  const file = Bun.file(`${DOCS_DIR}/${slug}.mdx`);
  if (!(await file.exists())) return {};

  const text = await file.text();
  const match = text.match(/^---\n([\s\S]*?)\n---/);
  if (!match) return {};

  const fm = match[1];
  const title = fm.match(/^title:\s*["']?(.+?)["']?\s*$/m)?.[1];
  const description = fm.match(/^description:\s*["']?(.+?)["']?\s*$/m)?.[1];
  return { title, description };
}

async function readFirstParagraph(slug: string): Promise<string | undefined> {
  const file = Bun.file(`${DOCS_DIR}/${slug}.mdx`);
  if (!(await file.exists())) return undefined;

  const text = await file.text();
  const body = text
    .replace(/^---\n[\s\S]*?\n---\n?/, "")
    .replace(/^import\s+.+\n/gm, "");

  for (const block of body.split(/\n\n+/)) {
    const trimmed = block.trim();
    if (!trimmed) continue;
    if (trimmed.startsWith("#")) continue;
    if (trimmed.startsWith("<")) continue;
    if (trimmed.startsWith("```")) continue;
    return trimmed.replace(/\n/g, " ");
  }
  return undefined;
}

async function readPageContent(slug: string): Promise<string> {
  const file = Bun.file(`${DOCS_DIR}/${slug}.mdx`);
  if (!(await file.exists())) return "";

  const text = await file.text();
  return text
    .replace(/^---\n[\s\S]*?\n---\n?/, "")   // strip frontmatter
    .replace(/^import\s+.+\n/gm, "")         // strip MDX imports
    .replace(/[ \t]+$/gm, "")                // strip trailing whitespace per line
    .trim();
}

// --- Navigation ---

const OVERVIEW_EXCLUDE = new Set(["cli", "mcp-server"]);
const LLMS_EXCLUDE = new Set(["usage"]);
const LLMS_MOVE_TO_CORE_CONCEPTS = ["datasets/index"];

const API_ENTRIES: Entry[] = [
  { type: "slug", slug: "cli" },
  { type: "slug", slug: "mcp-server" },
  { type: "slug", slug: "sdk/topk-py/overview" },
  { type: "slug", slug: "sdk/topk-js/overview" },
  {
    type: "external",
    title: "Rust SDK",
    url: "https://github.com/topk-io/topk/tree/main/topk-rs",
    description: "Get started with TopK Context Engine using Rust SDK.",
  },
];

function buildSections(): Section[] {
  const sections: Section[] = [];

  // Documentation tab
  const docTab = docs.navigation.tabs.find((t) => t.tab === "Documentation");
  if (docTab) {
    for (const group of normalizeTabPages(docTab.pages)) {
      const isOverview = group.group === "Overview" || !group.group;
      let slugs = isOverview
        ? group.pages.filter((s) => !OVERVIEW_EXCLUDE.has(s))
        : group.pages.filter((s) => !LLMS_EXCLUDE.has(s) && !LLMS_MOVE_TO_CORE_CONCEPTS.includes(s));

      if (group.group === "Core Concepts") {
        slugs = [...slugs, ...LLMS_MOVE_TO_CORE_CONCEPTS];
      }

      if (slugs.length === 0) continue;

      sections.push({
        heading: isOverview ? "Overview" : group.group!,
        entries: slugs.map((slug): Entry => ({ type: "slug", slug })),
      });

      if (group.group === "Core Concepts") {
        sections.push({ heading: "APIs", entries: API_ENTRIES });
      }
    }
  }

  // Python SDK Reference
  const pyTab = docs.navigation.tabs.find((t) => t.tab === "Python SDK");
  const pyRef = pyTab && normalizeTabPages(pyTab.pages).find((g) => g.group === "SDK Reference");
  if (pyRef) {
    sections.push({
      heading: "Python SDK Reference",
      entries: pyRef.pages.map((slug): Entry => ({ type: "slug", slug })),
    });
  }

  // JavaScript SDK Reference
  const jsTab = docs.navigation.tabs.find((t) => t.tab === "JavaScript SDK");
  const jsRef = jsTab && normalizeTabPages(jsTab.pages).find((g) => g.group === "SDK Reference");
  if (jsRef) {
    sections.push({
      heading: "JavaScript SDK Reference",
      entries: jsRef.pages.map((slug): Entry => ({ type: "slug", slug })),
    });
  }

  // Database tab
  const dbTab = docs.navigation.tabs.find((t) => t.tab === "Database");
  if (dbTab) {
    const DB_CONCEPT_SLUGS = [
      "concepts/semantic-search",
      "concepts/vector-search",
      "concepts/sparse-vector-search",
      "concepts/multi-vector-search",
      "concepts/keyword-search",
      "concepts/true-hybrid-search",
    ];
    sections.push({
      heading: "Database APIs",
      entries: [
        { type: "slug", slug: "database", useFirstParagraph: true },
        ...DB_CONCEPT_SLUGS.map((slug): Entry => ({ type: "slug", slug, titlePrefix: "TopK Database - " })),
      ],
    });
  }

  return sections;
}

// --- Renderers ---

const HEADER =
  "# TopK Documentation\n\n" +
  "> The context layer for vertical AI agents.\n\n";

async function renderLinks(sections: Section[]): Promise<string> {
  let out = HEADER;
  for (const section of sections) {
    out += `## ${section.heading}\n\n`;
    for (const entry of section.entries) {
      if (entry.type === "external") {
        const line = `- [${entry.title}](${entry.url})`;
        out += entry.description ? `${line}: ${entry.description}\n` : `${line}\n`;
      } else {
        const { title, description: fmDesc } = await readFrontmatter(entry.slug);
        const description = entry.useFirstParagraph
          ? await readFirstParagraph(entry.slug)
          : fmDesc;
        const displayTitle = `${entry.titlePrefix ?? ""}${title ?? entry.slug}`;
        const line = `- [${displayTitle}](${slugToUrl(entry.slug)})`;
        out += description ? `${line}: ${description}\n` : `${line}\n`;
      }
    }
    out += "\n";
  }
  return out.trimEnd() + "\n";
}

async function renderFull(sections: Section[]): Promise<string> {
  let out = HEADER;
  for (const section of sections) {
    out += `## ${section.heading}\n\n`;
    for (const entry of section.entries) {
      if (entry.type === "external") {
        out += `### ${entry.title}\n\n`;
        if (entry.description) out += `${entry.description}\n\n`;
        out += `URL: ${entry.url}\n\n`;
      } else {
        const { title } = await readFrontmatter(entry.slug);
        const displayTitle = `${entry.titlePrefix ?? ""}${title ?? entry.slug}`;
        const content = await readPageContent(entry.slug);
        out += `### ${displayTitle}\n\n`;
        out += `URL: ${slugToUrl(entry.slug)}\n\n`;
        if (content) out += `${content}\n\n`;
      }
    }
  }
  return out.trimEnd() + "\n";
}

// --- Commands ---

program.command("generate").action(async () => {
  const sections = buildSections();
  await Bun.write(`${DOCS_DIR}/llms.txt`, await renderLinks(sections));
  console.log("llms.txt generated successfully");
  await Bun.write(`${DOCS_DIR}/llms-full.txt`, await renderFull(sections));
  console.log("llms-full.txt generated successfully");
});

program.command("generate-links").action(async () => {
  const output = await renderLinks(buildSections());
  await Bun.write(`${DOCS_DIR}/llms.txt`, output);
  console.log("llms.txt generated successfully");
});

program.command("generate-full").action(async () => {
  const output = await renderFull(buildSections());
  await Bun.write(`${DOCS_DIR}/llms-full.txt`, output);
  console.log("llms-full.txt generated successfully");
});

program.parse();
