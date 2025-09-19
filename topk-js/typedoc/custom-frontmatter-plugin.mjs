import { MarkdownPageEvent } from "typedoc-plugin-markdown";

/**
 * @param {import('typedoc-plugin-markdown').MarkdownApplication} app
 */

export function load(app) {
  app.renderer.on(MarkdownPageEvent.END, (page) => {
    const titlesToReplace = ["Methods", "Properties", "Constructors", "Constructor", "Parameters", "Returns"];

    titlesToReplace.forEach(title => {
      const regex = new RegExp(`^(#{1,6})\\s*${title}`, 'gm');
      page.contents = page.contents.replace(regex, `**${title}**`);
    });

    page.contents = page.contents.replaceAll(/\[([^\]]*)\]\(([^)]*?)\.mdx([^)]*)\)/g, '[$1]($2$3)');
  });

  app.renderer.on(MarkdownPageEvent.BEGIN, (page) => {
    page.frontmatter = {
      title: `topk-js/${page.model.name}`,
      ...page.frontmatter,
    };
  });
}
