import { MarkdownPageEvent } from 'typedoc-plugin-markdown';

export function load(app) {
  app.renderer.on(MarkdownPageEvent.BEGIN, (page) => {
    page.frontmatter = {
      title: `topk-js/${page.model.name}`,
      ...page.frontmatter,
    };
  });
}