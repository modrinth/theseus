<script setup>
import { computed, readonly, ref } from 'vue'
import {
  Avatar,
  Button,
  CalendarIcon,
  Card,
  Categories,
  Checkbox,
  ClearIcon,
  ClientIcon,
  DownloadIcon,
  DropdownSelect,
  EnvironmentIndicator,
  formatCategory,
  formatCategoryHeader,
  formatNumber,
  HeartIcon,
  NavRow,
  Pagination,
  Promotion,
  SearchFilter,
  SearchIcon,
  ServerIcon,
  StarIcon,
  XIcon,
} from 'omorphia'
import Multiselect from 'vue-multiselect'
import { handleError } from '@/store/state'
import { get_categories, get_game_versions, get_loaders } from '@/helpers/tags'
import SplashScreen from '@/components/ui/SplashScreen.vue'

const loading = ref(false)
const query = ref('')
const facets = ref([])
const orFacets = ref([])
const selectedVersions = ref([])
const onlyOpenSource = ref(false)
const showSnapshots = ref(false)
const selectedEnvironments = ref([])
const sortTypes = readonly([
  { display: 'Relevance', name: 'relevance' },
  { display: 'Download count', name: 'downloads' },
  { display: 'Follow count', name: 'follows' },
  { display: 'Recently published', name: 'newest' },
  { display: 'Recently updated', name: 'updated' },
])
const sortType = ref(sortTypes[0])
const maxResults = ref(20)
const currentPage = ref(1)
const projectType = ref('modpack')

const searchWrapper = ref(null)

const sortedCategories = computed(() => {
  const values = new Map()
  for (const category of categories.value.filter((cat) => cat.project_type === 'mod')) {
    if (!values.has(category.header)) {
      values.set(category.header, [])
    }
    values.get(category.header).push(category)
  }
  return values
})

const [categories, loaders, availableGameVersions] = await Promise.all([
  get_categories().catch(handleError).then(ref),
  get_loaders().catch(handleError).then(ref),
  get_game_versions().catch(handleError).then(ref),
])

const pageCount = ref(1)

const selectableProjectTypes = computed(() => {
  return [
    { label: 'Shaders', href: `` },
    { label: 'Resource Packs', href: `` },
    { label: 'Data Packs', href: `` },
    { label: 'Mods', href: '' },
    { label: 'Modpacks', href: '' },
  ]
})

defineProps({
  showSearch: {
    type: Boolean,
    default: false,
  },
})
</script>

