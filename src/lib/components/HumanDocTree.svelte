<script lang="ts">
  import { FileText, PanelLeftClose, Search, X } from "@lucide/svelte";
  import type { HumanDocEntry } from "../types";
  import { generateLabel, TERMS, UI_MESSAGES } from "../terminology";
  import ChevronIcon from "./icons/ChevronIcon.svelte";

  interface Props {
    docs: HumanDocEntry[];
    activePath?: string | null;
    loading?: boolean;
    onselect: (doc: HumanDocEntry) => void;
    oncollapse?: () => void;
  }

  let {
    docs,
    activePath = null,
    loading = false,
    onselect,
    oncollapse,
  }: Props = $props();

  type DocTreeNode = {
    id: string;
    segment: string;
    label: string;
    docs: HumanDocEntry[];
    children: DocTreeNode[];
    isFolder: boolean;
    depth: number;
  };

  const SECTION_LABELS: Record<string, string> = {
    human: TERMS.humanKnowledge,
    agent: TERMS.agentKnowledge,
    structured: "结构化索引",
  };

  const FOLDER_LABELS: Record<string, string> = {
    modules: "模块",
    interfaces: "接口",
    routes: "路由",
    events: "事件",
    "4.Deep-Exploration": "深度探索",
    "4.deep-exploration": "深度探索",
    "deep-exploration": "深度探索",
    "Deep-Exploration": "深度探索",
  };

  let openNodes = $state<Record<string, boolean>>({});
  let filter = $state("");

  function treePath(doc: HumanDocEntry): string {
    const rel = doc.relative_path.replace(/\\/g, "/");
    const section = doc.section ?? "human";
    if (section === "agent") {
      const idx = rel.indexOf("/agent/");
      return idx >= 0 ? rel.slice(idx + "/agent/".length) : rel.replace(/^agent\//, "");
    }
    if (section === "structured") {
      return rel;
    }
    const idx = rel.indexOf("/human/");
    return idx >= 0 ? rel.slice(idx + "/human/".length) : rel;
  }

  function prettyFileLabel(path: string): string {
    const base = path.split("/").pop() ?? path;
    return base
      .replace(/\.md$/i, "")
      .replace(/^\d+\.\s*/, "")
      .replace(/[-_]+/g, " ")
      .replace(/\b\w/g, (c) => c.toUpperCase());
  }

  function folderLabel(segment: string): string {
    const custom =
      FOLDER_LABELS[segment] ??
      Object.entries(FOLDER_LABELS).find(
        ([key]) => key.toLowerCase() === segment.toLowerCase(),
      )?.[1];
    const numMatch = segment.match(/^(\d+)\./);
    if (custom) {
      return numMatch ? `${numMatch[1]}.${custom}` : custom;
    }
    return prettyFileLabel(segment);
  }

  function segmentOrderKey(segment: string): string {
    const numMatch = segment.match(/^(\d+)/);
    if (numMatch) return numMatch[1].padStart(4, "0");
    return `9999-${segment}`;
  }

  function rowIndent(depth: number): string {
    return `${8 + depth * 12}px`;
  }

  function isDeepExplorationSegment(segment: string): boolean {
    return /deep[- ]?exploration/i.test(segment);
  }

  function defaultOpen(node: DocTreeNode): boolean {
    if (node.isFolder && isDeepExplorationSegment(node.segment)) return false;
    if (node.isFolder) return node.depth < 2;
    return true;
  }

  function compareNodes(a: DocTreeNode, b: DocTreeNode): number {
    const order = segmentOrderKey(a.segment).localeCompare(
      segmentOrderKey(b.segment),
      undefined,
      { numeric: true, sensitivity: "base" },
    );
    if (order !== 0) return order;
    if (a.isFolder !== b.isFolder) return a.isFolder ? -1 : 1;
    return a.label.localeCompare(b.label, undefined, { numeric: true, sensitivity: "base" });
  }

  function buildTree(entries: HumanDocEntry[]): DocTreeNode[] {
    const rootChildren: DocTreeNode[] = [];

    const sorted = [...entries].sort((a, b) =>
      treePath(a).localeCompare(treePath(b), undefined, { numeric: true, sensitivity: "base" }),
    );

    for (const doc of sorted) {
      const rel = treePath(doc);
      const parts = rel.split("/").filter(Boolean);
      if (parts.length === 0) continue;

      if (parts.length === 1) {
        rootChildren.push({
          id: rel,
          segment: parts[0],
          label: doc.title || prettyFileLabel(parts[0]),
          docs: [doc],
          children: [],
          isFolder: false,
          depth: 0,
        });
        continue;
      }

      let siblings = rootChildren;
      let pathPrefix = "";
      let depth = 0;

      for (let i = 0; i < parts.length - 1; i++) {
        const segment = parts[i];
        pathPrefix = pathPrefix ? `${pathPrefix}/${segment}` : segment;
        let node = siblings.find((n) => n.isFolder && n.segment === segment);
        if (!node) {
          node = {
            id: pathPrefix,
            segment,
            label: folderLabel(segment),
            docs: [],
            children: [],
            isFolder: true,
            depth,
          };
          siblings.push(node);
        }
        siblings = node.children;
        depth += 1;
      }

      const parent = findNode(rootChildren, parts.slice(0, -1).join("/"));
      if (parent) {
        parent.docs.push(doc);
      }
    }

    const sortTree = (nodes: DocTreeNode[]): DocTreeNode[] =>
      [...nodes]
        .sort(compareNodes)
        .map((n) => ({ ...n, children: sortTree(n.children) }));

    return sortTree(rootChildren);
  }

  function findNode(nodes: DocTreeNode[], id: string): DocTreeNode | null {
    for (const node of nodes) {
      if (node.id === id) return node;
      const found = findNode(node.children, id);
      if (found) return found;
    }
    return null;
  }

  const query = $derived(filter.trim().toLowerCase());

  const visibleDocs = $derived.by(() => {
    if (!query) return docs;
    return docs.filter(
      (doc) =>
        doc.title.toLowerCase().includes(query) ||
        treePath(doc).toLowerCase().includes(query),
    );
  });

  const sections = $derived.by(() => {
    const grouped = new Map<string, HumanDocEntry[]>();
    for (const doc of visibleDocs) {
      const section = doc.section ?? "human";
      const list = grouped.get(section) ?? [];
      list.push(doc);
      grouped.set(section, list);
    }
    return [...grouped.entries()].sort(([a], [b]) => a.localeCompare(b));
  });

  $effect(() => {
    const defaults: Record<string, boolean> = {};
    const walk = (nodes: DocTreeNode[]) => {
      for (const node of nodes) {
        if (openNodes[node.id] === undefined) {
          defaults[node.id] = defaultOpen(node);
        }
        walk(node.children);
      }
    };
    for (const [, sectionDocs] of sections) {
      walk(buildTree(sectionDocs));
    }
    if (Object.keys(defaults).length > 0) {
      openNodes = { ...openNodes, ...defaults };
    }
  });

  function toggle(id: string, fallback: boolean) {
    openNodes = { ...openNodes, [id]: !(openNodes[id] ?? fallback) };
  }

  function isOpen(id: string, fallback: boolean) {
    // While filtering, every branch that survived the filter should be visible.
    if (query) return true;
    return openNodes[id] ?? fallback;
  }

  function nodeCount(node: DocTreeNode): number {
    let count = node.docs.length;
    for (const child of node.children) count += nodeCount(child);
    return count;
  }
</script>

{#snippet docButton(doc: HumanDocEntry, depth: number)}
  <button
    type="button"
    class={`mb-0.5 flex w-full items-center gap-1.5 rounded-md py-1.5 pr-2 text-left text-xs leading-snug transition-colors ${
      activePath === doc.path
        ? "bg-tr-accent-soft-strong font-medium text-tr-on-accent hover:bg-tr-accent-soft-strong"
        : "text-tr-ink-2 hover:bg-tr-elevated"
    }`}
    style={`padding-left: ${rowIndent(depth)}`}
    onclick={() => onselect(doc)}
    title={treePath(doc)}
  >
    <FileText
      size={12}
      strokeWidth={2}
      class="w-3 shrink-0 text-tr-ink-4"
      aria-hidden="true"
    />
    <span class="min-w-0 flex-1 truncate">{doc.title || prettyFileLabel(treePath(doc))}</span>
  </button>
{/snippet}

{#snippet treeNode(node: DocTreeNode)}
  {@const fallbackOpen = defaultOpen(node)}
  {@const deepFolder = node.isFolder && isDeepExplorationSegment(node.segment)}

  {#if !node.isFolder && node.docs.length === 1}
    {@render docButton(node.docs[0], node.depth)}
  {:else}
    <div>
      <button
        type="button"
        class={`mb-0.5 flex w-full items-center gap-1.5 rounded-md py-1.5 pr-2 text-left text-xs leading-snug transition-colors hover:bg-tr-elevated ${
          deepFolder && node.depth > 0 ? "text-tr-accent" : "text-tr-ink-2"
        }`}
        style={`padding-left: ${rowIndent(node.depth)}`}
        onclick={() => toggle(node.id, fallbackOpen)}
        aria-expanded={isOpen(node.id, fallbackOpen)}
        title={node.id}
      >
        <span class="inline-flex w-3 shrink-0 items-center justify-center text-tr-ink-3">
          <ChevronIcon
            direction={isOpen(node.id, fallbackOpen) ? "down" : "right"}
            size={12}
            class="shrink-0"
          />
        </span>
        <span
          class={`min-w-0 flex-1 truncate ${
            deepFolder && node.depth > 0 ? "text-tr-accent" : ""
          }`}
        >
          {node.label}
        </span>
        <span class="shrink-0 text-[10px] text-tr-ink-4">{nodeCount(node)}</span>
      </button>

      {#if isOpen(node.id, fallbackOpen)}
        <div class="space-y-0.5 pb-1">
          {#each node.docs as doc}
            {@render docButton(doc, node.depth + 1)}
          {/each}
          {#each node.children as child}
            {@render treeNode(child)}
          {/each}
        </div>
      {/if}
    </div>
  {/if}
{/snippet}

<div class="flex min-h-0 flex-1 flex-col">
  <div class="border-b border-tr-border-strong px-3 py-2.5">
    <div class="flex items-center gap-2">
      <span class="min-w-0 flex-1 text-xs font-semibold text-tr-ink-2">文档目录</span>
      {#if loading}
        <span class="inline-flex shrink-0 items-center gap-1.5 text-[10px] text-tr-accent">
          <span class="h-2.5 w-2.5 animate-spin rounded-full border border-current border-t-transparent"></span>
          {UI_MESSAGES.loadingDocs}
        </span>
      {:else}
        <span class="shrink-0 text-[10px] text-tr-ink-3">
          {query ? `${visibleDocs.length}/${docs.length}` : `${docs.length} 篇`}
        </span>
      {/if}
      {#if oncollapse}
        <button
          type="button"
          class="tr-press -mr-1 inline-flex shrink-0 items-center justify-center rounded-md p-1 text-tr-ink-3 transition-colors hover:bg-tr-elevated hover:text-tr-ink"
          onclick={oncollapse}
          aria-label="收起文档目录"
          title="收起文档目录"
        >
          <PanelLeftClose size={14} strokeWidth={2} aria-hidden="true" />
        </button>
      {/if}
    </div>

    <div class="relative mt-2">
      <Search
        size={12}
        strokeWidth={2}
        class="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-tr-ink-4"
        aria-hidden="true"
      />
      <input
        class="w-full rounded-md border border-tr-border bg-tr-elevated py-1 pl-6 pr-6 text-xs text-tr-ink outline-none placeholder:text-tr-ink-4 focus:border-tr-accent"
        placeholder="筛选文档…"
        aria-label="筛选文档"
        bind:value={filter}
      />
      {#if filter}
        <button
          type="button"
          class="absolute right-1 top-1/2 inline-flex -translate-y-1/2 items-center justify-center rounded p-0.5 text-tr-ink-4 transition-colors hover:text-tr-ink"
          onclick={() => (filter = "")}
          aria-label="清除筛选"
          title="清除筛选"
        >
          <X size={12} strokeWidth={2} aria-hidden="true" />
        </button>
      {/if}
    </div>
  </div>

  <div class="flex-1 overflow-y-auto px-2 py-2">
    {#if loading && docs.length === 0}
      <div class="space-y-2 px-1 py-2">
        {#each Array(4) as _}
          <div class="h-7 animate-pulse rounded-md bg-tr-elevated"></div>
        {/each}
      </div>
    {:else if sections.length > 0}
      <div class="space-y-3">
        {#each sections as [section, sectionDocs]}
          <div>
            <p class="mb-1 px-2 text-[10px] font-semibold uppercase tracking-wider text-tr-ink-3">
              {SECTION_LABELS[section] ?? section}
            </p>
            <div class="space-y-1">
              {#each buildTree(sectionDocs) as node}
                {@render treeNode(node)}
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {:else if query}
      <p class="px-2 py-4 text-xs leading-relaxed text-tr-ink-3">
        没有匹配「<span class="text-tr-ink-2">{filter.trim()}</span>」的文档。
      </p>
    {:else if !loading}
      <p class="px-2 py-4 text-xs leading-relaxed text-tr-ink-3">
        尚无{TERMS.humanKnowledge}。请在工具栏点击 <span class="text-tr-ink-2">{generateLabel(TERMS.humanKnowledge, false)}</span>。
      </p>
    {/if}
  </div>
</div>
