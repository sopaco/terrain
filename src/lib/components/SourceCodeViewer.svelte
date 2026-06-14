<script lang="ts">
  import { highlightSourceLines } from "../highlightSetup";
  import "../source-code.css";

  interface Props {
    content: string;
    filePath: string;
    startLine?: number;
    focusLine?: number;
  }

  let { content, filePath, startLine = 1, focusLine }: Props = $props();

  let container = $state<HTMLDivElement | null>(null);

  const lines = $derived(highlightSourceLines(content, filePath, startLine));

  $effect(() => {
    const target = focusLine;
    const root = container;
    if (!target || !root) return;

    queueMicrotask(() => {
      const row = root.querySelector<HTMLElement>(`tr[data-line="${target}"]`);
      row?.scrollIntoView({ block: "center", behavior: "smooth" });
    });
  });
</script>

<div class="source-code-viewer" bind:this={container} role="region" aria-label="Source code">
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
