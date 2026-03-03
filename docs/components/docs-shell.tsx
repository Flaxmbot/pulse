"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import type { DocPage } from "../lib/docs";
import { SECTION_ORDER } from "../lib/docs";

type DocsShellProps = {
  docs: DocPage[];
  activeSlug: string;
  children: React.ReactNode;
};

export function DocsShell({ docs, activeSlug, children }: DocsShellProps) {
  const [query, setQuery] = useState("");

  const grouped = useMemo(() => {
    const lowerQuery = query.trim().toLowerCase();
    const filtered = !lowerQuery
      ? docs
      : docs.filter((doc) => {
          const haystack = [
            doc.title,
            doc.summary,
            doc.section,
            doc.level,
            ...doc.keywords,
          ]
            .join(" ")
            .toLowerCase();
          return haystack.includes(lowerQuery);
        });

    const bySection: Record<string, DocPage[]> = {};
    for (const section of SECTION_ORDER) {
      bySection[section] = filtered.filter((doc) => doc.section === section);
    }
    return bySection;
  }, [docs, query]);

  return (
    <div className="docs-grid">
      <aside className="sidebar">
        <Link href="/" className="brand">
          Pulse Docs
        </Link>
        <p className="brand-subtitle">Beginner to advanced documentation</p>
        <input
          className="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Search docs..."
          aria-label="Search docs"
        />

        {SECTION_ORDER.map((section) => {
          const sectionDocs = grouped[section] ?? [];
          if (!sectionDocs.length) return null;
          return (
            <div key={section} className="nav-group">
              <h3>{section}</h3>
              <ul>
                {sectionDocs.map((doc) => {
                  const href = `/docs/${doc.slug}`;
                  const active = activeSlug === doc.slug;
                  return (
                    <li key={doc.slug}>
                      <Link href={href} className={active ? "active" : ""}>
                        <span>{doc.title}</span>
                        <small>{doc.readTime}</small>
                      </Link>
                    </li>
                  );
                })}
              </ul>
            </div>
          );
        })}

        <div className="sidebar-footer">
          <Link href="/docs/interactive/playground" className="playground-link">
            🎮 Try the Playground
          </Link>
        </div>
      </aside>

      <main className="docs-main">{children}</main>
    </div>
  );
}

