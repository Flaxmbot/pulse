import Link from "next/link";
import { getAllDocs } from "../lib/docs";
import { Playground } from "../components/playground";

export default function HomePage() {
  const docs = getAllDocs();
  const featured = [
    docs.find((doc) => doc.slug === "start/installation"),
    docs.find((doc) => doc.slug === "tutorials/build-a-cli"),
    docs.find((doc) => doc.slug === "reference/grammar"),
    docs.find((doc) => doc.slug === "guides/aot-jit"),
    docs.find((doc) => doc.slug === "advanced/selfhost-bootstrap"),
    docs.find((doc) => doc.slug === "advanced/production-checklist")
  ].filter(Boolean) as typeof docs;

  return (
    <div className="home">
      <section className="hero">
        <h1>Pulse Documentation</h1>
        <p>
          Full documentation from beginner to advanced: tutorials, grammar
          reference, language guides, production workflows, and compiler/runtime
          internals.
        </p>
        <Link href="/docs/start/overview" className="primary">
          Start with Overview
        </Link>

        <div className="hero-grid">
          {featured.map((doc) => (
            <Link className="hero-card" key={doc.slug} href={`/docs/${doc.slug}`}>
              <strong>{doc.title}</strong>
              <span>{doc.summary}</span>
            </Link>
          ))}
        </div>
      </section>

      {/* Interactive Playground */}
      <section className="playground-section">
        <Playground />
      </section>
    </div>
  );
}

