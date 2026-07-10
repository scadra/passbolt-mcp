// The documentation site: the pages of ../docs, the sidebar from their folders and front matter.
// `npm ci && npm run build` writes it to build/; `npm start` serves it while editing.
const { themes } = require("prism-react-renderer");

const repository = process.env.GITHUB_REPOSITORY ?? "scadra/passbolt-mcp";
const [owner, name] = repository.split("/");

/** @type {import('@docusaurus/types').Config} */
module.exports = {
  title: "passbolt-mcp",
  favicon: "img/favicon.ico",
  tagline: "An MCP server for Passbolt: an AI agent works through a Passbolt user of its own",
  url: `https://${owner}.github.io`,
  baseUrl: `/${name}/`,
  organizationName: owner,
  projectName: name,
  trailingSlash: false,
  onBrokenLinks: "throw",
  onBrokenAnchors: "throw",
  markdown: {
    // Plain Markdown: `<UUID>` and `{…}` in the pages are text, not JSX.
    format: "detect",
    // ```mermaid blocks are diagrams, as on GitHub.
    mermaid: true,
    hooks: { onBrokenMarkdownLinks: "throw" },
  },
  i18n: { defaultLocale: "en", locales: ["en"] },
  themes: ["@docusaurus/theme-mermaid"],
  presets: [
    [
      "classic",
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          path: "../docs",
          routeBasePath: "/",
          sidebarPath: require.resolve("./sidebars.js"),
          editUrl: `https://github.com/${repository}/edit/main/`,
        },
        blog: false,
        pages: false,
        theme: { customCss: require.resolve("./src/css/custom.css") },
      }),
    ],
  ],
  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      colorMode: { defaultMode: "dark", respectPrefersColorScheme: true },
      mermaid: { theme: { light: "neutral", dark: "dark" } },
      navbar: {
        title: "MCP server",
        logo: { alt: "Passbolt", src: "img/logo.svg", width: 120 },
        items: [
          { to: "/getting-started/installation", label: "Install", position: "left" },
          { to: "/rules", label: "Rules", position: "left" },
          { to: "/tools", label: "Tools", position: "left" },
          { to: "/security", label: "Security", position: "left" },
          { href: "https://www.passbolt.com/docs", label: "Passbolt docs", position: "right" },
          { href: `https://github.com/${repository}`, label: "GitHub", position: "right" },
        ],
      },
      footer: {
        style: "dark",
        logo: { alt: "Passbolt", src: "img/logo.svg", href: "https://www.passbolt.com", width: 120 },
        links: [
          {
            title: "Docs",
            items: [
              { label: "Install", to: "/getting-started/installation" },
              { label: "Rules", to: "/rules" },
              { label: "Tools", to: "/tools" },
              { label: "Security", to: "/security" },
            ],
          },
          {
            title: "Community",
            items: [
              { label: "Forum", href: "https://community.passbolt.com" },
              { label: "Issues", href: `https://github.com/${repository}/issues` },
              { label: "Rust SDK", href: "https://crates.io/crates/switch-case-passbolt-sdk" },
            ],
          },
          {
            title: "Passbolt",
            items: [
              { label: "passbolt.com", href: "https://www.passbolt.com" },
              { label: "Passbolt docs", href: "https://www.passbolt.com/docs" },
            ],
          },
        ],
        copyright: "AGPL-3.0-or-later. Passbolt is a registered trademark of Passbolt SA.",
      },
      prism: {
        theme: themes.github,
        darkTheme: themes.dracula,
        additionalLanguages: ["bash", "toml", "json", "rust", "docker"],
      },
    }),
};
