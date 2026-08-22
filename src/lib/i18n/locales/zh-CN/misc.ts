/** Misc UI copy: help, nav tabs, project selector, progress bar, mermaid lightbox. */
export default {
  help: {
    title: "术语说明",
    subtitle: "Terrain 常用概念速查",
    button: "帮助说明",
    footerKnowledge: "知识库存放在各仓库的",
    footerRegistry: "目录；项目列表登记在",
    footerEnd: "。",
  },
  progress: {
    generating: "正在生成知识资产",
  },
  errorNotice: {
    showDetails: "查看详情",
    copyLog: "复制完整日志",
  },
  projects: {
    empty: "尚无项目，请添加仓库。",
    add: "添加并初始化仓库",
    adding: "正在添加并初始化…",
    openFolder: "打开仓库目录",
    openFolderFor: "打开 {name} 的文件夹",
    remove: "从列表移除",
    removeAria: "从列表移除 {label}",
    removeConfirm:
      "从列表中移除「{label}」？\n\n仅移除 Terrain 登记，不会删除仓库或 .terrain/ 知识资产。",
    statusStale: "需修复",
    statusPartial: "待完善",
    repairStale:
      "仓库 `.terrain` 已缺失或损坏（{path}），可一键重新扫描并生成知识资产。",
    repairMissing: "尚未就绪：{assets}。",
    repairPartial: "部分知识资产尚未就绪。",
    repairSuffix: "{missing}（{path}）",
    assetJoin: "、",
  },
  nav: {
    overview: "概览",
    overviewTitle: "项目概览",
    knowledge: "知识库",
    env: "环境",
    sddTitle: "SDD 工作流：规格驱动开发",
    ariaLabel: "主导航",
  },
  mermaid: {
    title: "Mermaid 图表",
    zoomIn: "放大 (+)",
    zoomOut: "缩小 (-)",
    resetView: "重置视图 (0)",
    copyImage: "复制图片",
    copying: "复制中…",
    copiedImage: "图片已复制到剪贴板",
    copiedSvg: "SVG 源码已复制到剪贴板",
    copyFailed: "复制失败：{error}",
  },
} as const;
