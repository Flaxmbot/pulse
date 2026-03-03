import { notFound } from "next/navigation";
import { Markdown } from "../../../components/markdown";
import { DocsShell } from "../../../components/docs-shell";
import { getAllDocs, getDocBySlug } from "../../../lib/docs";

type Params = {
  slug?: string[];
};

export function generateStaticParams(): Params[] {
  return getAllDocs().map((doc) => ({
    slug: doc.slug.split("/")
  }));
}

export default function DocPage({
  params
}: {
  params: Params;
}) {
  const slugPath = (params.slug ?? []).join("/");
  const resolvedSlug = slugPath || "start/overview";
  const doc = getDocBySlug(resolvedSlug);
  if (!doc) notFound();

  const docs = getAllDocs();

  return (
    <DocsShell docs={docs} activeSlug={doc.slug}>
      <div className="doc-meta">
        <span className="chip">{doc.section}</span>
        <span className="chip">{doc.level}</span>
        <span className="chip">{doc.readTime}</span>
      </div>
      <Markdown content={doc.content} />
    </DocsShell>
  );
}

