<script lang="ts">
    import { Checkbox, FormField, TextInput, Button } from "omorphia"
    import GlobalSettings from "$components/GlobalSettings.svelte"
    import TitledSection from "$components/TitledSection.svelte"

    export let overrideGlobalSettings = false
</script>

<div class="section">
    <TitledSection title="Override global settings" toggleable={true}>
        <GlobalSettings />
    </TitledSection>
</div>

<style lang="postcss">
    .section {
        display: flex;
        flex-direction: column;
        padding: 1rem;
        grid-gap: 1rem;

        &__title {
            display: flex;
            grid-gap: 1rem;
            align-items: center;

            &::after {
                flex: 1 1;
                content: " ";
                background-color: hsla(0, 0%, 100%, 0.2);
                height: 0.2rem;
                border-radius: var(--rounded-max);
            }
        }

        &__items {
            display: flex;
            flex-direction: column;
            grid-gap: 1rem;
            padding: 0 8px;
        }
    }
</style>
