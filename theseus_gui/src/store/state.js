import { createStore } from 'vuex'

export default createStore({
  state() {
    return {
      darkTheme: true,
      instances: [],
      news: [],
    }
  },
  mutations: {
    toggleTheme(state) {
      state.darkTheme = !state.darkTheme
    },
    fetchInstances(state) {
      // Fetch from backend.
      const instances = [
        {
          id: 1,
          name: 'Fabulously Optimized',
          version: '1.18.1',
          downloads: 10,
        },
        {
          id: 2,
          name: 'New Caves',
          version: '1.18 ',
          downloads: 8,
        },
        {
          id: 3,
          name: 'All the Mods 6',
          version: '1.16.5',
          downloads: 4,
        },
        {
          id: 4,
          name: 'Bees',
          version: '1.15.2',
          downloads: 9,
        },
        {
          id: 5,
          name: 'SkyFactory 4',
          version: '1.12.2',
          downloads: 1000,
        },
        {
          id: 6,
          name: 'RLCraft',
          version: '1.12.2',
          downloads: 10000,
        },
        {
          id: 7,
          name: 'Regrowth',
          version: '1.7.10',
          downloads: 1000,
        },
      ]

      state.instances = [...instances]
    },
    fetchNews(state) {
      // Fetch from backend.
      const news = [
        {
          id: 1,
          headline: 'Caves & Cliffs Update: Part II Dev Q&A',
          blurb: 'Your questions, answered!',
          source: 'From Minecraft.Net',
        },
        {
          id: 2,
          headline: 'Project of the WeeK: Gobblygook',
          blurb: 'Your questions, answered!',
          source: 'Modrinth Blog',
        },
      ]

      state.news = [...news]
    },
  },
})
