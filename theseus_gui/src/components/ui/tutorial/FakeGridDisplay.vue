<script setup>
import { ref } from 'vue'
import { Card, DropdownSelect, SearchIcon, XIcon, Button, Avatar } from 'omorphia'

const search = ref('')
const group = ref('Category')
const filters = ref('All profiles')
const sortBy = ref('Name')

defineProps({
  showFilters: {
    type: Boolean,
    default: false,
  },
  showInstances: {
    type: Boolean,
    default: false,
  },
})
</script>
<template>
  <Card class="header" :class="{ highlighted: showFilters }">
    <div class="iconified-input">
      <SearchIcon />
      <input v-model="search" type="text" placeholder="Search" class="search-input" />
      <Button class="r-btn" @click="() => (search = '')">
        <XIcon />
      </Button>
    </div>
    <div class="labeled_button">
      <span>Sort by</span>
      <DropdownSelect
        v-model="sortBy"
        class="sort-dropdown"
        name="Sort Dropdown"
        :options="['Name', 'Last played', 'Date created', 'Date modified', 'Game version']"
        placeholder="Select..."
      />
    </div>
    <div class="labeled_button">
      <span>Filter by</span>
      <DropdownSelect
        v-model="filters"
        class="filter-dropdown"
        name="Filter Dropdown"
        :options="['All profiles', 'Custom instances', 'Downloaded modpacks']"
        placeholder="Select..."
      />
    </div>
    <div class="labeled_button">
      <span>Group by</span>
      <DropdownSelect
        v-model="group"
        class="group-dropdown"
        name="Group dropdown"
        :options="['Category', 'Loader', 'Game version', 'None']"
        placeholder="Select..."
      />
    </div>
  </Card>
  <div class="row">
    <section class="instances">
      <Card
        v-for="project in 20"
        :key="project"
        class="instance-card-item button-base"
        :class="{ highlighted: project === 1 && showInstance }"
      >
        <Avatar
          size="sm"
          src="https://launcher-files.modrinth.com/assets/default_profile.png"
          alt="Mod card"
          class="mod-image"
        />
        <div class="project-info">
          <p class="title">Example Profile</p>
          <p class="description">Forge/Fabric 1.20.1</p>
        </div>
      </Card>
      <slot />
    </section>
  </div>
</template>
<style lang="scss" scoped>
.row {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  width: 100%;
  padding: 1rem;

  .divider {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    gap: 1rem;
    margin-bottom: 1rem;

    p {
      margin: 0;
      font-size: 1rem;
      white-space: nowrap;
      color: var(--color-contrast);
    }

    hr {
      background-color: var(--color-gray);
      height: 1px;
      width: 100%;
      border: none;
    }
  }
}

.header {
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 1rem;
  align-items: inherit;
  margin: 1rem 1rem 0 !important;
  padding: 1rem;
  width: calc(100% - 2rem);

  .iconified-input {
    flex-grow: 1;

    input {
      min-width: 100%;
    }
  }

  .sort-dropdown {
    width: 10rem;
  }

  .filter-dropdown {
    width: 15rem;
  }

  .group-dropdown {
    width: 10rem;
  }

  .labeled_button {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.5rem;
    white-space: nowrap;
  }
}

.instances {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));
  width: 100%;
  gap: 1rem;
  margin-right: auto;
  scroll-behavior: smooth;
}

.instance {
  position: relative;
}

.instance-card-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  padding: var(--gap-md);
  transition: 0.1s ease-in-out all !important; /* overrides Omorphia defaults */
  margin-bottom: 0;

  .mod-image {
    --size: 100%;

    width: 100% !important;
    height: auto !important;
    max-width: unset !important;
    max-height: unset !important;
    aspect-ratio: 1 / 1 !important;
  }

  .project-info {
    margin-top: 1rem;
    width: 100%;

    .title {
      color: var(--color-contrast);
      overflow: hidden;
      white-space: nowrap;
      text-overflow: ellipsis;
      width: 100%;
      margin: 0;
      font-weight: 600;
      font-size: 1rem;
      line-height: 110%;
      display: inline-block;
    }

    .description {
      color: var(--color-base);
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
      font-weight: 500;
      font-size: 0.775rem;
      line-height: 125%;
      margin: 0.25rem 0 0;
      text-transform: capitalize;
      white-space: nowrap;
      text-overflow: ellipsis;
    }
  }
}
</style>
