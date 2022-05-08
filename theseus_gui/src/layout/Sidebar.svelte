<script lang="ts">
    import VerticalNav from '../components/VerticalNav.svelte'
    import IconHome from 'virtual:icons/lucide/home'
    import IconSearch from 'virtual:icons/heroicons-outline/search'
    import IconPlus from 'virtual:icons/heroicons-outline/plus'
    import IconLibrary from 'virtual:icons/lucide/library';
    import IconSettings from 'virtual:icons/lucide/settings'
    import { page } from "$app/stores";
    import { Button } from 'omorphia'
</script>

<div class="sidebar">
    <div class="account">
        <div class="account__heads">
            <img src="https://mc-heads.net/avatar/venashial" alt="Minecraft head"/>
        </div>

        <a class="account__info" href="/settings/accounts">
            <div>venashial</div>
            <div class="account__info__manage">Manage accounts</div>
        </a>
    </div>

    <VerticalNav items={[
        {
            label: 'Home',
            href: '',
            icon: IconHome,
        },
        {
            label: 'Search',
            href: 'search',
            icon: IconSearch,
        },
        {
            label: 'Library',
            href: 'library',
            icon: IconLibrary,
        }
    ]}/>

    <div class="instance-list">
        <div class="instance-list__title">Instances</div>
        <div class="instance-list__container">
            <a class="instance-list__container__item">Fabulously Optimized</a>
            <a class="instance-list__container__item">Fabulously Optimized</a>
            <a class="instance-list__container__item">Fabulously Optimized</a>
            <a class="instance-list__container__item">Fabulously Optimized</a>
            <a class="instance-list__container__item">Fabulously Optimized</a>
            <a class="instance-list__container__item">Fabulously Optimized</a>
            <a class="instance-list__container__item">Fabulously Optimized</a>
            <a class="instance-list__container__item">Fabulously Optimized</a>
            <a class="instance-list__container__item">Fabulously Optimized</a>
            <a class="instance-list__container__item">Fabulously Optimized</a>
            <a class="instance-list__container__item">Fabulously Optimized</a>
            <a class="instance-list__container__item">Fabulously Optimized</a>
        </div>
        <div class="instance-list__create">
            <Button color="primary"><IconPlus /></Button> Create instance
        </div>
    </div>

    <a class="nav-item" href="/settings" class:active={$page.url.pathname.startsWith('/settings')}>
        <IconSettings />
        Settings
    </a>
</div>

<style lang="postcss">
    .sidebar {
        display: flex;
        flex-direction: column;
        padding: 1rem;
        height: 100%;
        background-color: var(--sidebar-bg);

        .account {
            display: flex;
            grid-gap: 0.75rem;

            &__heads {
                img {
                    width: 2.5rem;
                    border-radius: var(--rounded-sm);
                }
            }

            &__info {
                &__manage {
                    color: var(--color-text-light);
                }
            }
        }

        .instance-list {
            padding: 0 8px;
            display: flex;
            flex-direction: column;
            color: var(--color-text-light);

            &__title {
                text-transform: uppercase;
                letter-spacing: 1px;
                font-size: 11px;
                line-height: 100%;
            }

            &__container {
                max-height: calc(100vh - 400px);
                overflow-y: auto;
                mask-image: linear-gradient(to bottom, transparent, hsla(0, 0%, 0%, 1) 5% 95%, transparent);
                scrollbar-width: none;
                padding: 8px 0;
                display: flex;
                flex-direction: column;
                gap: 4px;

                &::-webkit-scrollbar {
                    display: none;
                }
            }

            &__create {
                margin-top: 16px;
                display: flex;
                align-items: center;
                grid-gap: 8px;

                :global(button) {
                    width: 32px;
                }
            }
        }

        > :global(*) {
            margin-bottom: 24px;
        }

        > *:last-child {
            margin-top: auto;
            margin-bottom: 0;
        }
    }
</style>
