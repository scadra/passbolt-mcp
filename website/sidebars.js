// The sidebar, grouped as the MCP documentation is: get started, learn, use, security, reference.
/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
module.exports = {
  docs: [
    {
      type: "category",
      label: "Get started",
      collapsible: false,
      items: ["index", "getting-started/installation", "getting-started/quick-start"],
    },
    {
      type: "category",
      label: "Learn",
      collapsible: false,
      items: ["architecture", "guides/mcp-user", "rules", "tools", "examples"],
    },
    {
      type: "category",
      label: "Use",
      collapsible: false,
      items: ["guides/serve", "guides/recipes", "troubleshooting"],
    },
    {
      type: "category",
      label: "Security",
      collapsible: false,
      items: ["security", "secrets-without-seeing"],
    },
    {
      type: "category",
      label: "Reference",
      collapsible: false,
      items: ["tools-reference"],
    },
    {
      type: "category",
      label: "Project",
      collapsible: false,
      items: ["roadmap", "contributing/development"],
    },
  ],
};
