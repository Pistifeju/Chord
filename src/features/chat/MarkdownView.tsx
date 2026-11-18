// The agent's message as Markdown: headings, lists, tasks, quotes, tables, links and code blocks with a copy button.
import { memo, useEffect, useState, type ReactNode } from "react";
import { openLink } from "../browser/store";
import { CheckIcon, CopyIcon } from "../../ui/icons";
import { blocksOf, type Block, type Inline } from "./markdown";
import "./markdown.css";

function Spans({ spans }: { spans: Inline[] }) {
  return spans.map((span, i): ReactNode => {
    switch (span.kind) {
      case "text":
        return span.text;
      case "code":
        return <code key={i}>{span.text}</code>;
      case "strong":
        return <strong key={i}><Spans spans={span.children} /></strong>;
      case "em":
        return <em key={i}><Spans spans={span.children} /></em>;
      case "del":
        return <del key={i}><Spans spans={span.children} /></del>;
      case "break":
        return <br key={i} />;
      case "link":
        return (
          <a
            key={i}
            href={span.href}
            title={span.href}
            onClick={(e) => {
              e.preventDefault();
              openLink(span.href);
            }}
          >
            <Spans spans={span.children} />
          </a>
        );
    }
  });
}

const HEADINGS = ["h1", "h2", "h3", "h4", "h5", "h6"] as const;

/** An icon button that copies `text` and shows a check for 1.5s after it copied. */
export function CopyButton({ text, label, className }: { text: string; label: string; className: string }) {
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(timer);
  }, [copied]);
  return (
    <button type="button" className={`icon-button ${className}`} title={label} aria-label={label} onClick={() => navigator.clipboard.writeText(text).then(() => setCopied(true), console.error)}>
      {copied ? <CheckIcon /> : <CopyIcon />}
    </button>
  );
}

function CodeBlock({ text }: { text: string }) {
  return (
    <div className="md-code">
      <CopyButton text={text} label="Copy Code" className="md-code-copy" />
      <pre>
        <code>{text}</code>
      </pre>
    </div>
  );
}

function Blocks({ blocks }: { blocks: Block[] }) {
  return blocks.map((block, i): ReactNode => {
    switch (block.kind) {
      case "heading": {
        const Tag = HEADINGS[Math.min(block.level, 6) - 1];
        return <Tag key={i} className="md-heading"><Spans spans={block.children} /></Tag>;
      }
      case "paragraph":
        return <p key={i}><Spans spans={block.children} /></p>;
      case "code":
        return <CodeBlock key={i} text={block.text} />;
      case "quote":
        return <blockquote key={i}><Blocks blocks={block.children} /></blockquote>;
      case "rule":
        return <hr key={i} />;
      case "list": {
        const Tag = block.ordered ? "ol" : "ul";
        return (
          <Tag key={i} start={block.ordered ? block.start : undefined}>
            {block.items.map((item, j) => (
              <li key={j} data-task={item.checked === null ? undefined : item.checked ? "done" : "open"}>
                {item.checked !== null && (
                  <span className="md-task" role="img" aria-label={item.checked ? "Done" : "Open"}>
                    {item.checked && <CheckIcon />}
                  </span>
                )}
                <div>
                  <Blocks blocks={item.children} />
                </div>
              </li>
            ))}
          </Tag>
        );
      }
      case "table":
        return (
          <div key={i} className="md-table">
            <table>
              <thead>
                <tr>
                  {block.head.map((cell, c) => (
                    <th key={c} style={{ textAlign: block.align[c] ?? undefined }}>
                      <Spans spans={cell} />
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {block.rows.map((row, r) => (
                  <tr key={r}>
                    {row.map((cell, c) => (
                      <td key={c} style={{ textAlign: block.align[c] ?? undefined }}>
                        <Spans spans={cell} />
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        );
    }
  });
}

export const Markdown = memo(function Markdown({ text }: { text: string }) {
  return (
    <div className="md">
      <Blocks blocks={blocksOf(text)} />
    </div>
  );
});
