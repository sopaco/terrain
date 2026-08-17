/** Knowledge browsing: document tree, article reader, TOC, markdown affordances. */
export default {
  tree: {
    title: "文档目录",
    count: "{count} 篇",
    countFiltered: "{visible}/{total}",
    collapse: "收起文档目录",
    filterPlaceholder: "筛选文档…",
    filterAria: "筛选文档",
    clearFilter: "清除筛选",
    sectionStructured: "结构化索引",
    folderModules: "模块",
    folderInterfaces: "接口",
    folderRoutes: "路由",
    folderEvents: "事件",
    folderDeepExploration: "深度探索",
    noMatchPrefix: "没有匹配「",
    noMatchSuffix: "」的文档。",
    emptyPrefix: "尚无{term}。请在工具栏点击 ",
    emptySuffix: "。",
  },
  article: {
    pathAria: "文档路径",
    backToTop: "回到顶部",
    top: "顶部",
  },
  toc: {
    ariaLabel: "文章目录",
    title: "本页目录",
    expand: "展开本页目录",
    collapse: "收起本页目录",
  },
  markdown: {
    copyCode: "复制代码",
  },
} as const;
