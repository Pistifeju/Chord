const BLOCK = "p,li,h1,h2,h3,h4,h5,h6,td,th,pre,div,blockquote";

export function matchesIn(root: Element, query: string): Range[] {
  if (!query) return [];
  const needle = query.toLowerCase();
  const ranges: Range[] = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let run: Text[] = [];
  let block: Element | null = null;
  const flush = () => {
    let text = "";
    const starts = run.map((node) => {
      const start = text.length;
      text += node.data.toLowerCase();
      return start;
    });
    const nodeAt = (offset: number, end: boolean) => {
      let k = starts.length - 1;
      while (k > 0 && (end ? starts[k] >= offset : starts[k] > offset)) k--;
      return k;
    };
    for (let at = text.indexOf(needle); at >= 0; at = text.indexOf(needle, at + needle.length)) {
      const first = nodeAt(at, false);
      const last = nodeAt(at + needle.length, true);
      const range = new Range();
      range.setStart(run[first], at - starts[first]);
      range.setEnd(run[last], at + needle.length - starts[last]);
      ranges.push(range);
    }
    run = [];
  };
  for (let node = walker.nextNode() as Text | null; node; node = walker.nextNode() as Text | null) {
    const own = node.parentElement?.closest(BLOCK) ?? null;
    if (own !== block) flush();
    block = own;
    run.push(node);
  }
  flush();
  return ranges;
}

const painted = new Map<string, { all: Range[]; current: Range | undefined }>();

function repaint() {
  if (!("highlights" in CSS)) return;
  const all = [...painted.values()].flatMap((pane) => pane.all);
  const current = [...painted.values()].flatMap((pane) => (pane.current ? [pane.current] : []));
  CSS.highlights.set("find-match", new Highlight(...all));
  CSS.highlights.set("find-current", new Highlight(...current));
}

export function paintFind(pane: string, all: Range[], current: Range | undefined) {
  painted.set(pane, { all, current });
  repaint();
}

export function clearFind(pane: string) {
  painted.delete(pane);
  repaint();
}
