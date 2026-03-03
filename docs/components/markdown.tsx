type CodeBlock = {
  lang: string;
  code: string;
};

type ParsedBlock =
  | { kind: "h1"; text: string }
  | { kind: "h2"; text: string }
  | { kind: "h3"; text: string }
  | { kind: "p"; text: string }
  | { kind: "ul"; items: string[] }
  | { kind: "ol"; items: string[] }
  | { kind: "code"; block: CodeBlock }
  | { kind: "hr" };

function parseMarkdown(markdown: string): ParsedBlock[] {
  const lines = markdown.replace(/\r\n/g, "\n").split("\n");
  const blocks: ParsedBlock[] = [];
  let i = 0;

  const isSpecial = (line: string): boolean =>
    line.startsWith("# ") ||
    line.startsWith("## ") ||
    line.startsWith("### ") ||
    line.startsWith("- ") ||
    /^\d+\.\s+/.test(line) ||
    line.startsWith("```") ||
    line.trim() === "---";

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    if (!trimmed) {
      i += 1;
      continue;
    }

    if (trimmed === "---") {
      blocks.push({ kind: "hr" });
      i += 1;
      continue;
    }

    if (line.startsWith("```")) {
      const lang = line.slice(3).trim();
      i += 1;
      const codeLines: string[] = [];
      while (i < lines.length && !lines[i].startsWith("```")) {
        codeLines.push(lines[i]);
        i += 1;
      }
      if (i < lines.length) {
        i += 1;
      }
      blocks.push({
        kind: "code",
        block: { lang, code: codeLines.join("\n") },
      });
      continue;
    }

    if (line.startsWith("# ")) {
      blocks.push({ kind: "h1", text: line.slice(2).trim() });
      i += 1;
      continue;
    }
    if (line.startsWith("## ")) {
      blocks.push({ kind: "h2", text: line.slice(3).trim() });
      i += 1;
      continue;
    }
    if (line.startsWith("### ")) {
      blocks.push({ kind: "h3", text: line.slice(4).trim() });
      i += 1;
      continue;
    }

    if (line.startsWith("- ")) {
      const items: string[] = [];
      while (i < lines.length && lines[i].startsWith("- ")) {
        items.push(lines[i].slice(2).trim());
        i += 1;
      }
      blocks.push({ kind: "ul", items });
      continue;
    }

    if (/^\d+\.\s+/.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^\d+\.\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\d+\.\s+/, "").trim());
        i += 1;
      }
      blocks.push({ kind: "ol", items });
      continue;
    }

    const paragraph: string[] = [trimmed];
    i += 1;
    while (
      i < lines.length &&
      lines[i].trim() &&
      !isSpecial(lines[i])
    ) {
      paragraph.push(lines[i].trim());
      i += 1;
    }
    blocks.push({ kind: "p", text: paragraph.join(" ") });
  }

  return blocks;
}

function InlineCode({ text }: { text: string }) {
  const parts = text.split(/(`[^`]+`)/g).filter(Boolean);
  return (
    <>
      {parts.map((part, idx) => {
        if (part.startsWith("`") && part.endsWith("`")) {
          return (
            <code key={idx} className="inline-code">
              {part.slice(1, -1)}
            </code>
          );
        }
        return <span key={idx}>{part}</span>;
      })}
    </>
  );
}

export function Markdown({ content }: { content: string }) {
  const blocks = parseMarkdown(content);

  return (
    <article className="markdown">
      {blocks.map((block, index) => {
        if (block.kind === "h1") return <h1 key={index}>{block.text}</h1>;
        if (block.kind === "h2") return <h2 key={index}>{block.text}</h2>;
        if (block.kind === "h3") return <h3 key={index}>{block.text}</h3>;
        if (block.kind === "p")
          return (
            <p key={index}>
              <InlineCode text={block.text} />
            </p>
          );
        if (block.kind === "ul")
          return (
            <ul key={index}>
              {block.items.map((item, itemIndex) => (
                <li key={itemIndex}>
                  <InlineCode text={item} />
                </li>
              ))}
            </ul>
          );
        if (block.kind === "ol")
          return (
            <ol key={index}>
              {block.items.map((item, itemIndex) => (
                <li key={itemIndex}>
                  <InlineCode text={item} />
                </li>
              ))}
            </ol>
          );
        if (block.kind === "code")
          return (
            <div key={index} className="code-wrap">
              {block.block.lang ? (
                <div className="code-lang">{block.block.lang}</div>
              ) : null}
              <pre>
                <code>{block.block.code}</code>
              </pre>
            </div>
          );
        return <hr key={index} />;
      })}
    </article>
  );
}

