export default defineAppConfig({
  docus: {
    title: 'CapyDeploy',
    description: 'Deploy games to your handheld devices. Fast, easy, chill.',
    image: '/logo.jpg',
    socials: {
      github: 'lobinuxsoft/capydeploy'
    },
    aside: {
      level: 1,
      collapsed: false,
      exclude: []
    },
    header: {
      logo: true,
      showLinkIcon: true,
      exclude: [],
      fluid: true
    },
    footer: {
      credits: {
        text: 'Made with the chill energy of a capybara 🦫',
        href: 'https://github.com/lobinuxsoft/capydeploy'
      },
      textLinks: [],
      iconLinks: [
        {
          href: 'https://github.com/lobinuxsoft/capydeploy',
          icon: 'simple-icons:github'
        }
      ]
    },
    github: {
      dir: 'docs/content',
      branch: 'main',
      repo: 'capydeploy',
      owner: 'lobinuxsoft',
      edit: true
    }
  }
})
