<template>
  <div class="breadcrumbs">
    <div
      v-if="props.afterLogo && breadcrumbContext.routeBreadcrumbs.value?.length > 0"
      class="breadcrumbs__item"
    >
      <ChevronRightIcon class="chevron" />
    </div>
    <div
      v-for="breadcrumb in breadcrumbContext.routeBreadcrumbs.value"
      :key="breadcrumb.name"
      class="breadcrumbs__item"
    >
      <router-link
        v-if="breadcrumb.link"
        :to="{
          path: breadcrumb.link.replace('{id}', encodeURIComponent($route.params.id)),
          query: breadcrumb.query,
        }"
      >
        {{ breadcrumbName(breadcrumb.name) }}
      </router-link>
      <span v-else class="selected">
        {{ breadcrumbName(breadcrumb.name) }}
      </span>
      <ChevronRightIcon v-if="breadcrumb.link" class="chevron" />
    </div>
  </div>
</template>

<script setup>
import { ChevronRightIcon } from 'omorphia'
import { useBreadcrumbs, useBreadcrumbContext } from '@/store/breadcrumbs'
import { useRoute } from 'vue-router'

const props = defineProps({
  afterLogo: {
    type: Boolean,
    default: false,
  },
})

const breadcrumbData = useBreadcrumbs()

const route = useRoute()
const breadcrumbContext = useBreadcrumbContext(route)

breadcrumbData.$subscribe(() => {
  breadcrumbData?.resetToNames(breadcrumbContext.routeBreadcrumbs.value)
})

const breadcrumbName = (bcn) => {
  if (bcn.charAt(0) === '?') {
    return breadcrumbData.getName(bcn.slice(1))
  }
  return bcn
}
</script>

<style scoped lang="scss">
.breadcrumbs {
  display: flex;
  flex-direction: row;

  .breadcrumbs__item {
    display: flex;
    flex-direction: row;
    vertical-align: center;
    margin: auto 0;

    .chevron {
      margin: auto 0.5rem;
    }

    a {
      margin: auto 0;
    }
  }

  .breadcrumbs__back,
  .breadcrumbs__forward {
    margin: auto 0;
    color: var(--color-base);
    height: unset;
    width: unset;
  }

  .breadcrumbs__forward {
    margin-right: 1rem;
  }
}

.selected {
  color: var(--color-contrast);
}
</style>