<template>
  <div class="search-container">
    <aside class="filter-panel">
      <Card class="search-panel-card" :class="{ highlighted: showSearch }">
        <Button role="button" disabled> <ClearIcon /> Clear Filters </Button>
        <div class="loaders">
          <h2>Loaders</h2>
          <div
            v-for="loader in loaders.filter(
              (l) =>
                (projectType !== 'mod' && l.supported_project_types?.includes(projectType)) ||
                (projectType === 'mod' && ['fabric', 'forge', 'quilt'].includes(l.name))
            )"
            :key="loader"
          >
            <SearchFilter
              :active-filters="orFacets"
              :icon="loader.icon"
              :display-name="formatCategory(loader.name)"
              :facet-name="`categories:${encodeURIComponent(loader.name)}`"
              class="filter-checkbox"
            />
          </div>
        </div>
        <div class="versions">
          <h2>Minecraft versions</h2>
          <Checkbox v-model="showSnapshots" class="filter-checkbox" label="Include snapshots" />
          <multiselect
            v-model="selectedVersions"
            :options="
              showSnapshots
                ? availableGameVersions.map((x) => x.version)
                : availableGameVersions
                    .filter((it) => it.version_type === 'release')
                    .map((x) => x.version)
            "
            :multiple="true"
            :searchable="true"
            :show-no-results="false"
            :close-on-select="false"
            :clear-search-on-select="false"
            :show-labels="false"
            placeholder="Choose versions..."
          />
        </div>
        <div
          v-for="categoryList in Array.from(sortedCategories)"
          :key="categoryList[0]"
          class="categories"
        >
          <h2>{{ formatCategoryHeader(categoryList[0]) }}</h2>
          <div v-for="category in categoryList[1]" :key="category.name">
            <SearchFilter
              :active-filters="facets"
              :icon="category.icon"
              :display-name="formatCategory(category.name)"
              :facet-name="`categories:${encodeURIComponent(category.name)}`"
              class="filter-checkbox"
            />
          </div>
        </div>
        <div v-if="projectType !== 'datapack'" class="environment">
          <h2>Environments</h2>
          <SearchFilter
            :active-filters="selectedEnvironments"
            display-name="Client"
            facet-name="client"
            class="filter-checkbox"
          >
            <ClientIcon aria-hidden="true" />
          </SearchFilter>
          <SearchFilter
            :active-filters="selectedEnvironments"
            display-name="Server"
            facet-name="server"
            class="filter-checkbox"
          >
            <ServerIcon aria-hidden="true" />
          </SearchFilter>
        </div>
        <div class="open-source">
          <h2>Open source</h2>
          <Checkbox v-model="onlyOpenSource" label="Open source only" class="filter-checkbox" />
        </div>
      </Card>
    </aside>
    <div ref="searchWrapper" class="search">
      <Promotion class="promotion" :external="false" query-param="?r=launcher" />
      <Card class="project-type-container">
        <NavRow :links="selectableProjectTypes" />
      </Card>
      <Card class="search-panel-container" :class="{ highlighted: showSearch }">
        <div class="iconified-input">
          <SearchIcon aria-hidden="true" />
          <input
            v-model="query"
            autocomplete="off"
            type="text"
            :placeholder="`Search ${projectType}s...`"
          />
          <Button class="r-btn" @click="() => (query = '')">
            <XIcon />
          </Button>
        </div>
        <div class="inline-option">
          <span>Sort by</span>
          <DropdownSelect
            v-model="sortType"
            name="Sort by"
            :options="sortTypes"
            :display-name="(option) => option?.display"
          />
        </div>
        <div class="inline-option">
          <span>Show per page</span>
          <DropdownSelect
            v-model="maxResults"
            name="Max results"
            :options="[5, 10, 15, 20, 50, 100]"
            :default-value="maxResults"
            :model-value="maxResults"
            class="limit-dropdown"
          />
        </div>
      </Card>
      <Pagination :page="currentPage" :count="pageCount" class="pagination-before" />
      <SplashScreen v-if="loading" />
      <section v-else class="project-list display-mode--list instance-results" role="list">
        <Card v-for="project in 20" :key="project" class="search-card button-base">
          <div class="icon">
            <Avatar
              src="https://launcher-files.modrinth.com/assets/default_profile.png"
              size="md"
              class="search-icon"
            />
          </div>
          <div class="content-wrapper">
            <div class="title joined-text">
              <h2>Example Modpack</h2>
              <span>by Modrinth</span>
            </div>
            <div class="description">
              A very cool project that does cool project things that you can your friends can do.
            </div>
            <div class="tags">
              <Categories
                :categories="
                  categories
                    .filter((cat) => cat.project_type === projectType)
                    .slice(project / 2, project / 2 + 3)
                "
                :type="modpack"
              >
                <EnvironmentIndicator
                  :type-only="true"
                  :client-side="true"
                  :server-side="true"
                  type="modpack"
                  :search="true"
                />
              </Categories>
            </div>
          </div>
          <div class="stats button-group">
            <div v-if="featured" class="badge">
              <StarIcon />
              Featured
            </div>
            <div class="badge">
              <DownloadIcon />
              {{ formatNumber(420) }}
            </div>
            <div class="badge">
              <HeartIcon />
              {{ formatNumber(69) }}
            </div>
            <div class="badge">
              <CalendarIcon />
              A minute ago
            </div>
          </div>
        </Card>
      </section>
      <pagination :page="currentPage" :count="pageCount" class="pagination-after" />
    </div>
  </div>
