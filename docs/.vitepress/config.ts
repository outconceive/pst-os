import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'PST OS',
  description: 'Parallel String Theory OS — one primitive, one solver, every surface',
  base: '/pst-os/',
  ignoreDeadLinks: true,

  themeConfig: {
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Markout', link: '/api/markout' },
      { text: 'Architecture', link: '/architecture/overview' },
      { text: 'Paper', link: '/architecture/paper' },
    ],

    sidebar: {
      '/guide/': [
        {
          text: 'Getting Started',
          items: [
            { text: 'Introduction', link: '/guide/getting-started' },
            { text: 'Running PST OS', link: '/guide/running' },
            { text: 'Your First Markout', link: '/guide/first-markout' },
          ]
        },
        {
          text: 'Markout Language',
          items: [
            { text: 'Components', link: '/guide/components' },
            { text: 'Containers', link: '/guide/containers' },
            { text: 'Styles', link: '/guide/styles' },
            { text: 'Grid Layout', link: '/guide/grid' },
            { text: 'Parametric Layout', link: '/guide/parametric' },
            { text: 'State & Reactivity', link: '/guide/state' },
            { text: 'Lists (@each)', link: '/guide/lists' },
            { text: 'Editor (@editor)', link: '/guide/editor' },
            { text: 'Validation', link: '/guide/validation' },
          ]
        },
        {
          text: 'Desktop',
          items: [
            { text: 'Windows & Focus', link: '/guide/desktop' },
            { text: 'Browser (dt:// & gh://)', link: '/guide/browser' },
            { text: 'Configuration (/pst/)', link: '/guide/config' },
            { text: 'Persistence', link: '/guide/persistence' },
          ]
        },
      ],
      '/api/': [
        {
          text: 'Reference',
          items: [
            { text: 'Markout Syntax', link: '/api/markout' },
            { text: 'Component Types', link: '/api/components' },
            { text: 'Style Reference', link: '/api/styles' },
            { text: 'Constraint Reference', link: '/api/constraints' },
            { text: 'Validation Rules', link: '/api/validation' },
            { text: 'Event Properties', link: '/api/events' },
          ]
        }
      ],
      '/architecture/': [
        {
          text: 'Architecture',
          items: [
            { text: 'Overview', link: '/architecture/overview' },
            { text: 'Parallel Strings', link: '/architecture/parallel-strings' },
            { text: 'Rendering Pipeline', link: '/architecture/rendering' },
            { text: 'Interaction Model', link: '/architecture/interaction' },
            { text: 'seL4 Integration', link: '/architecture/sel4' },
            { text: 'Zenodo Paper', link: '/architecture/paper' },
          ]
        }
      ]
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/outconceive/pst-os' }
    ],

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright 2025-present PST OS'
    },

    search: {
      provider: 'local'
    }
  }
})
