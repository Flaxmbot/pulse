import { notFound } from "next/navigation";
import { Markdown } from "../../../components/markdown";
import { DocsShell } from "../../../components/docs-shell";
import { Playground } from "../../../components/playground";
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
  
  // Special case for playground
  if (slugPath === "interactive/playground") {
    const docs = getAllDocs();
    return (
      <DocsShell docs={docs} activeSlug="interactive/playground">
        <div className="doc-meta">
          <span className="chip">Interactive</span>
          <span className="chip">Beginner</span>
          <span className="chip">5 min</span>
        </div>
        <div className="markdown">
          <h1>Pulse Playground</h1>
          <p>
            The Pulse Playground is an interactive environment where you can write, run, and test Pulse code 
            without having to install anything locally.
          </p>
          
          <h2>Features</h2>
          <ul>
            <li><strong>Live Code Editing</strong>: Write and edit Pulse code in the integrated editor</li>
            <li><strong>Instant Execution</strong>: Run your code with a single click</li>
            <li><strong>Real-time Output</strong>: See results and errors immediately</li>
            <li><strong>Error Handling</strong>: Clear error messages and stack traces</li>
            <li><strong>Syntax Highlighting</strong>: Code is highlighted for better readability</li>
            <li><strong>Reset Functionality</strong>: Start fresh with the default example code</li>
          </ul>
          
          <h2>Getting Started</h2>
          <ol>
            <li>Write your Pulse code in the editor on the left</li>
            <li>Click the "Run" button to execute your code</li>
            <li>View the results in the output panel on the right</li>
            <li>Click "Reset" to return to the default example</li>
          </ol>
          
          <h2>Try It Now</h2>
        </div>
        <Playground />
      </DocsShell>
    );
  }

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