</template>

<style src="vue-multiselect/dist/vue-multiselect.css"></style>
<style lang="scss">
.small-instance {
  min-height: unset !important;

  .instance {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.5rem;

    .title {
      font-weight: 600;
      color: var(--color-contrast);
    }
  }

  .small-instance_info {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    justify-content: space-between;
    padding: 0.25rem 0;
  }
}

.filter-checkbox {
  margin-bottom: 0.3rem;
  font-size: 1rem;

  svg {
    display: flex;
    align-self: center;
    justify-self: center;
  }

  button.checkbox {
    border: none;
  }
}
</style>
<style lang="scss" scoped>
.project-type-dropdown {
  width: 100% !important;
}

.promotion {
  margin-top: 1rem;
}

.project-type-container {
  display: flex;
  flex-direction: column;
  width: 100%;
}

.search-panel-card {
  display: flex;
  flex-direction: column;
  margin-bottom: 0 !important;
  min-height: min-content !important;
}

.iconified-input {
  input {
    max-width: none !important;
    flex-basis: auto;
  }
}

.search-panel-container {
  display: inline-flex;
  flex-direction: row;
  align-items: center;
  flex-wrap: wrap;
  width: 100%;
  padding: 1rem !important;
  white-space: nowrap;
  gap: 1rem;

  .inline-option {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.5rem;

    .sort-dropdown {
      max-width: 12.25rem;
    }

    .limit-dropdown {
      width: 5rem;
    }
  }

  .iconified-input {
    flex-grow: 1;
  }

  .filter-panel {
    button {
      display: flex;
      align-items: center;
      justify-content: space-evenly;

      svg {
        margin-right: 0.4rem;
      }
    }
  }
}

.search-container {
  display: flex;

  .filter-panel {
    position: fixed;
    width: 20rem;
    padding: 1rem 0.5rem 1rem 1rem;
    display: flex;
    flex-direction: column;
    height: fit-content;
    min-height: calc(100vh - 3.25rem);
    max-height: calc(100vh - 3.25rem);
    overflow-y: auto;
    -ms-overflow-style: none;
    scrollbar-width: none;

    &::-webkit-scrollbar {
      width: 0;
      background: transparent;
    }

    h2 {
      color: var(--color-contrast);
      margin-top: 1rem;
      margin-bottom: 0.5rem;
      font-size: 1.16rem;
    }
  }

  .search {
    scroll-behavior: smooth;
    margin: 0 1rem 0.5rem 20.5rem;
    width: calc(100% - 20.5rem);

    .loading {
      margin: 2rem;
      text-align: center;
    }
  }
}

.search-card {
  margin-bottom: 0;
  display: grid;
  grid-template-columns: 6rem auto 7rem;
  gap: 0.75rem;
  padding: 1rem;

  &:active:not(&:disabled) {
    scale: 0.98 !important;
  }
}

.joined-text {
  display: inline-flex;
  flex-wrap: wrap;
  flex-direction: row;
  column-gap: 0.5rem;
  align-items: baseline;
  overflow: hidden;
  text-overflow: ellipsis;

  h2 {
    margin-bottom: 0 !important;
    word-wrap: break-word;
    overflow-wrap: anywhere;
  }
}

.button-group {
  display: inline-flex;
  flex-direction: row;
  gap: 0.5rem;
  align-items: flex-start;
  flex-wrap: wrap;
  justify-content: flex-start;
}

.icon {
  grid-column: 1;
  grid-row: 1;
  align-self: center;
  height: 6rem;
}

.content-wrapper {
  display: flex;
  justify-content: space-between;
  grid-column: 2 / 4;
  flex-direction: column;
  grid-row: 1;
  gap: 0.5rem;

  .description {
    word-wrap: break-word;
    overflow-wrap: anywhere;
  }
}

.stats {
  grid-column: 1 / 3;
  grid-row: 2;
  justify-self: stretch;
  align-self: start;
}
</style>
