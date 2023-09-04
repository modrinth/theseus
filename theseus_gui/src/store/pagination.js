import { defineStore } from 'pinia'

export const usePagination = defineStore('paginationStore', {
  state: () => ({
    maxResults: 20,
    paginationState: 20,
  }),
  actions: {
    setMaxResults(newMaxResults) {
        this.maxResults = newMaxResults;
    },
    setPagination(newPaginationState) {
        this.paginationState = newPaginationState;
    },
  },
})