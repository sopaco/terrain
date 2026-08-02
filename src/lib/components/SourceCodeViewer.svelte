<script lang="ts">
  import { tick } from "svelte";
  import type { HighlightedLine } from "../highlightSetup";
  import "../syntax-tokens.css";
  import "../source-code.css";

  interface Props {
    content: string;
    filePath: string;
    startLine?: number;
    focusLine?: number;
  }

  let { content, filePath, startLine = 1, focusLine }: Props = $props();

  let container = $state<HTMLDivElement | null>(null);
  let lines = $state<HighlightedLine[]>([]);

  $effect(() => {
    const nextContent = content;
    const nextPath = filePath;
    const nextStart = startLine;
    let cancelled = false;

    void import("../highlightSetup").then(({ highlightSourceLines }) => {
      if (cancelled) return;
      lines = highlightSourceLines(nextContent, nextPath, nextStart);
    });

    return () => {
      cancelled = true;
    };
  });

  // Highlighting is async; wait until rows exist before scrolling to the cited line.
  $effect(() => {
    const target = focusLine;
    const root = container;
    const rendered = lines.length;
    if (!target || !root || rendered === 0) return;

    void tick().then(() => {
      const row = root.querySelector<HTMLElement>(`tr[data-line="${target}"]`);
      row?.scrollIntoView({ block: "center", behavior: "smooth" });
    });
  });
</script>

<div
  class="source-code-viewer tr-syntax"
  bind:this={container}
  role="region"
  aria-label="Source code"
>
  <table>
    <tbody>
      {#each lines as line (line.number)}
        <tr
          data-line={line.number}
          class:focus-line={focusLine != null && line.number === focusLine}
        >
          <td class="line-num">{line.number}</td>
          <td class="line-code"><code class="hljs">{@html line.html}</code></td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
